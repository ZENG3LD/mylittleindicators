//! Funding rate — perpetual futures funding rate snapshot.

/// Perpetual futures funding rate snapshot.
#[derive(Debug, Clone)]
pub struct FundingRate {
    /// Trading pair symbol.
    pub symbol: String,
    /// Current funding rate (e.g. 0.0001 = 0.01%).
    pub rate: f64,
    /// Timestamp of next funding settlement in milliseconds, if known.
    pub next_funding_time: Option<i64>,
    /// Snapshot timestamp in milliseconds.
    pub timestamp: i64,
}
