//! Relay-mode client: signed HTTP, rendezvous pairing, send/drain (BATON.md §10).
use crate::device::{id_for, Device, Peer};
use anyhow::{anyhow, bail, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use baton_core::{
    crypto::{self, Header},
    noise::{Established, Handshake},
    pb,
};
use ed25519_dalek::Signer;
use futures_util::{SinkExt, StreamExt};
use prost::Message;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_tungstenite::tungstenite::Message as Ws;

pub struct Relay<'a> {
    pub base: String,
    pub dev: &'a Device,
    http: reqwest::Client,
}

fn now_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

impl<'a> Relay<'a> {
    pub fn new(base: &str, dev: &'a Device) -> Self {
        Self {
            base: base.trim_end_matches('/').into(),
            dev,
            http: reqwest::Client::new(),
        }
    }

    async fn call(&self, method: &str, path: &str, body: Option<Value>) -> Result<(u16, Value)> {
        let raw = body.map(|b| b.to_string().into_bytes()).unwrap_or_default();
        let ts = now_s();
        let canon = format!(
            "{method}\n{path}\n{ts}\n{}",
            hex::encode(Sha256::digest(&raw))
        );
        let sig = self.dev.signing.sign(canon.as_bytes());
        let auth = format!(
            "Baton-Sig id={},ts={ts},sig={}",
            hex::encode(self.dev.id()),
            B64.encode(sig.to_bytes())
        );
        let m = reqwest::Method::from_bytes(method.as_bytes())?;
        let mut req = self
            .http
            .request(m, format!("{}{path}", self.base))
            .header("Authorization", auth);
        if !raw.is_empty() {
            req = req.body(raw);
        }
        let res = req.send().await?;
        let status = res.status().as_u16();
        Ok((status, res.json().await.unwrap_or(Value::Null)))
    }

    pub async fn register(&self) -> Result<()> {
        let (s, v) = self
            .call(
                "POST",
                "/v1/devices",
                Some(json!({
                    "ed25519_pub": B64.encode(self.dev.ed25519_pub()),
                    "x25519_pub": B64.encode(self.dev.x25519.public_bytes()),
                    "platform": "cli",
                })),
            )
            .await?;
        if s != 201 {
            bail!("register failed: {s} {v}");
        }
        Ok(())
    }

    /// Authorize `peer` to send to us.
    pub async fn grant(&self, peer: &Peer) -> Result<()> {
        let (s, v) = self
            .call(
                "POST",
                "/v1/peers",
                Some(json!({"peer_id": hex::encode(peer.id)})),
            )
            .await?;
        if s != 201 {
            bail!("grant failed: {s} {v}");
        }
        Ok(())
    }

    pub async fn send(
        &self,
        peer: &Peer,
        payload: &pb::Payload,
        push: pb::PushHint,
    ) -> Result<Vec<u8>> {
        let env = crypto::seal(
            payload,
            &Header {
                sender: self.dev.id(),
                recipient: peer.id,
                sent_at_ms: now_s() * 1000,
                ttl_s: 3600,
                push,
                collapse_key: [0; 16],
            },
            &self.dev.x25519,
            &peer.x25519_pub,
        )?;
        let (s, v) = self
            .call(
                "POST",
                "/v1/messages",
                Some(json!({"envelopes": [B64.encode(env.encode_to_vec())]})),
            )
            .await?;
        if s != 200 || !v["rejected"].as_array().is_some_and(Vec::is_empty) {
            bail!("send failed: {s} {v}");
        }
        Ok(env.msg_id)
    }

    /// Drain the mailbox, decrypt with the sender's known key, and ack each message.
    pub async fn receive(&self, peers: &[Peer]) -> Result<Vec<pb::Payload>> {
        let (_, v) = self.call("GET", "/v1/messages", None).await?;
        let mut out = vec![];
        for e in v["envelopes"]
            .as_array()
            .ok_or_else(|| anyhow!("no envelopes"))?
        {
            let env = pb::Envelope::decode(B64.decode(e.as_str().unwrap_or_default())?.as_slice())?;
            let peer = peers
                .iter()
                .find(|p| p.id.as_slice() == env.sender.as_slice())
                .ok_or_else(|| anyhow!("unknown sender"))?;
            out.push(crypto::open(&env, &self.dev.x25519, &peer.x25519_pub)?);
            self.call(
                "DELETE",
                &format!("/v1/messages/{}", hex::encode(&env.msg_id)),
                None,
            )
            .await?;
        }
        Ok(out)
    }
}

/// Run the Noise IKpsk2 pairing handshake through the rendezvous DO. Frames are base64 text.
/// Returns the established session (for the SAS) and the peer record.
pub async fn pair_via_rendezvous(
    base: &str,
    rid: &str,
    me: &Device,
    initiator: bool,
    desktop_pub: &[u8],
    psk: &[u8; 32],
) -> Result<(Established, Peer)> {
    let url = format!(
        "{}/v1/rendezvous/{rid}",
        base.trim_end_matches('/').replacen("http", "ws", 1)
    );
    let (mut ws, _) = tokio_tungstenite::connect_async(url).await?;
    let sk = me.x25519.secret_bytes();
    let ed = me.ed25519_pub().to_vec();
    let (est, remote_ed) = if initiator {
        let mut hs = Handshake::pairing_initiator(&sk, desktop_pub, psk)?;
        ws.send(Ws::text(B64.encode(hs.write(&ed)?))).await?;
        let msg = next_frame(&mut ws).await?;
        let remote = hs.read(&msg)?;
        (hs.finish()?, remote)
    } else {
        let mut hs = Handshake::pairing_responder(&sk, psk)?;
        let msg = next_frame(&mut ws).await?;
        let remote = hs.read(&msg)?;
        ws.send(Ws::text(B64.encode(hs.write(&ed)?))).await?;
        (hs.finish()?, remote)
    };
    let ed25519_pub: [u8; 32] = remote_ed.try_into().map_err(|_| anyhow!("bad peer key"))?;
    let peer = Peer {
        id: id_for(&ed25519_pub),
        ed25519_pub,
        x25519_pub: est.remote_static.clone(),
    };
    Ok((est, peer))
}

async fn next_frame<S>(ws: &mut S) -> Result<Vec<u8>>
where
    S: StreamExt<Item = Result<Ws, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    while let Some(m) = ws.next().await {
        if let Ws::Text(t) = m? {
            return Ok(B64.decode(t.as_bytes())?);
        }
    }
    bail!("rendezvous closed")
}
