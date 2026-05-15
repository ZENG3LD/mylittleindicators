//! Open interest — total number of outstanding derivative contracts.

/// Open interest snapshot for a futures/perpetual market.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OpenInterest {
    /// Trading pair symbol.
    pub symbol: String,
    /// Open interest in base currency units (e.g. number of contracts or coins).
    pub open_interest: f64,
    /// Open interest denominated in quote currency (USD notional), if available.
    pub open_interest_value: Option<f64>,
    /// Snapshot timestamp in milliseconds.
    pub timestamp: i64,
}
