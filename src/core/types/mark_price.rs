//! Mark price — exchange-computed fair price used for PnL and liquidations.

/// Mark price snapshot for a perpetual futures market.
#[derive(Debug, Clone)]
pub struct MarkPrice {
    /// Trading pair symbol.
    pub symbol: String,
    /// Exchange mark price.
    pub mark_price: f64,
    /// Spot/index price, if available.
    pub index_price: Option<f64>,
    /// Current funding rate embedded in the mark price feed, if provided.
    pub funding_rate: Option<f64>,
    /// Snapshot timestamp in milliseconds.
    pub timestamp: i64,
}
