//! Trait for indicators that consume public liquidation event streams.

use crate::core::types::Liquidation;

/// Indicators that process public liquidation events.
///
/// The bar pipeline is a no-op for these indicators; call
/// `update_liquidation` for each event received from the exchange feed.
pub trait LiquidationConsumer {
    /// Process a new liquidation event and return the updated indicator value.
    fn update_liquidation(&mut self, liq: &Liquidation) -> ();


    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
