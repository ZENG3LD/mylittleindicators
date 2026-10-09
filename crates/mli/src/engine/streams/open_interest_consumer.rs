//! Trait for indicators that consume open interest snapshots.

use crate::core::types::OpenInterest;

/// Indicators that process open interest snapshots.
pub trait OpenInterestConsumer {
    /// Process a new open interest snapshot and return updated value.
    fn update_oi(&mut self, oi: &OpenInterest) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if indicator has enough data to produce signals.
    fn is_ready(&self) -> bool;
}
