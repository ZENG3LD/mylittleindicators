//! Trait for indicators that consume aggregated trade events.

use crate::core::types::AggTrade;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process aggregated trade events.
pub trait AggTradeConsumer {
    /// Process a new aggregated trade and return the updated value.
    fn update_agg_trade(&mut self, t: &AggTrade) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
