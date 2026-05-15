//! BlockTradeImpact — rolling block trade event rate (events per minute).

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::BlockTradeConsumer;
use crate::core::types::BlockTrade;

/// Rolling rate of block trade events in events per minute.
///
/// rate = (count of events in window) / window_minutes
///
/// Output: `Single(events_per_min)`. Returns 0.0 until at least one event.
#[derive(Clone)]
pub struct BlockTradeImpact {
    events: VecDeque<i64>,
    window_ms: i64,
    last_rate: f64,
}

impl BlockTradeImpact {
    /// Create a new indicator.
    ///
    /// - `window_ms`: rolling time window in milliseconds (clamped to at least 1).
    pub fn new(window_ms: i64) -> Self {
        Self {
            events: VecDeque::new(),
            window_ms: window_ms.max(1),
            last_rate: 0.0,
        }
    }

    fn compute_rate(count: usize, window_ms: i64) -> f64 {
        let window_minutes = window_ms as f64 / 60_000.0;
        count as f64 / window_minutes
    }
}

impl Default for BlockTradeImpact {
    fn default() -> Self {
        Self::new(60_000) // 1 minute
    }
}

impl BlockTradeConsumer for BlockTradeImpact {
    fn update_block_trade(&mut self, bt: &BlockTrade) -> IndicatorValue {
        let cutoff = bt.timestamp - self.window_ms;
        while self.events.front().map_or(false, |&ts| ts < cutoff) {
            self.events.pop_front();
        }
        self.events.push_back(bt.timestamp);
        self.last_rate = Self::compute_rate(self.events.len(), self.window_ms);
        IndicatorValue::Single(self.last_rate)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_rate)
    }

    fn reset(&mut self) {
        self.events.clear();
        self.last_rate = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bt(timestamp: i64) -> BlockTrade {
        BlockTrade {
            block_id: "test".to_string(),
            price: 100.0,
            quantity: 1.0,
            is_buy: true,
            timestamp,
            is_iv: false,
        }
    }

    #[test]
    fn rate_equals_events_per_minute() {
        // window = 60_000ms = 1 min, 6 events → 6 per min
        let mut ind = BlockTradeImpact::new(60_000);
        for i in 0..6 {
            ind.update_block_trade(&make_bt(i * 10_000));
        }
        if let IndicatorValue::Single(r) = ind.value() {
            assert!((r - 6.0).abs() < 1e-9, "expected 6.0 events/min, got {r}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn expired_events_excluded() {
        let mut ind = BlockTradeImpact::new(60_000);
        ind.update_block_trade(&make_bt(0));
        // advance by more than window
        ind.update_block_trade(&make_bt(70_000));
        // only t=70_000 is in window
        if let IndicatorValue::Single(r) = ind.value() {
            let expected = 1.0 / 1.0; // 1 event / 1 min
            assert!((r - expected).abs() < 1e-9, "got {r}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BlockTradeImpact::new(60_000);
        ind.update_block_trade(&make_bt(1000));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
