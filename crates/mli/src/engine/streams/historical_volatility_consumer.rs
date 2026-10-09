//! Trait for indicators that consume historical volatility snapshots.

use crate::core::types::HistoricalVolatility;

/// Indicators that process historical volatility data.
pub trait HistoricalVolatilityConsumer {
    /// Process a new historical volatility snapshot and return the updated value.
    fn update_historical_volatility(&mut self, hv: &HistoricalVolatility) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
