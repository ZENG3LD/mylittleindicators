//! Insurance fund balance snapshot.

/// Insurance fund balance snapshot.
///
/// Exchange insurance fund used to cover losses from underwater liquidations.
/// `symbol` omitted — mli is symbol-agnostic.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InsuranceFund {
    /// Current fund balance in quote currency.
    pub balance: f64,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
}
