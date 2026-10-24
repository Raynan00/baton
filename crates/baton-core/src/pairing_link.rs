//! Pairing link carried by the desktop QR (BATON.md §5, §9.4).
//! Params live in the URL fragment so the one-time secret never reaches a web server.
use crate::{Error, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};

const BASE: &str = "https://batonsync.app/p#";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingLink {
    /// Rendezvous id for the relay fallback.
    pub rid: [u8; 16],
    /// Desktop static X25519 public key.
    pub desktop_pub: [u8; 32],
    /// One-time pre-shared secret.
    pub psk: [u8; 32],
    /// LAN address hints, `ip:port`.
    pub addrs: Vec<String>,
}

impl PairingLink {
    pub fn to_url(&self) -> String {
        format!(
            "{BASE}v=1&rid={}&k={}&psk={}&a={}",
            B64.encode(self.rid),
            B64.encode(self.desktop_pub),
            B64.encode(self.psk),
            self.addrs.join(",")
        )
    }

    pub fn parse(link: &str) -> Result<Self> {
        let frag = link
            .strip_prefix(BASE)
            .or_else(|| link.strip_prefix("batonsync://pair#"))
            .ok_or(Error::Envelope("not a pairing link"))?;
        let (mut v, mut rid, mut k, mut psk, mut a) = (None, None, None, None, "");
        for kv in frag.split('&') {
            match kv.split_once('=') {
                Some(("v", x)) => v = Some(x),
                Some(("rid", x)) => rid = Some(x),
                Some(("k", x)) => k = Some(x),
                Some(("psk", x)) => psk = Some(x),
                Some(("a", x)) => a = x,
                _ => {}
            }
        }
        if v != Some("1") {
            return Err(Error::Envelope("unsupported pairing link version"));
        }
        let dec = |x: Option<&str>| -> Result<Vec<u8>> {
            B64.decode(x.ok_or(Error::Envelope("missing field"))?)
                .map_err(|_| Error::Envelope("bad base64"))
        };
        let fixed = |v: Vec<u8>| -> Result<[u8; 32]> {
            v.try_into().map_err(|_| Error::Envelope("bad key length"))
        };
        Ok(Self {
            rid: dec(rid)?
                .try_into()
                .map_err(|_| Error::Envelope("bad rid length"))?,
            desktop_pub: fixed(dec(k)?)?,
            psk: fixed(dec(psk)?)?,
            addrs: a
                .split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let l = PairingLink {
            rid: [1; 16],
            desktop_pub: [2; 32],
            psk: [3; 32],
            addrs: vec!["192.168.1.5:4242".into(), "10.0.0.2:4242".into()],
        };
        assert_eq!(PairingLink::parse(&l.to_url()).unwrap(), l);
    }

    #[test]
    fn rejects_garbage() {
        assert!(PairingLink::parse("https://evil.example/p#v=1").is_err());
        assert!(PairingLink::parse("https://batonsync.app/p#v=2&rid=a").is_err());
        assert!(PairingLink::parse("https://batonsync.app/p#v=1&rid=AA&k=AA&psk=AA").is_err());
    }
}
