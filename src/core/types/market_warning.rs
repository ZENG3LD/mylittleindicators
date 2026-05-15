//! Market warning — exchange-issued alert for a specific symbol.

/// Market warning event.
///
/// `symbol` is kept here because warnings are inherently contextual to a specific
/// instrument — callers cannot route without knowing the target.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarketWarning {
    /// Symbol this warning applies to (e.g., `"BTCUSDT"`).
    pub symbol: String,
    /// Warning kind identifier (exchange-defined, e.g., `"high_volatility"`, `"margin_call"`).
    pub warning_kind: String,
    /// Human-readable warning message from the exchange.
    pub message: String,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
}
