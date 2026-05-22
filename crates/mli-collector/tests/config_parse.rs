//! Integration tests for collector config parsing.

use digdigdig3_station::Stream;
use mli_collector_lib::config::CollectorConfig;

#[test]
fn config_from_toml_multi_exchange() {
    let toml = r#"
storage_dir = "/tmp/collector_data"
warm_start = 100

[[exchanges]]
id = "binance"

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

[[exchanges.subscriptions]]
symbol = "BTCUSDT"
account_type = "FuturesCross"
stream_type = "Liquidation"
"#;
    let config: CollectorConfig = toml::from_str(toml).expect("toml parse failed");
    assert_eq!(config.exchanges.len(), 2);
    assert_eq!(config.warm_start, 100);

    let binance = &config.exchanges[0];
    assert_eq!(binance.id.0, "binance");
    assert_eq!(binance.subscriptions.len(), 2);

    use digdigdig3::ExchangeId;
    assert_eq!(binance.exchange_id(), Some(ExchangeId::Binance));
    assert_eq!(config.exchanges[1].exchange_id(), Some(ExchangeId::Bybit));
}

#[test]
fn stream_type_parses_to_station_stream() {
    use mli_collector_lib::config::StreamTypeStr;
    assert!(matches!(StreamTypeStr("trade".into()).parse(), Some(Stream::Trade)));
    assert!(matches!(StreamTypeStr("Ticker".into()).parse(), Some(Stream::Ticker)));
    assert!(matches!(StreamTypeStr("FundingRate".into()).parse(), Some(Stream::FundingRate)));
    assert!(matches!(StreamTypeStr("liquidation".into()).parse(), Some(Stream::Liquidation)));
    assert!(matches!(StreamTypeStr("kline".into()).parse(), Some(Stream::Kline(_))));
    assert!(matches!(StreamTypeStr("kline:5m".into()).parse(), Some(Stream::Kline(_))));
    // Unsupported by Station (no corresponding variant)
    assert!(StreamTypeStr("composite_index".into()).parse().is_none());
    assert!(StreamTypeStr("option_greeks".into()).parse().is_none());
}

#[test]
fn warm_start_defaults_to_zero() {
    let toml = r#"
storage_dir = "/tmp/x"

[[exchanges]]
id = "binance"
"#;
    let cfg: CollectorConfig = toml::from_str(toml).expect("parse");
    assert_eq!(cfg.warm_start, 0);
}
