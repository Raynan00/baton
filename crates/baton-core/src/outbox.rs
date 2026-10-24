//! SQLite outbox with retry backoff (BATON.md §12). Short transactions, no locks held across calls.
use crate::Result;
use rusqlite::{params, Connection};

/// Handoff items older than this are dropped as stale.
pub const STALE_HANDOFF_MS: u64 = 15 * 60 * 1000;
const BACKOFF_MIN_MS: u64 = 1_000;
const BACKOFF_MAX_MS: u64 = 5 * 60 * 1000;

pub struct Outbox {
    db: Connection,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Item {
    pub msg_id: Vec<u8>,
    pub envelope: Vec<u8>,
    pub attempts: u32,
}

/// Exponential backoff 1 s → 5 min; `jitter` in 0..=100 is a percent of the delay added.
pub fn backoff_ms(attempts: u32, jitter: u64) -> u64 {
    let base = BACKOFF_MIN_MS
        .saturating_mul(1u64 << attempts.min(20))
        .min(BACKOFF_MAX_MS);
    base + base * jitter.min(100) / 100 / 4
}

impl Outbox {
    pub fn open(path: &str) -> Result<Self> {
        let db = Connection::open(path)?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS outbox (
               msg_id BLOB PRIMARY KEY, envelope BLOB NOT NULL, created_ms INTEGER NOT NULL,
               stale_after_ms INTEGER NOT NULL, attempts INTEGER NOT NULL DEFAULT 0,
               next_try_ms INTEGER NOT NULL);",
        )?;
        Ok(Self { db })
    }

    pub fn enqueue(
        &self,
        msg_id: &[u8],
        envelope: &[u8],
        now_ms: u64,
        stale_after_ms: u64,
    ) -> Result<()> {
        self.db.execute(
            "INSERT OR IGNORE INTO outbox (msg_id, envelope, created_ms, stale_after_ms, next_try_ms)
             VALUES (?1, ?2, ?3, ?4, ?3)",
            params![msg_id, envelope, now_ms as i64, stale_after_ms as i64],
        )?;
        Ok(())
    }

    /// Drop stale items, then return those due for a send attempt.
    pub fn due(&self, now_ms: u64) -> Result<Vec<Item>> {
        self.db.execute(
            "DELETE FROM outbox WHERE created_ms + stale_after_ms < ?1",
            params![now_ms as i64],
        )?;
        let mut st = self.db.prepare(
            "SELECT msg_id, envelope, attempts FROM outbox WHERE next_try_ms <= ?1 ORDER BY created_ms",
        )?;
        let rows = st.query_map(params![now_ms as i64], |r| {
            Ok(Item {
                msg_id: r.get(0)?,
                envelope: r.get(1)?,
                attempts: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn mark_failed(&self, msg_id: &[u8], now_ms: u64, jitter: u64) -> Result<()> {
        let attempts: u32 = self
            .db
            .query_row(
                "SELECT attempts FROM outbox WHERE msg_id = ?1",
                params![msg_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        self.db.execute(
            "UPDATE outbox SET attempts = attempts + 1, next_try_ms = ?2 WHERE msg_id = ?1",
            params![msg_id, (now_ms + backoff_ms(attempts, jitter)) as i64],
        )?;
        Ok(())
    }

    pub fn remove(&self, msg_id: &[u8]) -> Result<()> {
        self.db
            .execute("DELETE FROM outbox WHERE msg_id = ?1", params![msg_id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_caps() {
        assert_eq!(backoff_ms(0, 0), 1_000);
        assert_eq!(backoff_ms(3, 0), 8_000);
        assert_eq!(backoff_ms(30, 0), BACKOFF_MAX_MS);
    }

    #[test]
    fn retry_stale_and_remove() {
        let o = Outbox::open(":memory:").unwrap();
        o.enqueue(b"a", b"env-a", 0, STALE_HANDOFF_MS).unwrap();
        o.enqueue(b"b", b"env-b", 0, 10).unwrap();
        o.enqueue(b"a", b"dup", 0, STALE_HANDOFF_MS).unwrap(); // idempotent
        assert_eq!(o.due(5).unwrap().len(), 2);
        // b goes stale at t=11
        let due = o.due(11).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].envelope, b"env-a");
        o.mark_failed(b"a", 11, 0).unwrap();
        assert!(o.due(500).unwrap().is_empty());
        assert_eq!(o.due(1_011).unwrap()[0].attempts, 1);
        o.remove(b"a").unwrap();
        assert!(o.due(2_000).unwrap().is_empty());
    }
}
