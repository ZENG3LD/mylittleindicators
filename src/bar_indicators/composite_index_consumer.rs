//! Trait for indicators that consume composite index snapshots.

use crate::core::types::CompositeIndex;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process composite index data.
pub trait CompositeIndexConsumer {
    /// Process a new composite index snapshot and return the updated value.
    fn update_composite_index(&mut self, ci: &CompositeIndex) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
