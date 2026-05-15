//! Ticker type — 24-hour market statistics snapshot.
//!
//! Mirrors `digdigdig3::core::types::market_data::Ticker` exactly so that
//! consumers of both crates can convert without allocation.

/// 24-hour market statistics snapshot (ticker stream).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Ticker {
    /// Trading symbol.
    pub symbol: String,
    /// Last traded price.
    pub last_price: f64,
    /// Best bid price.
    pub bid_price: Option<f64>,
    /// Best ask price.
    pub ask_price: Option<f64>,
    /// 24-hour high.
    pub high_24h: Option<f64>,
    /// 24-hour low.
    pub low_24h: Option<f64>,
    /// 24-hour volume in base asset.
    pub volume_24h: Option<f64>,
    /// 24-hour volume in quote asset.
    pub quote_volume_24h: Option<f64>,
    /// Absolute price change over 24 hours.
    pub price_change_24h: Option<f64>,
    /// Percentage price change over 24 hours.
    pub price_change_percent_24h: Option<f64>,
    /// Unix timestamp in milliseconds.
    pub timestamp: i64,
}
