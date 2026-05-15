//! Composite index price — weighted basket of constituent symbols.

/// Composite index snapshot.
///
/// Represents a weighted basket price (e.g., Binance composite index).
/// `symbol` omitted — mli is symbol-agnostic.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompositeIndex {
    /// Weighted basket price.
    pub price: f64,
    /// Constituent symbols and their weights: `(symbol, weight)`.
    pub components: Vec<(String, f64)>,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
}
