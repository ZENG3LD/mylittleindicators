//! Trait for indicators that consume basis (futures−spot spread) snapshots.

use crate::core::types::Basis;

/// Indicators that process futures basis data.
pub trait BasisConsumer {
    /// Process a new basis snapshot and return the updated value.
    fn update_basis(&mut self, b: &Basis) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
