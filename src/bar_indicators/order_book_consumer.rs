//! Trait for indicators that consume L2 orderbook snapshots.

use crate::core::types::OrderBook;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process L2 orderbook data.
/// Used by `book/` category indicators.
pub trait OrderBookConsumer {
    /// Process a new orderbook snapshot and return updated value.
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if indicator has enough data to produce signals.
    fn is_ready(&self) -> bool;
}
