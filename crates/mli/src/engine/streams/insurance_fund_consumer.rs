//! Trait for indicators that consume insurance fund balance snapshots.

use crate::core::types::InsuranceFund;

/// Indicators that process insurance fund balance data.
pub trait InsuranceFundConsumer {
    /// Process a new insurance fund snapshot and return the updated value.
    fn update_insurance_fund(&mut self, ins: &InsuranceFund) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
