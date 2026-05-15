//! Historical volatility snapshot from exchange feeds.

/// Historical volatility snapshot.
///
/// Exchange-published realized/historical volatility metric.
/// `symbol` omitted — mli is symbol-agnostic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HistoricalVolatility {
    /// Annualized volatility value (e.g., 0.85 = 85%).
    pub volatility: f64,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
}
