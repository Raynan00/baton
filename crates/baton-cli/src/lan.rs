//! LAN link: TCP + `Noise_KK`, length-prefixed frames carrying HPKE-sealed envelopes (BATON.md §3.2).
use crate::device::{Device, Peer};
use anyhow::{anyhow, bail, Result};
use baton_core::noise::{Established, Handshake};
use std::{future::Future, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};

/// Connect + ack budget before the router falls back to the relay.
pub const LAN_BUDGET: Duration = Duration::from_millis(1500);
const MAX_FRAME: usize = 65_535;

async fn write_frame(s: &mut TcpStream, b: &[u8]) -> Result<()> {
    if b.len() > MAX_FRAME {
        bail!("frame too large");
    }
    s.write_all(&(b.len() as u16).to_be_bytes()).await?;
    s.write_all(b).await?;
    Ok(())
}

async fn read_frame(s: &mut TcpStream) -> Result<Vec<u8>> {
    let mut len = [0u8; 2];
    s.read_exact(&mut len).await?;
    let mut buf = vec![0u8; u16::from_be_bytes(len) as usize];
    s.read_exact(&mut buf).await?;
    Ok(buf)
}

pub struct LanLink {
    stream: TcpStream,
    noise: Established,
}

impl LanLink {
    /// Initiator side. Fails if the peer does not hold our static key.
    pub async fn connect(addr: &str, me: &Device, peer: &Peer) -> Result<Self> {
        timeout(LAN_BUDGET, async {
            let mut stream = TcpStream::connect(addr).await?;
            let mut hs = Handshake::lan(&me.x25519.secret_bytes(), &peer.x25519_pub, true)?;
            write_frame(&mut stream, &hs.write(&[])?).await?;
            hs.read(&read_frame(&mut stream).await?)?;
            Ok::<_, anyhow::Error>(Self {
                stream,
                noise: hs.finish()?,
            })
        })
        .await
        .map_err(|_| anyhow!("lan connect timed out"))?
    }

    /// Send one envelope and wait for the 1-byte ack, within `LAN_BUDGET`.
    pub async fn send(&mut self, envelope: &[u8]) -> Result<()> {
        timeout(LAN_BUDGET, async {
            let ct = self.noise.encrypt(envelope)?;
            write_frame(&mut self.stream, &ct).await?;
            let ack = self.noise.decrypt(&read_frame(&mut self.stream).await?)?;
            if ack != [1] {
                bail!("bad ack");
            }
            Ok(())
        })
        .await
        .map_err(|_| anyhow!("lan ack timed out"))?
    }

    pub async fn recv(&mut self) -> Result<Vec<u8>> {
        let env = self.noise.decrypt(&read_frame(&mut self.stream).await?)?;
        let ack = self.noise.encrypt(&[1])?;
        write_frame(&mut self.stream, &ack).await?;
        Ok(env)
    }

    pub fn remote_static(&self) -> &[u8] {
        &self.noise.remote_static
    }
}

pub struct LanListener {
    listener: TcpListener,
}

impl LanListener {
    pub async fn bind(addr: &str) -> Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(addr).await?,
        })
    }

    pub fn local_addr(&self) -> Result<String> {
        Ok(self.listener.local_addr()?.to_string())
    }

    /// Accept one connection. The initiator is identified by trying each paired peer's key;
    /// unknown peers are dropped before any payload is read.
    pub async fn accept(&self, me: &Device, peers: &[Peer]) -> Result<(LanLink, Peer)> {
        let (mut stream, _) = self.listener.accept().await?;
        let first = timeout(LAN_BUDGET, read_frame(&mut stream))
            .await
            .map_err(|_| anyhow!("handshake timeout"))??;
        for p in peers {
            let mut hs = Handshake::lan(&me.x25519.secret_bytes(), &p.x25519_pub, false)?;
            if hs.read(&first).is_ok() {
                write_frame(&mut stream, &hs.write(&[])?).await?;
                return Ok((
                    LanLink {
                        stream,
                        noise: hs.finish()?,
                    },
                    p.clone(),
                ));
            }
        }
        bail!("unknown peer")
    }
}

/// Send-once routing: try the LAN within its budget, otherwise use the relay.
/// Returns `true` when the LAN carried it. The caller dedups on `msg_id` if both ever arrive.
pub async fn send_with_fallback<F, Fut>(
    lan: Option<&mut LanLink>,
    envelope: &[u8],
    relay: F,
) -> Result<bool>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<()>>,
{
    if let Some(l) = lan {
        if l.send(envelope).await.is_ok() {
            return Ok(true);
        }
    }
    relay().await?;
    Ok(false)
}
