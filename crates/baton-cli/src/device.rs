use anyhow::Result;
use baton_core::{crypto::X25519Identity, noise};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

/// A device's two static keys. The X25519 key serves both Noise and HPKE.
pub struct Device {
    pub name: String,
    pub signing: SigningKey,
    pub x25519: X25519Identity,
}

#[derive(Clone, Debug)]
pub struct Peer {
    pub id: [u8; 16],
    pub ed25519_pub: [u8; 32],
    pub x25519_pub: Vec<u8>,
}

impl Device {
    pub fn generate(name: &str) -> Result<Self> {
        let (sk, _) = noise::generate_keypair()?;
        Ok(Self {
            name: name.into(),
            signing: SigningKey::generate(&mut OsRng),
            x25519: X25519Identity::from_secret_bytes(&sk)?,
        })
    }

    pub fn ed25519_pub(&self) -> [u8; 32] {
        self.signing.verifying_key().to_bytes()
    }

    /// First 16 bytes of sha256(ed25519 public key); matches the server.
    pub fn id(&self) -> [u8; 16] {
        id_for(&self.ed25519_pub())
    }
}

pub fn id_for(ed_pub: &[u8]) -> [u8; 16] {
    Sha256::digest(ed_pub)[..16].try_into().unwrap()
}
