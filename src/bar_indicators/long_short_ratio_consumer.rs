//! Trait for indicators that consume long/short ratio snapshots.

use crate::core::types::LongShortRatio;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process long/short ratio data.
pub trait LongShortRatioConsumer {
    /// Process a new long/short ratio snapshot and return the updated value.
    fn update_long_short_ratio(&mut self, lsr: &LongShortRatio) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
