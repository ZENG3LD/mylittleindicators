//! Trait for indicators that consume contract settlement events.

use crate::core::types::SettlementEvent;

/// Indicators that process contract settlement events.
pub trait SettlementEventConsumer {
    /// Process a new settlement event and return the updated value.
    fn update_settlement(&mut self, s: &SettlementEvent) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
