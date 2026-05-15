//! Trait for indicators that consume auction event snapshots.

use crate::core::types::AuctionEvent;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process exchange auction event data.
pub trait AuctionEventConsumer {
    /// Process a new auction event and return the updated value.
    fn update_auction(&mut self, a: &AuctionEvent) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
