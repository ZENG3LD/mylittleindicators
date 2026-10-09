//! Trait for indicators that consume Level-3 orderbook events.

use crate::core::types::OrderbookL3Event;

/// Indicators that process individual L3 order-level add/modify/delete events.
pub trait OrderbookL3Consumer {
    /// Process a new L3 orderbook event and return the updated value.
    fn update_orderbook_l3(&mut self, l3: &OrderbookL3Event) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
