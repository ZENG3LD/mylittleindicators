//! Trait for indicators that consume ticker (24-hour statistics) updates.

use crate::core::types::Ticker;

/// Indicators that process 24-hour ticker snapshots.
pub trait TickerConsumer {
    /// Process a new ticker snapshot and return updated value.
    fn update_ticker(&mut self, ticker: &Ticker) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if indicator has enough data to produce signals.
    fn is_ready(&self) -> bool;
}
