//! Integration tests for collector config parsing and EventWriter roundtrip.

use mli_collector_lib::config::CollectorConfig;
use mli_collector_lib::writer::EventWriter;
use mylittleindicators::core::types::FundingRate;
use mylittleindicators::data_loader::{StorageRoot, StreamKind, TimedEvent};

fn tempdir(tag: &str) -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("mli_collector_test_{}_{}", std::process::id(), tag));
    if p.exists() {
        std::fs::remove_dir_all(&p).unwrap();
    }
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn config_from_toml() {
    let toml = r#"
storage_dir = "/tmp/collector_data"
exchange = "binance"
symbols = ["BTCUSDT", "ETHUSDT"]

[[streams]]
kind = "Funding"

[[streams]]
kind = "OpenInterest"
symbols = ["BTCUSDT"]
"#;
    let config: CollectorConfig = toml::from_str(toml).expect("toml parse failed");
    assert_eq!(config.exchange, "binance");
    assert_eq!(config.symbols.len(), 2);
    assert_eq!(config.streams.len(), 2);
    assert_eq!(config.streams[0].kind, StreamKind::Funding);
    assert!(config.streams[0].symbols.is_empty());
    assert_eq!(config.streams[1].kind, StreamKind::OpenInterest);
    assert_eq!(config.streams[1].symbols, vec!["BTCUSDT"]);
}

#[test]
fn event_writer_roundtrip() {
    let dir = tempdir("writer_roundtrip");
    let writer = EventWriter::new(dir.clone());

    let event = TimedEvent::Funding(FundingRate {
        symbol: "BTCUSDT".into(),
        rate: 0.0001,
        next_funding_time: None,
        timestamp: 1_700_000_000_000,
    });

    writer.write("BTCUSDT", &event).expect("write failed");

    // Read back via StorageRoot.
    let storage = StorageRoot::new(&dir);
    let results = storage
        .read_range("BTCUSDT", StreamKind::Funding, 0, i64::MAX)
        .expect("read failed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].timestamp_ms(), 1_700_000_000_000);
}
