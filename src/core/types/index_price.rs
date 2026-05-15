//! Index price snapshot — underlying spot index for a derivatives market.

/// Index price snapshot.
///
/// Typically the spot price underlying a perpetual or futures contract.
/// `symbol` omitted — mli is symbol-agnostic.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IndexPrice {
    /// Index price value.
    pub price: f64,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
}
