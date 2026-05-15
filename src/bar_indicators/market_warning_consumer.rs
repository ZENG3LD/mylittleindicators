//! Trait for indicators that consume market warning events.

use crate::core::types::MarketWarning;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process exchange market warning events.
pub trait MarketWarningConsumer {
    /// Process a new market warning event and return the updated value.
    fn update_market_warning(&mut self, w: &MarketWarning) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
