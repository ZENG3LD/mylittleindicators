//! Trait for indicators that consume risk limit tier snapshots.

use crate::core::types::RiskLimit;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process exchange risk limit tier data.
pub trait RiskLimitConsumer {
    /// Process a new risk limit tier snapshot and return the updated value.
    fn update_risk_limit(&mut self, r: &RiskLimit) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
