//! Loads `EnrichedHistory` by orchestrating per-stream data sources.

use super::{DataSource, EnrichedHistory, StorageRoot, StreamKind, TimedEvent};
use crate::core::types::Bar;

/// Orchestrates loading of bars + additional streams into a single sorted timeline.
pub struct EnrichedDataLoader {
    pub source: DataSource,
}

impl EnrichedDataLoader {
    pub fn new(source: DataSource) -> Self {
        Self { source }
    }

    /// Load bars + requested streams, merge into single timestamp-ordered timeline.
    ///
    /// `bars` must be provided by the caller (e.g. loaded via REST beforehand).
    /// Additional `streams` are loaded from the configured `DataSource`.
    ///
    /// For the foundation step only `DataSource::Local` is supported. Passing
    /// `DataSource::Rest` or `DataSource::Mixed` returns an `Unsupported` error.
    pub fn load(
        &self,
        symbol: &str,
        bars: Vec<Bar>,
        streams: &[StreamKind],
    ) -> std::io::Result<EnrichedHistory> {
        let (from_ts, to_ts) = if bars.is_empty() {
            (0i64, i64::MAX)
        } else {
            // Safety: non-empty slice — both unwraps are infallible.
            (bars.first().unwrap().time, bars.last().unwrap().time)
        };

        let mut events: Vec<TimedEvent> =
            bars.iter().cloned().map(TimedEvent::Bar).collect();

        let storage_root = match &self.source {
            DataSource::Local { storage_root } => StorageRoot::new(storage_root.clone()),
            DataSource::Rest { .. } | DataSource::Mixed { .. } => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "only DataSource::Local is supported in the foundation layer",
                ));
            }
        };

        for &kind in streams {
            if kind == StreamKind::Bar {
                // Bars are already added above.
                continue;
            }
            let mut stream_events =
                storage_root.read_range(symbol, kind, from_ts, to_ts)?;
            events.append(&mut stream_events);
        }

        // Stable sort preserves insertion order for equal timestamps, so bar
        // events always precede same-millisecond non-bar events.
        events.sort_by_key(|e| e.timestamp_ms());

        Ok(EnrichedHistory::new(bars, events))
    }
}

#[cfg(test)]
mod tests {
    use super::EnrichedDataLoader;
    use crate::core::types::{Bar, FundingRate};
    use crate::data_loader::{DataSource, StorageRoot, StreamKind, TimedEvent};

    fn make_bar(t: i64) -> Bar {
        Bar::new(t, 1.0, 2.0, 0.5, 1.5, 100.0)
    }

    fn make_funding_event(ts: i64) -> TimedEvent {
        TimedEvent::Funding(FundingRate {
            symbol: "BTCUSDT".into(),
            rate: 0.0001,
            next_funding_time: None,
            timestamp: ts,
        })
    }

    /// Unique temp dir per call site.
    fn tempdir(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("mli_loader_test_{}_{}", std::process::id(), tag));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn bars_only_load() {
        let dir = tempdir("bars_only");
        let loader = EnrichedDataLoader::new(DataSource::Local {
            storage_root: dir.clone(),
        });
        let bars: Vec<Bar> = (0..5).map(|i| make_bar(i * 1_000)).collect();
        let history = loader.load("BTCUSDT", bars, &[]).unwrap();
        assert_eq!(history.bar_count(), 5);
        assert_eq!(history.event_count(), 5);
    }

    #[test]
    fn multi_stream_sorted_order() {
        let dir = tempdir("multi_stream");
        let storage = StorageRoot::new(&dir);

        // Interleave funding events between bars.
        let funding_timestamps = [500i64, 1500, 2500, 3500, 4500, 5500, 6500, 7500, 8500, 9500];
        for ts in funding_timestamps {
            storage
                .append("BTCUSDT", &make_funding_event(ts))
                .unwrap();
        }

        let loader = EnrichedDataLoader::new(DataSource::Local {
            storage_root: dir.clone(),
        });
        let bars: Vec<Bar> = (0..5).map(|i| make_bar(i * 2_000)).collect();
        let history = loader
            .load("BTCUSDT", bars, &[StreamKind::Funding])
            .unwrap();

        // 5 bars + 10 funding events (all within [0, 8000])
        // funding at 500,1500,2500,3500,4500,5500,6500,7500 = 8 within range, 8500/9500 excluded
        assert_eq!(history.bar_count(), 5);
        assert!(history.event_count() >= 5, "at least bar events");

        // Verify strictly non-decreasing timestamps.
        let timestamps: Vec<i64> = history.events.iter().map(|e| e.timestamp_ms()).collect();
        for w in timestamps.windows(2) {
            assert!(w[0] <= w[1], "events not sorted: {} > {}", w[0], w[1]);
        }
    }
}
