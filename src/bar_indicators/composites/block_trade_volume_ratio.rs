//! BlockTradeVolumeRatio — rolling ratio of block trade volume to aggregate trade volume.
//!
//! Dual consumer: `BlockTradeConsumer` + `AggTradeConsumer`.
//!
//! Logic:
//! - Rolling window `window_ms`
//! - `ratio = sum(block_quote_volume) / sum(agg_quote_volume)` within window
//! - If `agg_volume == 0` → `0.0`
//!
//! Output: `Single(ratio)`.

use std::collections::VecDeque;

use crate::bar_indicators::agg_trade_consumer::AggTradeConsumer;
use crate::bar_indicators::block_trade_consumer::BlockTradeConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::types::{AggTrade, BlockTrade};

/// Rolling block-to-aggregate trade volume ratio.
///
/// Implements both `BlockTradeConsumer` and `AggTradeConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Clone)]
pub struct BlockTradeVolumeRatio {
    window_ms: i64,
    block_events: VecDeque<(i64, f64)>,
    agg_events: VecDeque<(i64, f64)>,
    last_ratio: f64,
    last_ts: i64,
}

impl BlockTradeVolumeRatio {
    /// Create a new indicator.
    ///
    /// - `window_ms` — rolling time window in milliseconds (default 60_000).
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms,
            block_events: VecDeque::new(),
            agg_events: VecDeque::new(),
            last_ratio: 0.0,
            last_ts: 0,
        }
    }

    /// Evict both queues using the global latest timestamp.
    fn evict_all(&mut self, now: i64) {
        self.last_ts = self.last_ts.max(now);
        let cutoff = self.last_ts - self.window_ms;
        while self.block_events.front().map_or(false, |(ts, _)| *ts < cutoff) {
            self.block_events.pop_front();
        }
        while self.agg_events.front().map_or(false, |(ts, _)| *ts < cutoff) {
            self.agg_events.pop_front();
        }
    }

    fn recompute(&mut self) {
        let block_sum: f64 = self.block_events.iter().map(|(_, v)| v).sum();
        let agg_sum: f64 = self.agg_events.iter().map(|(_, v)| v).sum();
        self.last_ratio = if agg_sum > 0.0 { block_sum / agg_sum } else { 0.0 };
    }

    /// Passthrough for bar pipeline — returns current value.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_ratio)
    }

    /// True when at least one agg trade has been received.
    pub fn indicator_is_ready(&self) -> bool {
        !self.agg_events.is_empty()
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.block_events.clear();
        self.agg_events.clear();
        self.last_ratio = 0.0;
        self.last_ts = 0;
    }
}

impl Default for BlockTradeVolumeRatio {
    fn default() -> Self {
        Self::new(60_000)
    }
}

impl BlockTradeConsumer for BlockTradeVolumeRatio {
    fn update_block_trade(&mut self, bt: &BlockTrade) -> IndicatorValue {
        // Skip IV-priced block trades — not comparable in quote volume
        if bt.is_iv {
            return self.indicator_value();
        }
        self.evict_all(bt.timestamp);
        let quote_vol = bt.price * bt.quantity;
        self.block_events.push_back((bt.timestamp, quote_vol));
        self.recompute();
        self.indicator_value()
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
    }

    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl AggTradeConsumer for BlockTradeVolumeRatio {
    fn update_agg_trade(&mut self, t: &AggTrade) -> IndicatorValue {
        self.evict_all(t.timestamp);
        let quote_vol = t.price * t.quantity;
        self.agg_events.push_back((t.timestamp, quote_vol));
        self.recompute();
        self.indicator_value()
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
    }

    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bt(price: f64, quantity: f64, ts: i64) -> BlockTrade {
        BlockTrade { block_id: "BT1".to_string(), price, quantity, is_buy: true, timestamp: ts, is_iv: false }
    }

    fn make_agg(price: f64, quantity: f64, ts: i64) -> AggTrade {
        AggTrade { aggregate_id: 1, price, quantity, first_trade_id: 1, last_trade_id: 1, is_buy: true, timestamp: ts }
    }

    #[test]
    fn ratio_equals_block_over_agg() {
        let mut ind = BlockTradeVolumeRatio::new(60_000);
        ind.update_agg_trade(&make_agg(30000.0, 1.0, 1000));  // agg vol = 30000
        ind.update_block_trade(&make_bt(30000.0, 0.5, 2000)); // block vol = 15000
        if let IndicatorValue::Single(ratio) = ind.indicator_value() {
            assert!((ratio - 0.5).abs() < 1e-9, "ratio={ratio}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn zero_agg_volume_gives_zero_ratio() {
        let mut ind = BlockTradeVolumeRatio::new(60_000);
        // No agg trades, only block
        ind.update_block_trade(&make_bt(30000.0, 1.0, 1000));
        if let IndicatorValue::Single(ratio) = ind.indicator_value() {
            assert_eq!(ratio, 0.0);
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn old_events_evicted_from_window() {
        let mut ind = BlockTradeVolumeRatio::new(60_000);
        // Events at t=0 (outside window relative to t=70000)
        ind.update_agg_trade(&make_agg(30000.0, 1.0, 0));
        ind.update_block_trade(&make_bt(30000.0, 0.5, 0));
        // Events at t=70000 (inside window)
        ind.update_agg_trade(&make_agg(30000.0, 2.0, 70_000));
        // Old events should be evicted; ratio = block(0) / agg(60000) = 0
        if let IndicatorValue::Single(ratio) = ind.indicator_value() {
            assert_eq!(ratio, 0.0, "block events should be evicted; ratio={ratio}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn iv_priced_block_trades_ignored() {
        let mut ind = BlockTradeVolumeRatio::new(60_000);
        ind.update_agg_trade(&make_agg(30000.0, 1.0, 1000));
        let mut iv_bt = make_bt(0.30, 10.0, 2000);
        iv_bt.is_iv = true;
        ind.update_block_trade(&iv_bt);
        if let IndicatorValue::Single(ratio) = ind.indicator_value() {
            assert_eq!(ratio, 0.0, "IV trades should be ignored; ratio={ratio}");
        } else {
            panic!("expected Single");
        }
    }
}
