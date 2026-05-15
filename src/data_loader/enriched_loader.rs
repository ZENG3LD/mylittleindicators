//! Loads `EnrichedHistory` by orchestrating per-stream data sources.

use std::path::PathBuf;
use std::sync::Arc;

use super::{DataSource, EnrichedHistory, StorageRoot, StreamKind, TimedEvent};
use super::rest_fetcher::RestFetcher;
use crate::core::types::Bar;

/// Orchestrates loading of bars + additional streams into a single sorted timeline.
pub struct EnrichedDataLoader {
    pub source: DataSource,
    rest_fetcher: Option<Arc<dyn RestFetcher>>,
}

impl EnrichedDataLoader {
    pub fn new(source: DataSource) -> Self {
        Self { source, rest_fetcher: None }
    }

    /// Attach a `RestFetcher` implementation for `DataSource::Rest` variants.
    ///
    /// Without a fetcher, any `Rest` source returns `ErrorKind::Unsupported`.
    pub fn with_rest_fetcher(mut self, fetcher: Arc<dyn RestFetcher>) -> Self {
        self.rest_fetcher = Some(fetcher);
        self
    }

    /// Load bars + requested streams, merge into single timestamp-ordered timeline.
    ///
    /// `bars` must be provided by the caller (e.g. loaded via REST beforehand).
    /// Additional `streams` are loaded from the configured `DataSource`.
    pub fn load(
        &self,
        symbol: &str,
        bars: Vec<Bar>,
        streams: &[StreamKind],
    ) -> std::io::Result<EnrichedHistory> {
        let (from_ts, to_ts) = if bars.is_empty() {
            (0i64, i64::MAX)
        } else {
            // Non-empty slice — first/last unwraps are infallible.
            (bars.first().unwrap().time, bars.last().unwrap().time)
        };

        let mut events: Vec<TimedEvent> =
            bars.iter().cloned().map(TimedEvent::Bar).collect();

        for &kind in streams {
            if kind == StreamKind::Bar {
                // Bars already added above.
                continue;
            }
            let mut stream_events =
                self.load_stream_events(&self.source, symbol, kind, from_ts, to_ts)?;
            events.append(&mut stream_events);
        }

        // Stable sort: bar events precede same-millisecond non-bar events.
        events.sort_by_key(|e| e.timestamp_ms());

        Ok(EnrichedHistory::new(bars, events))
    }

