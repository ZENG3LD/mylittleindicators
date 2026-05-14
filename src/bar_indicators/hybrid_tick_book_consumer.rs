//! Consumer trait for indicators requiring synchronized tick + orderbook state.
//!
//! Workflow:
//! 1. Caller maintains a current OrderBook snapshot (via update_orderbook)
//! 2. On each Tick, caller passes the trade together with the latest book
//! 3. Indicator compares trade execution vs visible book liquidity

use crate::core::types::{Tick, OrderBook};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicator that requires both a trade tick and the current orderbook state.
///
/// The caller is responsible for maintaining the latest `OrderBook` snapshot.
/// On each trade, call `update_tick_with_book` passing the book as it was
/// immediately before the trade. Optionally call `update_book_only` on book
/// updates where no trade occurred, to keep internal state fresh.
pub trait HybridTickBookConsumer {
    /// Called when a trade is observed. `book` is the orderbook snapshot
    /// immediately before this trade (caller's responsibility to maintain).
    fn update_tick_with_book(&mut self, tick: &Tick, book: &OrderBook) -> IndicatorValue;

    /// Optional: update internal book reference without a trade.
    /// Default is a no-op — indicators that need book state between ticks override this.
    fn update_book_only(&mut self, book: &OrderBook);

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if indicator has enough data to produce signals.
    fn is_ready(&self) -> bool;
}
