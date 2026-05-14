//! Trait for indicators that consume tick (trade) stream data.

use crate::core::types::Tick;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process tick-by-tick trade data instead of (or in addition to)
/// bar OHLCV. Used by `clusters/` category indicators that need real buy/sell classification.
pub trait TickConsumer {
    /// Process a new tick and return updated value.
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if indicator has enough data to produce signals.
    fn is_ready(&self) -> bool;
}
