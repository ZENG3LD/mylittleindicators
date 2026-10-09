//! Trait for indicators that consume option Greeks snapshots.

use crate::core::types::OptionGreeks;

/// Indicators that process option Greeks data.
pub trait OptionGreeksConsumer {
    /// Process a new option Greeks snapshot and return the updated value.
    fn update_option_greeks(&mut self, g: &OptionGreeks) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
