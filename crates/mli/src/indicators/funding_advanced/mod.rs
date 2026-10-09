//! Advanced funding rate indicators.

pub mod annualized_funding_rate;
pub mod funding_direction_shift;
pub mod funding_extreme_alert;

pub use annualized_funding_rate::AnnualizedFundingRate;
pub use funding_direction_shift::FundingDirectionShift;
pub use funding_extreme_alert::FundingExtremeAlert;

// Relocated from volume/ (Phase 1 reorg 2026-06-16): funding-stream squatters.
pub mod funding_momentum;
pub mod funding_price_divergence;
pub mod funding_z_score;
pub use funding_momentum::*;
pub use funding_price_divergence::*;
pub use funding_z_score::*;

// Relocated from risk_funding/ (Phase 1 reorg 2026-06-16): funding-stream indicators.
pub mod funding_drift;
pub mod funding_settlement_impact;
pub mod funding_time_decay;
pub mod predicted_funding_extreme;
pub mod settled_funding_momentum;
pub use funding_drift::*;
pub use funding_settlement_impact::*;
pub use funding_time_decay::*;
pub use predicted_funding_extreme::*;
pub use settled_funding_momentum::*;
