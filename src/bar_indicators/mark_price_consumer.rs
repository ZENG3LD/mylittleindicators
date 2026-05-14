//! Trait for indicators that consume mark price updates.

use crate::core::types::MarkPrice;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process mark price snapshots.
pub trait MarkPriceConsumer {
    /// Process a new mark price snapshot and return updated value.
    fn update_mark(&mut self, mp: &MarkPrice) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if indicator has enough data to produce signals.
    fn is_ready(&self) -> bool;
}
