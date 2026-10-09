//! Trait for indicators that consume funding settlement events.

use crate::core::types::FundingSettlement;

/// Indicators that process funding settlement events.
pub trait FundingSettlementConsumer {
    /// Process a new funding settlement event and return the updated value.
    fn update_funding_settlement(&mut self, fs: &FundingSettlement) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
