//! Trait for indicators that consume auction event snapshots.

use crate::core::types::AuctionEvent;

/// Indicators that process exchange auction event data.
pub trait AuctionEventConsumer {
    /// Process a new auction event and return the updated value.
    fn update_auction(&mut self, a: &AuctionEvent) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
