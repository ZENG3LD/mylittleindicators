//! Trait for indicators that consume index price snapshots.

use crate::core::types::IndexPrice;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process index price data.
pub trait IndexPriceConsumer {
    /// Process a new index price snapshot and return the updated value.
    fn update_index_price(&mut self, ip: &IndexPrice) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
