//! Aggregated trade — exchange-side aggregation of consecutive same-price trades.

/// Aggregated trade event.
///
/// Represents one or more consecutive trades at the same price, same side,
/// merged by the exchange into a single event.
/// `symbol` omitted — mli is symbol-agnostic.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AggTrade {
    /// Exchange-assigned aggregate trade id.
    pub aggregate_id: i64,
    /// Trade price.
    pub price: f64,
    /// Total quantity across all merged trades.
    pub quantity: f64,
    /// First constituent trade id.
    pub first_trade_id: i64,
    /// Last constituent trade id.
    pub last_trade_id: i64,
    /// `true` = buyer is maker (sell aggressor); `false` = buyer is taker (buy aggressor).
    pub is_buy: bool,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
}
