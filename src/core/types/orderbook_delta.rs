//! Orderbook delta — incremental L2 updates.
//!
//! Each level represents either:
//! - new/updated price level (size > 0)
//! - removed price level (size == 0)
//!
//! `prev_update_id` allows gap detection between sequential deltas.

use crate::core::types::OrderBookLevel;

/// Incremental L2 orderbook update.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderbookDelta {
    /// Bid side changes. Level with `size == 0.0` means removal.
    pub bids: Vec<OrderBookLevel>,
    /// Ask side changes. Level with `size == 0.0` means removal.
    pub asks: Vec<OrderBookLevel>,
    /// Timestamp in milliseconds.
    pub timestamp: i64,
    /// First update id in this delta (exchange-provided sequence).
    pub first_update_id: Option<u64>,
    /// Last update id in this delta.
    pub last_update_id: Option<u64>,
    /// Previous update id for gap detection (None = first delta).
    pub prev_update_id: Option<u64>,
}

impl OrderbookDelta {
    /// Levels that were removed (size == 0.0).
    pub fn removed_bids(&self) -> impl Iterator<Item = f64> + '_ {
        self.bids.iter().filter(|l| l.size == 0.0).map(|l| l.price)
    }

    /// Levels that were removed on ask side (size == 0.0).
    pub fn removed_asks(&self) -> impl Iterator<Item = f64> + '_ {
        self.asks.iter().filter(|l| l.size == 0.0).map(|l| l.price)
    }

    /// Levels that were added or updated on bid side (size > 0.0).
    pub fn updated_bids(&self) -> impl Iterator<Item = &OrderBookLevel> {
        self.bids.iter().filter(|l| l.size > 0.0)
    }

    /// Levels that were added or updated on ask side (size > 0.0).
    pub fn updated_asks(&self) -> impl Iterator<Item = &OrderBookLevel> {
        self.asks.iter().filter(|l| l.size > 0.0)
    }

    /// Total number of changed levels across both sides.
    pub fn total_changes(&self) -> usize {
        self.bids.len() + self.asks.len()
    }
}
