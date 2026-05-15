//! Volatility index snapshot — exchange-published implied volatility index.

/// Volatility index snapshot (e.g., DVOL, BVOL).
///
/// Exchange-published forward-looking implied volatility index.
/// `symbol` omitted — mli is symbol-agnostic.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VolatilityIndex {
    /// Index value (annualized implied volatility, e.g., 0.85 = 85%).
    pub value: f64,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
}
