//! HPKE Auth-mode sealing of payloads (BATON.md §9.4).
use crate::{pb, Error, Result};
use hpke::{
    aead::ChaCha20Poly1305, kdf::HkdfSha256, kem::X25519HkdfSha256, Deserializable, Kem as _,
    OpModeR, OpModeS, Serializable,
};
use prost::Message;
use rand::{rngs::OsRng, RngCore};

type Kem = X25519HkdfSha256;
type PrivKey = <Kem as hpke::Kem>::PrivateKey;
type PubKey = <Kem as hpke::Kem>::PublicKey;

const INFO: &[u8] = b"baton/v1/msg";

/// A device's static X25519 key pair.
pub struct X25519Identity {
    sk: PrivKey,
    pk: PubKey,
}

impl X25519Identity {
    pub fn generate() -> Self {
        let (sk, pk) = Kem::gen_keypair(&mut OsRng);
        Self { sk, pk }
    }

    pub fn from_secret_bytes(b: &[u8]) -> Result<Self> {
        let sk = PrivKey::from_bytes(b).map_err(|e| Error::Crypto(e.to_string()))?;
        let pk = Kem::sk_to_pk(&sk);
        Ok(Self { sk, pk })
    }

    pub fn secret_bytes(&self) -> Vec<u8> {
        self.sk.to_bytes().to_vec()
    }

    pub fn public_bytes(&self) -> Vec<u8> {
        self.pk.to_bytes().to_vec()
    }
}

/// Envelope header fields the sender chooses (everything except the ciphertext parts).
pub struct Header {
    pub sender: [u8; 16],
    pub recipient: [u8; 16],
    pub sent_at_ms: u64,
    pub ttl_s: u32,
    pub push: pb::PushHint,
    pub collapse_key: [u8; 16],
}

/// AAD = envelope fields 1–8 serialized (`hpke_enc`/`ciphertext` left empty).
fn aad(env: &pb::Envelope) -> Vec<u8> {
    pb::Envelope {
        hpke_enc: Vec::new(),
        ciphertext: Vec::new(),
        ..env.clone()
    }
    .encode_to_vec()
}

pub fn seal(
    payload: &pb::Payload,
    header: &Header,
    sender: &X25519Identity,
    recipient_pub: &[u8],
) -> Result<pb::Envelope> {
    let pk_r = PubKey::from_bytes(recipient_pub).map_err(|e| Error::Crypto(e.to_string()))?;
    let mut msg_id = vec![0u8; 16];
    OsRng.fill_bytes(&mut msg_id);
    let mut env = pb::Envelope {
        v: 1,
        msg_id,
        sender: header.sender.to_vec(),
        recipient: header.recipient.to_vec(),
        sent_at_ms: header.sent_at_ms,
        ttl_s: header.ttl_s.min(86_400),
        push: header.push as i32,
        collapse_key: header.collapse_key.to_vec(),
        hpke_enc: Vec::new(),
        ciphertext: Vec::new(),
    };
    let (enc, ct) = hpke::single_shot_seal::<ChaCha20Poly1305, HkdfSha256, Kem, _>(
        &OpModeS::Auth((sender.sk.clone(), sender.pk.clone())),
        &pk_r,
        INFO,
        &payload.encode_to_vec(),
        &aad(&env),
        &mut OsRng,
    )
    .map_err(|e| Error::Crypto(e.to_string()))?;
    env.hpke_enc = enc.to_bytes().to_vec();
    env.ciphertext = ct;
    Ok(env)
}

pub fn open(
    env: &pb::Envelope,
    recipient: &X25519Identity,
    sender_pub: &[u8],
) -> Result<pb::Payload> {
    if env.v != 1 {
        return Err(Error::Envelope("unsupported version"));
    }
    let pk_s = PubKey::from_bytes(sender_pub).map_err(|e| Error::Crypto(e.to_string()))?;
    let enc = <Kem as hpke::Kem>::EncappedKey::from_bytes(&env.hpke_enc)
        .map_err(|e| Error::Crypto(e.to_string()))?;
    let pt = hpke::single_shot_open::<ChaCha20Poly1305, HkdfSha256, Kem>(
        &OpModeR::Auth(pk_s),
        &recipient.sk,
        &enc,
        INFO,
        &env.ciphertext,
        &aad(env),
    )
    .map_err(|e| Error::Crypto(e.to_string()))?;
    Ok(pb::Payload::decode(pt.as_slice())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(a: u8, b: u8) -> Header {
        Header {
            sender: [a; 16],
            recipient: [b; 16],
            sent_at_ms: 1,
            ttl_s: 100_000,
            push: pb::PushHint::Alert,
            collapse_key: [9; 16],
        }
    }

    fn clip(text: &str) -> pb::Payload {
        pb::Payload {
            kind: Some(pb::payload::Kind::Clip(pb::Clip {
                clip_id: "c".into(),
                body: Some(pb::clip::Body::Text(text.into())),
                char_count: text.len() as u32,
                expires_at_ms: 0,
            })),
        }
    }

    #[test]
    fn roundtrip_and_no_plaintext() {
        let (a, b) = (X25519Identity::generate(), X25519Identity::generate());
        let p = clip("https://secret.example/title");
        let env = seal(&p, &header(1, 2), &a, &b.public_bytes()).unwrap();
        assert_eq!(env.ttl_s, 86_400);
        let wire = env.encode_to_vec();
        assert!(!wire.windows(6).any(|w| w == b"secret"));
        assert_eq!(open(&env, &b, &a.public_bytes()).unwrap(), p);
    }

    #[test]
    fn wrong_sender_or_tampered_header_rejected() {
        let (a, b, c) = (
            X25519Identity::generate(),
            X25519Identity::generate(),
            X25519Identity::generate(),
        );
        let env = seal(&clip("x"), &header(1, 2), &a, &b.public_bytes()).unwrap();
        assert!(open(&env, &b, &c.public_bytes()).is_err());
        let mut t = env.clone();
        t.ttl_s += 1;
        assert!(open(&t, &b, &a.public_bytes()).is_err());
    }
}
