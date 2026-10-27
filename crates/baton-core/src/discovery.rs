//! mDNS advert hint (BATON.md §3.2): `h = first 8 bytes of BLAKE2s(device_id ‖ weekly salt)`.
use blake2::{Blake2s256, Digest};

pub const SERVICE: &str = "_batonsync._tcp.local.";
const WEEK_S: u64 = 7 * 24 * 3600;

pub fn week(unix_s: u64) -> u64 {
    unix_s / WEEK_S
}

/// Hint for one device in one week. Passers-by can't track a stable ID; paired peers recompute it.
pub fn lan_hint(device_id: &[u8; 16], week: u64) -> String {
    let mut h = Blake2s256::new();
    h.update(device_id);
    h.update(week.to_be_bytes());
    hex::encode(&h.finalize()[..8])
}

/// Accept the current and previous week to tolerate clock/rotation boundaries.
pub fn hint_matches(hint: &str, device_id: &[u8; 16], now_unix_s: u64) -> bool {
    let w = week(now_unix_s);
    hint == lan_hint(device_id, w) || (w > 0 && hint == lan_hint(device_id, w - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotates_weekly_and_matches_neighbours() {
        let id = [7u8; 16];
        assert_eq!(lan_hint(&id, 5).len(), 16);
        assert_ne!(lan_hint(&id, 5), lan_hint(&id, 6));
        assert_ne!(lan_hint(&id, 5), lan_hint(&[8u8; 16], 5));
        let t = 6 * WEEK_S + 10;
        assert!(hint_matches(&lan_hint(&id, 6), &id, t));
        assert!(hint_matches(&lan_hint(&id, 5), &id, t));
        assert!(!hint_matches(&lan_hint(&id, 3), &id, t));
    }
}
