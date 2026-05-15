//! Risk limit tier — exchange margin and leverage constraints per tier.

/// Risk limit tier snapshot.
///
/// Exchange-published margin tier defining maximum leverage and position size.
/// `symbol` omitted — mli is symbol-agnostic.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RiskLimit {
    /// Tier index (1-based, higher = larger position allowed).
    pub tier: u32,
    /// Maximum leverage allowed at this tier.
    pub max_leverage: f64,
    /// Maximum notional position value in quote currency at this tier.
    pub max_position_value: f64,
    /// Maintenance margin ratio (fraction, e.g., 0.005 = 0.5%).
    pub mmr: f64,
    /// Initial margin ratio (fraction, e.g., 0.01 = 1%).
    pub imr: f64,
    /// Event timestamp in milliseconds.
    pub timestamp: i64,
}
