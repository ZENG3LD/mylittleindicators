//! Trait for indicators that consume basis (futures−spot spread) snapshots.

use crate::core::types::Basis;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process futures basis data.
pub trait BasisConsumer {
    /// Process a new basis snapshot and return the updated value.
    fn update_basis(&mut self, b: &Basis) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
