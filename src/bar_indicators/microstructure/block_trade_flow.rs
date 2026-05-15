//! BlockTradeFlow — rolling cumulative net flow from block trades.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::BlockTradeConsumer;
use crate::core::types::BlockTrade;

/// Rolling net flow from block trades within a time window.
///
/// net_flow = Σ(buy_qty) − Σ(sell_qty) for events within `window_ms` milliseconds.
///
/// Output: `Single(net_flow)`. Returns 0.0 until at least one event.
#[derive(Clone)]
pub struct BlockTradeFlow {
    /// Circular buffer: (timestamp_ms, quantity, is_buy)
    events: VecDeque<(i64, f64, bool)>,
    window_ms: i64,
    last_net_flow: f64,
}

impl BlockTradeFlow {
    /// Create a new indicator.
    ///
    /// - `window_ms`: rolling time window in milliseconds (clamped to at least 1).
    pub fn new(window_ms: i64) -> Self {
        Self {
            events: VecDeque::new(),
            window_ms: window_ms.max(1),
            last_net_flow: 0.0,
        }
    }

    fn compute_net_flow(events: &VecDeque<(i64, f64, bool)>) -> f64 {
        events.iter().fold(0.0, |acc, &(_, qty, is_buy)| {
            if is_buy { acc + qty } else { acc - qty }
        })
    }
}

impl Default for BlockTradeFlow {
    fn default() -> Self {
        Self::new(60_000) // 1 minute
    }
}

impl BlockTradeConsumer for BlockTradeFlow {
    fn update_block_trade(&mut self, bt: &BlockTrade) -> IndicatorValue {
        let cutoff = bt.timestamp - self.window_ms;
        while self.events.front().map_or(false, |&(ts, _, _)| ts < cutoff) {
            self.events.pop_front();
        }
        self.events.push_back((bt.timestamp, bt.quantity, bt.is_buy));
        self.last_net_flow = Self::compute_net_flow(&self.events);
        IndicatorValue::Single(self.last_net_flow)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_net_flow)
    }

    fn reset(&mut self) {
        self.events.clear();
        self.last_net_flow = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bt(timestamp: i64, quantity: f64, is_buy: bool) -> BlockTrade {
        BlockTrade {
            block_id: "test".to_string(),
            price: 100.0,
            quantity,
            is_buy,
            timestamp,
            is_iv: false,
        }
    }

    #[test]
    fn net_flow_buy_dominant() {
        let mut ind = BlockTradeFlow::new(60_000);
        ind.update_block_trade(&make_bt(1000, 10.0, true));
        ind.update_block_trade(&make_bt(2000, 3.0, false));
        if let IndicatorValue::Single(nf) = ind.value() {
            assert!((nf - 7.0).abs() < 1e-9, "net_flow should be 7.0, got {nf}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn expired_events_excluded() {
        let mut ind = BlockTradeFlow::new(60_000);
        // event at t=0 is before cutoff when t=70_000
        ind.update_block_trade(&make_bt(0, 100.0, true));
        ind.update_block_trade(&make_bt(70_000, 5.0, false));
        // now only the sell at t=70_000 is in window (cutoff = 70_000 - 60_000 = 10_000, so t=0 dropped)
        if let IndicatorValue::Single(nf) = ind.value() {
            assert!((nf - (-5.0)).abs() < 1e-9, "old event should be dropped, got {nf}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BlockTradeFlow::new(60_000);
        ind.update_block_trade(&make_bt(1000, 10.0, true));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
