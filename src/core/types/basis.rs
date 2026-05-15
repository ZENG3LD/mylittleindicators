//! Basis — spread between futures price and spot index price.

/// Futures basis snapshot.
///
/// Basis = futures_price − spot_index_price.
/// Positive = contango (futures above spot), negative = backwardation.
/// `symbol` omitted — mli is symbol-agnostic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Basis {
    /// Futures price minus spot index price.
    pub basis: f64,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
}