    fn load_stream_events(
        &self,
        source: &DataSource,
        symbol: &str,
        kind: StreamKind,
        from_ts: i64,
        to_ts: i64,
    ) -> std::io::Result<Vec<TimedEvent>> {
        match source {
            DataSource::Binary { storage_root } => {
                StorageRoot::new(storage_root.clone()).read_range(symbol, kind, from_ts, to_ts)
            }
            DataSource::Json { storage_root } => {
                self.read_json(storage_root, symbol, kind, from_ts, to_ts)
            }
            DataSource::Rest { exchange: _ } => {
                if let Some(fetcher) = &self.rest_fetcher {
                    fetcher
                        .fetch(symbol, kind, from_ts, to_ts)
                        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Unsupported,
                        "Rest source requires RestFetcher (use with_rest_fetcher)",
                    ))
                }
            }
            DataSource::Mixed { per_stream } => {
                if let Some(sub) = per_stream.get(&kind) {
                    self.load_stream_events(sub, symbol, kind, from_ts, to_ts)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Mixed source has no entry for {:?}", kind),
                    ))
                }
            }
        }
    }

    fn read_json(
        &self,
        root: &PathBuf,
        symbol: &str,
        kind: StreamKind,
        from_ts: i64,
        to_ts: i64,
    ) -> std::io::Result<Vec<TimedEvent>> {
        let path = root.join(symbol).join(format!("{}.json", kind.as_str()));
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&path)?;
        let all: Vec<TimedEvent> = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(all
            .into_iter()
            .filter(|e| {
                let ts = e.timestamp_ms();
                ts >= from_ts && ts <= to_ts
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{EnrichedDataLoader, RestFetcher};
    use crate::core::types::{Bar, FundingRate, OpenInterest};
    use crate::data_loader::{DataSource, StorageRoot, StreamKind, TimedEvent};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;

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

    fn make_oi_event(ts: i64) -> TimedEvent {
        TimedEvent::OpenInterest(OpenInterest {
            symbol: "BTCUSDT".into(),
            open_interest: 1000.0,
            open_interest_value: None,
            timestamp: ts,
        })
    }

    fn tempdir(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("mli_loader_test_{}_{}", std::process::id(), tag));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    // ---- Binary (formerly Local) ----

    #[test]
    fn binary_bars_only_load() {
        let dir = tempdir("binary_bars_only");
        let loader = EnrichedDataLoader::new(DataSource::Binary {
            storage_root: dir.clone(),
        });
        let bars: Vec<Bar> = (0..5).map(|i| make_bar(i * 1_000)).collect();
        let history = loader.load("BTCUSDT", bars, &[]).unwrap();
        assert_eq!(history.bar_count(), 5);
        assert_eq!(history.event_count(), 5);
    }

    #[test]
    fn binary_multi_stream_sorted_order() {
        let dir = tempdir("binary_multi_stream");
        let storage = StorageRoot::new(&dir);

        let funding_timestamps = [500i64, 1500, 2500, 3500, 4500, 5500, 6500, 7500, 8500, 9500];
        for ts in funding_timestamps {
            storage.append("BTCUSDT", &make_funding_event(ts)).unwrap();
        }

        let loader = EnrichedDataLoader::new(DataSource::Binary {
            storage_root: dir.clone(),
        });
        let bars: Vec<Bar> = (0..5).map(|i| make_bar(i * 2_000)).collect();
        let history = loader
            .load("BTCUSDT", bars, &[StreamKind::Funding])
            .unwrap();

        assert_eq!(history.bar_count(), 5);
        assert!(history.event_count() >= 5);

        let timestamps: Vec<i64> = history.events.iter().map(|e| e.timestamp_ms()).collect();
        for w in timestamps.windows(2) {
            assert!(w[0] <= w[1], "events not sorted: {} > {}", w[0], w[1]);
        }
    }

    // ---- Json ----

    #[test]
    fn json_read_filters_by_timestamp() {
        let dir = tempdir("json_filter");
        let symbol_dir = dir.join("BTCUSDT");
        std::fs::create_dir_all(&symbol_dir).unwrap();

        // Write JSON array with 5 funding events.
        let events: Vec<TimedEvent> = [1000i64, 2000, 3000, 4000, 5000]
            .iter()
            .map(|&ts| make_funding_event(ts))
            .collect();
        let json = serde_json::to_string(&events).unwrap();
        std::fs::write(symbol_dir.join("funding.json"), json).unwrap();

        let loader = EnrichedDataLoader::new(DataSource::Json {
            storage_root: dir.clone(),
        });
        let bars: Vec<Bar> = vec![make_bar(2000), make_bar(4000)];
        let history = loader
            .load("BTCUSDT", bars, &[StreamKind::Funding])
            .unwrap();

        // from_ts=2000, to_ts=4000 → events at 2000, 3000, 4000 pass filter.
        let funding_count = history
            .events
            .iter()
            .filter(|e| matches!(e, TimedEvent::Funding(_)))
            .count();
        assert_eq!(funding_count, 3, "expected 3 funding events in [2000,4000]");
    }

    #[test]
    fn json_missing_file_returns_empty() {
        let dir = tempdir("json_missing");
        let loader = EnrichedDataLoader::new(DataSource::Json {
            storage_root: dir.clone(),
        });
        let bars = vec![make_bar(1000)];
        let history = loader
            .load("BTCUSDT", bars, &[StreamKind::Funding])
            .unwrap();
        // Only the bar event, no funding.
        assert_eq!(history.event_count(), 1);
    }

    // ---- Rest without fetcher ----

    #[test]
    fn rest_without_fetcher_returns_unsupported() {
        let loader = EnrichedDataLoader::new(DataSource::Rest {
            exchange: "binance".into(),
        });
        let bars = vec![make_bar(1000)];
        let err = loader
            .load("BTCUSDT", bars, &[StreamKind::Funding])
            .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }

    // ---- Rest with fetcher ----

    struct StaticFetcher(Vec<TimedEvent>);

    impl RestFetcher for StaticFetcher {
        fn fetch(
            &self,
            _symbol: &str,
            _kind: StreamKind,
            from_ts: i64,
            to_ts: i64,
        ) -> Result<Vec<TimedEvent>, String> {
            Ok(self
                .0
                .iter()
                .cloned()
                .filter(|e| {
                    let ts = e.timestamp_ms();
                    ts >= from_ts && ts <= to_ts
                })
                .collect())
        }
    }

    #[test]
    fn rest_with_fetcher_returns_events() {
        let fetcher_events: Vec<TimedEvent> =
            [500i64, 1500, 2500].iter().map(|&ts| make_funding_event(ts)).collect();
        let fetcher = Arc::new(StaticFetcher(fetcher_events));

        let loader = EnrichedDataLoader::new(DataSource::Rest {
            exchange: "binance".into(),
        })
        .with_rest_fetcher(fetcher);

        let bars = vec![make_bar(1000), make_bar(2000)];
        let history = loader
            .load("BTCUSDT", bars, &[StreamKind::Funding])
            .unwrap();

        // from_ts=1000, to_ts=2000 → fetcher returns 1500, 2500 filtered to 1500.
        let funding_count = history
            .events
            .iter()
            .filter(|e| matches!(e, TimedEvent::Funding(_)))
            .count();
        assert_eq!(funding_count, 1, "only ts=1500 within [1000,2000]");
    }

    // ---- Mixed ----

    #[test]
    fn mixed_per_stream_routing() {
        // Binary for Funding, Json for OpenInterest.
        let binary_dir = tempdir("mixed_binary");
        let json_dir = tempdir("mixed_json");

        // Populate binary store with funding events.
        let storage = StorageRoot::new(&binary_dir);
        for ts in [1000i64, 2000, 3000] {
            storage.append("BTCUSDT", &make_funding_event(ts)).unwrap();
        }

        // Populate JSON store with OI events.
        let oi_dir = json_dir.join("BTCUSDT");
        std::fs::create_dir_all(&oi_dir).unwrap();
        let oi_events: Vec<TimedEvent> =
            [1500i64, 2500].iter().map(|&ts| make_oi_event(ts)).collect();
        std::fs::write(oi_dir.join("open_interest.json"), serde_json::to_string(&oi_events).unwrap()).unwrap();

        let mut per_stream: HashMap<StreamKind, Box<DataSource>> = HashMap::new();
        per_stream.insert(
            StreamKind::Funding,
            Box::new(DataSource::Binary { storage_root: binary_dir }),
        );
        per_stream.insert(
            StreamKind::OpenInterest,
            Box::new(DataSource::Json { storage_root: json_dir }),
        );

        let loader = EnrichedDataLoader::new(DataSource::Mixed { per_stream });
        let bars = vec![make_bar(1000), make_bar(2000), make_bar(3000)];
        let history = loader
            .load("BTCUSDT", bars, &[StreamKind::Funding, StreamKind::OpenInterest])
            .unwrap();

        let funding_count = history
            .events
            .iter()
            .filter(|e| matches!(e, TimedEvent::Funding(_)))
            .count();
        let oi_count = history
            .events
            .iter()
            .filter(|e| matches!(e, TimedEvent::OpenInterest(_)))
            .count();

        assert_eq!(funding_count, 3);
        assert_eq!(oi_count, 2);
    }

    #[test]
    fn mixed_missing_stream_returns_not_found() {
        let per_stream: HashMap<StreamKind, Box<DataSource>> = HashMap::new();
        let loader = EnrichedDataLoader::new(DataSource::Mixed { per_stream });
        let bars = vec![make_bar(1000)];
        let err = loader
            .load("BTCUSDT", bars, &[StreamKind::Funding])
            .unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
