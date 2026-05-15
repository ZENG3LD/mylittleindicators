//! Trait for indicators that consume option Greeks snapshots.

use crate::core::types::OptionGreeks;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process option Greeks data.
pub trait OptionGreeksConsumer {
    /// Process a new option Greeks snapshot and return the updated value.
    fn update_option_greeks(&mut self, g: &OptionGreeks) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
