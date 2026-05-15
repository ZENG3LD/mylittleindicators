//! Trait for indicators that consume historical volatility snapshots.

use crate::core::types::HistoricalVolatility;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process historical volatility data.
pub trait HistoricalVolatilityConsumer {
    /// Process a new historical volatility snapshot and return the updated value.
    fn update_historical_volatility(&mut self, hv: &HistoricalVolatility) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
