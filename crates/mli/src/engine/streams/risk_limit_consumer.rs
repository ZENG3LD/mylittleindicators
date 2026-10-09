//! Trait for indicators that consume risk limit tier snapshots.

use crate::core::types::RiskLimit;

/// Indicators that process exchange risk limit tier data.
pub trait RiskLimitConsumer {
    /// Process a new risk limit tier snapshot and return the updated value.
    fn update_risk_limit(&mut self, r: &RiskLimit) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
