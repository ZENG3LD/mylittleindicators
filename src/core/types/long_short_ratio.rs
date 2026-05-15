//! Long/short ratio snapshot — balance of long vs short positions.

/// Long/short ratio snapshot from exchange data feeds.
///
/// `symbol` omitted — mli is symbol-agnostic; callers route per-symbol.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LongShortRatio {
    /// Ratio type: `"top_account"` | `"top_position"` | `"global_account"` | `"taker"`
    pub ratio_type: String,
    /// Fraction of accounts/positions that are long (0.0–1.0).
    pub long_ratio: f64,
    /// Fraction of accounts/positions that are short (0.0–1.0).
    pub short_ratio: f64,
    /// long / short ratio. `None` when short_ratio is zero.
    pub ratio: Option<f64>,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
}
