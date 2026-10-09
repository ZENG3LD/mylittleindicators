//! Trait for indicators that consume L2 orderbook snapshots.

use crate::core::types::OrderBook;

/// Indicators that process L2 orderbook data.
/// Used by `book/` category indicators.
pub trait OrderBookConsumer {
    /// Process a new orderbook snapshot and return updated value.
    fn update_orderbook(&mut self, book: &OrderBook) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if indicator has enough data to produce signals.
    fn is_ready(&self) -> bool;
}
