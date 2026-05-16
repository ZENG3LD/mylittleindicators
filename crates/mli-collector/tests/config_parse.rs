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
fn config_from_toml_multi_exchange() {
    let toml = r#"
storage_dir = "/tmp/collector_data"

[[exchanges]]
id = "binance"
account_types = ["FuturesCross"]

[[exchanges.subscriptions]]
symbol = "BTCUSDT"
account_type = "FuturesCross"
stream_type = "FundingRate"

[[exchanges.subscriptions]]
symbol = "BTCUSDT"
account_type = "FuturesCross"
stream_type = "Liquidation"

[[exchanges]]
id = "bybit"
account_types = ["FuturesCross"]

[[exchanges.subscriptions]]
symbol = "BTCUSDT"
account_type = "FuturesCross"
stream_type = "Liquidation"
"#;
    let config: CollectorConfig = toml::from_str(toml).expect("toml parse failed");
    assert_eq!(config.exchanges.len(), 2);

    let binance = &config.exchanges[0];
    assert_eq!(binance.id.0, "binance");
    assert_eq!(binance.account_types.len(), 1);
    assert_eq!(binance.subscriptions.len(), 2);
    assert_eq!(binance.subscriptions[0].symbol, "BTCUSDT");
    assert_eq!(binance.subscriptions[0].stream_type.0.to_lowercase(), "fundingrate");

    let bybit = &config.exchanges[1];
    assert_eq!(bybit.id.0, "bybit");
    assert_eq!(bybit.subscriptions.len(), 1);

    // Parse helpers
    use digdigdig3::{AccountType, ExchangeId};
    assert_eq!(binance.exchange_id(), Some(ExchangeId::Binance));
    assert_eq!(binance.parsed_account_types(), vec![AccountType::FuturesCross]);
    assert_eq!(bybit.exchange_id(), Some(ExchangeId::Bybit));
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
