//! Noise sessions: `IKpsk2` pairing and `KK` LAN link (BATON.md §9.4).
use crate::{sas::sas_code, Error, Result};
use snow::{Builder, HandshakeState, TransportState};

pub const PAIRING_PATTERN: &str = "Noise_IKpsk2_25519_ChaChaPoly_BLAKE2s";
pub const LAN_PATTERN: &str = "Noise_KK_25519_ChaChaPoly_BLAKE2s";
const MAX_MSG: usize = 65_535;

fn err(e: snow::Error) -> Error {
    Error::Crypto(e.to_string())
}

/// One side of a Noise handshake; call `write`/`read` alternately until `finish`.
pub struct Handshake {
    hs: HandshakeState,
}

pub struct Established {
    pub transport: TransportState,
    pub handshake_hash: Vec<u8>,
    pub remote_static: Vec<u8>,
}

impl Established {
    /// 6-digit code both devices display.
    pub fn sas(&self) -> String {
        sas_code(&self.handshake_hash)
    }
    pub fn encrypt(&mut self, plain: &[u8]) -> Result<Vec<u8>> {
        let mut out = vec![0u8; plain.len() + 16];
        let n = self.transport.write_message(plain, &mut out).map_err(err)?;
        out.truncate(n);
        Ok(out)
    }
    pub fn decrypt(&mut self, ct: &[u8]) -> Result<Vec<u8>> {
        let mut out = vec![0u8; ct.len()];
        let n = self.transport.read_message(ct, &mut out).map_err(err)?;
        out.truncate(n);
        Ok(out)
    }
}

impl Handshake {
    /// Phone side of pairing: knows the desktop static key and QR psk.
    pub fn pairing_initiator(local_sk: &[u8], desktop_pub: &[u8], psk: &[u8; 32]) -> Result<Self> {
        let hs = Builder::new(PAIRING_PATTERN.parse().map_err(err)?)
            .local_private_key(local_sk)
            .remote_public_key(desktop_pub)
            .psk(2, psk)
            .build_initiator()
            .map_err(err)?;
        Ok(Self { hs })
    }

    pub fn pairing_responder(local_sk: &[u8], psk: &[u8; 32]) -> Result<Self> {
        let hs = Builder::new(PAIRING_PATTERN.parse().map_err(err)?)
            .local_private_key(local_sk)
            .psk(2, psk)
            .build_responder()
            .map_err(err)?;
        Ok(Self { hs })
    }

    /// LAN link: both statics known from pairing. Unknown peers fail the handshake.
    pub fn lan(local_sk: &[u8], remote_pub: &[u8], initiator: bool) -> Result<Self> {
        let b = Builder::new(LAN_PATTERN.parse().map_err(err)?)
            .local_private_key(local_sk)
            .remote_public_key(remote_pub);
        let hs = if initiator {
            b.build_initiator()
        } else {
            b.build_responder()
        }
        .map_err(err)?;
        Ok(Self { hs })
    }

    pub fn write(&mut self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut out = vec![0u8; MAX_MSG];
        let n = self.hs.write_message(payload, &mut out).map_err(err)?;
        out.truncate(n);
        Ok(out)
    }

    pub fn read(&mut self, msg: &[u8]) -> Result<Vec<u8>> {
        let mut out = vec![0u8; MAX_MSG];
        let n = self.hs.read_message(msg, &mut out).map_err(err)?;
        out.truncate(n);
        Ok(out)
    }

    pub fn is_finished(&self) -> bool {
        self.hs.is_handshake_finished()
    }

    pub fn finish(self) -> Result<Established> {
        let handshake_hash = self.hs.get_handshake_hash().to_vec();
        let remote_static = self
            .hs
            .get_remote_static()
            .map(<[u8]>::to_vec)
            .unwrap_or_default();
        Ok(Established {
            transport: self.hs.into_transport_mode().map_err(err)?,
            handshake_hash,
            remote_static,
        })
    }
}

/// Generate a Noise X25519 static keypair (secret, public).
pub fn generate_keypair() -> Result<(Vec<u8>, Vec<u8>)> {
    let kp = Builder::new(PAIRING_PATTERN.parse().map_err(err)?)
        .generate_keypair()
        .map_err(err)?;
    Ok((kp.private, kp.public))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(mut a: Handshake, mut b: Handshake) -> Result<(Established, Established)> {
        let m1 = a.write(b"info-a")?;
        assert_eq!(b.read(&m1)?, b"info-a");
        let m2 = b.write(b"info-b")?;
        assert_eq!(a.read(&m2)?, b"info-b");
        Ok((a.finish()?, b.finish()?))
    }

    #[test]
    fn pairing_same_sas_and_transport() {
        let (phone_sk, phone_pk) = generate_keypair().unwrap();
        let (desk_sk, desk_pk) = generate_keypair().unwrap();
        let psk = [7u8; 32];
        let (mut p, mut d) = run(
            Handshake::pairing_initiator(&phone_sk, &desk_pk, &psk).unwrap(),
            Handshake::pairing_responder(&desk_sk, &psk).unwrap(),
        )
        .unwrap();
        assert_eq!(p.sas(), d.sas());
        assert_eq!(p.remote_static, desk_pk);
        assert_eq!(d.remote_static, phone_pk);
        let ct = p.encrypt(b"hello").unwrap();
        assert_eq!(d.decrypt(&ct).unwrap(), b"hello");
    }

    #[test]
    fn wrong_psk_fails() {
        let (phone_sk, _) = generate_keypair().unwrap();
        let (desk_sk, desk_pk) = generate_keypair().unwrap();
        assert!(run(
            Handshake::pairing_initiator(&phone_sk, &desk_pk, &[1; 32]).unwrap(),
            Handshake::pairing_responder(&desk_sk, &[2; 32]).unwrap(),
        )
        .is_err());
    }

    #[test]
    fn lan_kk_known_peers_only() {
        let (a_sk, a_pk) = generate_keypair().unwrap();
        let (b_sk, b_pk) = generate_keypair().unwrap();
        let (_, mallory_pk) = generate_keypair().unwrap();
        let (mut x, mut y) = run(
            Handshake::lan(&a_sk, &b_pk, true).unwrap(),
            Handshake::lan(&b_sk, &a_pk, false).unwrap(),
        )
        .unwrap();
        let ct = x.encrypt(b"env").unwrap();
        assert_eq!(y.decrypt(&ct).unwrap(), b"env");
        // Responder expecting a different initiator key rejects.
        assert!(run(
            Handshake::lan(&a_sk, &b_pk, true).unwrap(),
            Handshake::lan(&b_sk, &mallory_pk, false).unwrap(),
        )
        .is_err());
    }
}
