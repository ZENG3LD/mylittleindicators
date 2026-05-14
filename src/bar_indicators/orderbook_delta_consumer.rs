//! Trait for indicators that consume incremental L2 orderbook deltas.

use crate::core::types::OrderbookDelta;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Indicators that process incremental L2 orderbook delta updates.
///
/// Delta updates provide added/updated/removed price levels rather than
/// full snapshots, enabling efficient tracking of orderbook state changes.
pub trait OrderbookDeltaConsumer {
    /// Process a new delta and return updated indicator value.
    fn update_delta(&mut self, delta: &OrderbookDelta) -> IndicatorValue;

    /// Current value without updating.
    fn value(&self) -> IndicatorValue;

    /// Reset internal state.
    fn reset(&mut self);

    /// True if indicator has enough data to produce signals.
    fn is_ready(&self) -> bool;
}
