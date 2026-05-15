//! Option Greeks snapshot — sensitivity metrics for derivatives pricing.

/// Option Greeks snapshot from exchange feed.
///
/// Covers the standard first- and second-order sensitivities plus
/// implied volatility variants.
/// `symbol` omitted — mli is symbol-agnostic.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OptionGreeks {
    /// Delta — sensitivity to underlying price change (−1 to +1).
    pub delta: f64,
    /// Gamma — rate of change of delta.
    pub gamma: f64,
    /// Vega — sensitivity to implied volatility change.
    pub vega: f64,
    /// Theta — time decay per day.
    pub theta: f64,
    /// Rho — sensitivity to risk-free rate change.
    pub rho: f64,
    /// Mark implied volatility.
    pub mark_iv: f64,
    /// Best bid implied volatility. `None` when not provided by exchange.
    pub bid_iv: Option<f64>,
    /// Best ask implied volatility. `None` when not provided by exchange.
    pub ask_iv: Option<f64>,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
}
