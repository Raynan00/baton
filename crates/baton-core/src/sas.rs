//! 6-digit short authentication string (BATON.md §9.4).
use blake2::{Blake2s256, Digest};

/// `u32(BLAKE2s("baton-sas-v1" ‖ handshake_hash)[0..4]) mod 10^6`, zero-padded.
pub fn sas_code(handshake_hash: &[u8]) -> String {
    let mut h = Blake2s256::new();
    h.update(b"baton-sas-v1");
    h.update(handshake_hash);
    let d = h.finalize();
    let n = u32::from_be_bytes([d[0], d[1], d[2], d[3]]) % 1_000_000;
    format!("{n:06}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_digits_and_deterministic() {
        let a = sas_code(b"hash");
        assert_eq!(a.len(), 6);
        assert_eq!(a, sas_code(b"hash"));
        assert_ne!(a, sas_code(b"other"));
    }
}
