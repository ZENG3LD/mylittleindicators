//! Trait for indicators that consume block trade events.

use crate::core::types::BlockTrade;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process block trade data.
pub trait BlockTradeConsumer {
    /// Process a new block trade event and return the updated value.
    fn update_block_trade(&mut self, bt: &BlockTrade) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
