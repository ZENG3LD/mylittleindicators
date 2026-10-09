//! Trait for indicators that consume composite index snapshots.

use crate::core::types::CompositeIndex;

/// Indicators that process composite index data.
pub trait CompositeIndexConsumer {
    /// Process a new composite index snapshot and return the updated value.
    fn update_composite_index(&mut self, ci: &CompositeIndex) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
