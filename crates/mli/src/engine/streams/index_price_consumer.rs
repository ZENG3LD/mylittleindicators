//! Trait for indicators that consume index price snapshots.

use crate::core::types::IndexPrice;

/// Indicators that process index price data.
pub trait IndexPriceConsumer {
    /// Process a new index price snapshot and return the updated value.
    fn update_index_price(&mut self, ip: &IndexPrice) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
