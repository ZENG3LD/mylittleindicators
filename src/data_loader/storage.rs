//! Local binary-log storage for streams without public REST history.
//!
//! Format per stream file: append-only, each record:
//!   `[i64 timestamp_ms LE][u32 payload_len LE][payload_bytes (serde_json)]`
//!
//! Path layout: `{data_dir}/{symbol}/{stream_kind}.bin`

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

use super::{StreamKind, TimedEvent};

/// Root of the local binary storage tree.
pub struct StorageRoot {
    pub data_dir: PathBuf,
}

impl StorageRoot {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    /// Absolute path for a given symbol + stream kind.
    pub fn path_for(&self, symbol: &str, kind: StreamKind) -> PathBuf {
        self.data_dir
            .join(symbol)
            .join(format!("{}.bin", kind.as_str()))
    }

    /// Append one timed event to the appropriate stream file.
    ///
    /// Creates parent directory and file if missing.
    pub fn append(&self, symbol: &str, event: &TimedEvent) -> std::io::Result<()> {
        let path = self.path_for(symbol, event.stream_kind());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        let ts = event.timestamp_ms();
        let payload = serde_json::to_vec(event)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let payload_len = payload.len() as u32;
        file.write_all(&ts.to_le_bytes())?;
        file.write_all(&payload_len.to_le_bytes())?;
        file.write_all(&payload)?;
        Ok(())
    }

    /// Read all events from a stream file within timestamp range `[from_ts, to_ts]` inclusive.
    ///
    /// Returns an empty `Vec` when the file does not exist.
    pub fn read_range(
        &self,
        symbol: &str,
        kind: StreamKind,
        from_ts: i64,
        to_ts: i64,
    ) -> std::io::Result<Vec<TimedEvent>> {
        let path = self.path_for(symbol, kind);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let mut file = File::open(&path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        let mut result = Vec::new();
        let mut cursor = 0usize;
        while cursor + 12 <= buf.len() {
            let ts = i64::from_le_bytes(
                buf[cursor..cursor + 8]
                    .try_into()
                    .expect("slice is exactly 8 bytes — checked by while condition"),
            );
            let payload_len = u32::from_le_bytes(
                buf[cursor + 8..cursor + 12]
                    .try_into()
                    .expect("slice is exactly 4 bytes — checked by while condition"),
            ) as usize;
            cursor += 12;

            if cursor + payload_len > buf.len() {
                // Truncated record — stop gracefully.
                break;
            }

            if ts >= from_ts && ts <= to_ts {
                let event: TimedEvent =
                    serde_json::from_slice(&buf[cursor..cursor + payload_len])
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                result.push(event);
            }
            cursor += payload_len;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::StorageRoot;
    use crate::core::types::FundingRate;
    use crate::data_loader::TimedEvent;

    fn make_funding(ts: i64, rate: f64) -> TimedEvent {
        TimedEvent::Funding(FundingRate {
            symbol: "BTCUSDT".into(),
            rate,
            next_funding_time: None,
            timestamp: ts,
        })
    }

    #[test]
    fn roundtrip_single_event() {
        let dir = tempdir("roundtrip");
        let storage = StorageRoot::new(&dir);

        let ev = make_funding(1_000_000, 0.0001);
        storage.append("BTCUSDT", &ev).unwrap();

        let results = storage
            .read_range("BTCUSDT", crate::data_loader::StreamKind::Funding, 0, i64::MAX)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].timestamp_ms(), 1_000_000);
    }

    #[test]
    fn read_range_filters_by_timestamp() {
        let dir = tempdir("range_filter");
        let storage = StorageRoot::new(&dir);

        for ts in [1000i64, 2000, 3000, 4000, 5000] {
            storage.append("BTCUSDT", &make_funding(ts, 0.0001)).unwrap();
        }

        let results = storage
            .read_range("BTCUSDT", crate::data_loader::StreamKind::Funding, 2000, 4000)
            .unwrap();
        assert_eq!(results.len(), 3);
        let timestamps: Vec<i64> = results.iter().map(|e| e.timestamp_ms()).collect();
        assert_eq!(timestamps, vec![2000, 3000, 4000]);
    }

    #[test]
    fn missing_file_returns_empty() {
        let dir = tempdir("missing");
        let storage = StorageRoot::new(&dir);
        let result = storage
            .read_range("BTCUSDT", crate::data_loader::StreamKind::Funding, 0, i64::MAX)
            .unwrap();
        assert!(result.is_empty());
    }

    /// Create a fresh temp directory with a unique tag.
    ///
    /// Uses process id + tag so parallel test runs do not collide.
    /// The directory is created fresh (any pre-existing contents are removed).
    fn tempdir(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("mli_storage_test_{}_{}", std::process::id(), tag));
        if p.exists() {
            std::fs::remove_dir_all(&p).unwrap();
        }
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
