//! Trait for indicators that consume perpetual futures funding rate updates.

use crate::core::types::FundingRate;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process funding rate snapshots.
pub trait FundingRateConsumer {
    /// Process a new funding rate snapshot and return updated value.
    fn update_funding(&mut self, fr: &FundingRate) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if indicator has enough data to produce signals.
    fn is_ready(&self) -> bool;
}
