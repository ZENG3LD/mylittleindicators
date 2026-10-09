//! Trait for indicators that consume aggregated trade events.

use crate::core::types::AggTrade;

/// Indicators that process aggregated trade events.
pub trait AggTradeConsumer {
    /// Process a new aggregated trade and return the updated value.
    fn update_agg_trade(&mut self, t: &AggTrade) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
