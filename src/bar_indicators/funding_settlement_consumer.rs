//! Trait for indicators that consume funding settlement events.

use crate::core::types::FundingSettlement;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process funding settlement events.
pub trait FundingSettlementConsumer {
    /// Process a new funding settlement event and return the updated value.
    fn update_funding_settlement(&mut self, fs: &FundingSettlement) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if the indicator has enough data to produce reliable signals.
    fn is_ready(&self) -> bool;
}
