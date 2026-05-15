//! Trait for indicators that consume volatility index snapshots.

use crate::core::types::VolatilityIndex;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process volatility index data (e.g., DVOL, BVOL).
pub trait VolatilityIndexConsumer {
    /// Process a new volatility index snapshot and return the updated value.
    fn update_volatility_index(&mut self, vi: &VolatilityIndex) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
