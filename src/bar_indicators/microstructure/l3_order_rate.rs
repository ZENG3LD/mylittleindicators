//! L3OrderRate — rolling rate of L3 orderbook events per second.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::OrderbookL3Consumer;
use crate::core::types::OrderbookL3Event;

/// Rolling rate of L3 orderbook events in events per second.
///
/// rate = (count of events in window) / window_seconds
///
/// Output: `Single(events_per_sec)`. Returns 0.0 until at least one event.
#[derive(Clone)]
pub struct L3OrderRate {
    events: VecDeque<i64>,
    window_ms: i64,
    last_rate: f64,
}

impl L3OrderRate {
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
        let window_seconds = window_ms as f64 / 1_000.0;
        count as f64 / window_seconds
    }
}

impl Default for L3OrderRate {
    fn default() -> Self {
        Self::new(10_000) // 10 seconds
    }
}

impl OrderbookL3Consumer for L3OrderRate {
    fn update_orderbook_l3(&mut self, l3: &OrderbookL3Event) -> IndicatorValue {
        let cutoff = l3.timestamp - self.window_ms;
        while self.events.front().map_or(false, |&ts| ts < cutoff) {
            self.events.pop_front();
        }
        self.events.push_back(l3.timestamp);
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
    use crate::core::types::{L3Action, OrderBookSide};

    fn make_l3(timestamp: i64) -> OrderbookL3Event {
        OrderbookL3Event {
            side: OrderBookSide::Bid,
            order_id: "test".to_string(),
            price: 100.0,
            quantity: 1.0,
            action: L3Action::Add,
            timestamp,
        }
    }

    #[test]
    fn rate_per_second() {
        // window = 10_000ms = 10s, 10 events → 1.0 per sec
        let mut ind = L3OrderRate::new(10_000);
        for i in 0..10 {
            ind.update_orderbook_l3(&make_l3(i * 1_000));
        }
        if let IndicatorValue::Single(r) = ind.value() {
            assert!((r - 1.0).abs() < 1e-9, "expected 1.0 events/sec, got {r}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn expired_events_excluded() {
        let mut ind = L3OrderRate::new(10_000);
        ind.update_orderbook_l3(&make_l3(0));
        ind.update_orderbook_l3(&make_l3(15_000)); // t=0 drops out
        if let IndicatorValue::Single(r) = ind.value() {
            let expected = 1.0 / 10.0; // 1 event / 10 seconds
            assert!((r - expected).abs() < 1e-9, "got {r}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = L3OrderRate::new(10_000);
        ind.update_orderbook_l3(&make_l3(1000));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
