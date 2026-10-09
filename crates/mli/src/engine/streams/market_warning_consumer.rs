//! Trait for indicators that consume market warning events.

use crate::core::types::MarketWarning;

/// Indicators that process exchange market warning events.
pub trait MarketWarningConsumer {
    /// Process a new market warning event and return the updated value.
    fn update_market_warning(&mut self, w: &MarketWarning) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
