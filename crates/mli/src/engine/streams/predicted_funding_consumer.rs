//! Trait for indicators that consume predicted funding rate snapshots.

use crate::core::types::PredictedFunding;

/// Indicators that process predicted funding rate data.
pub trait PredictedFundingConsumer {
    /// Process a new predicted funding rate snapshot and return the updated value.
    fn update_predicted_funding(&mut self, pf: &PredictedFunding) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
