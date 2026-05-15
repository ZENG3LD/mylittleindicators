//! QuoteLifecycleTracker — rolling average lifetime of L3 orders (Add→Delete).

use std::collections::{HashMap, VecDeque};

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::orderbook_l3_consumer::OrderbookL3Consumer;
use crate::core::types::{L3Action, OrderbookL3Event};

/// Rolling average lifetime of L3 orderbook quotes.
///
/// Tracks Add→Delete pairs for each order_id. When a Delete arrives,
/// computes `lifetime_ms = delete_ts - add_ts` and adds it to a rolling
/// window. Returns the mean lifetime across the window.
///
/// Modify events are ignored — only Add/Delete pairs are tracked.
///
/// Output: `Single(avg_lifetime_ms)`.
#[derive(Clone)]
pub struct QuoteLifecycleTracker {
    window_size: usize,
    /// Maps order_id → add timestamp
    pending: HashMap<String, i64>,
    /// Rolling window of completed lifetimes (ms)
    lifetimes: VecDeque<f64>,
    last_avg_lifetime: f64,
}

impl QuoteLifecycleTracker {
    /// Create a new tracker.
    ///
    /// - `window_size`: number of completed lifetimes to average over (clamped ≥ 2).
    pub fn new(window_size: usize) -> Self {
        let window_size = window_size.max(2);
        Self {
            window_size,
            pending: HashMap::new(),
            lifetimes: VecDeque::with_capacity(window_size),
            last_avg_lifetime: 0.0,
        }
    }

    fn compute_avg(&self) -> f64 {
        if self.lifetimes.is_empty() {
            return 0.0;
        }
        self.lifetimes.iter().sum::<f64>() / self.lifetimes.len() as f64
    }
}

impl Default for QuoteLifecycleTracker {
    fn default() -> Self {
        Self::new(50)
    }
}

impl OrderbookL3Consumer for QuoteLifecycleTracker {
    fn update_orderbook_l3(&mut self, l3: &OrderbookL3Event) -> IndicatorValue {
        match l3.action {
            L3Action::Add => {
                self.pending.insert(l3.order_id.clone(), l3.timestamp);
            }
            L3Action::Delete => {
                if let Some(add_ts) = self.pending.remove(&l3.order_id) {
                    let lifetime = (l3.timestamp - add_ts).max(0) as f64;
                    self.lifetimes.push_back(lifetime);
                    while self.lifetimes.len() > self.window_size {
                        self.lifetimes.pop_front();
                    }
                    self.last_avg_lifetime = self.compute_avg();
                }
            }
            L3Action::Modify => {
                // Ignored — only Add/Delete pairs tracked
            }
        }
        IndicatorValue::Single(self.last_avg_lifetime)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_avg_lifetime)
    }

    fn reset(&mut self) {
        self.pending.clear();
        self.lifetimes.clear();
        self.last_avg_lifetime = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.lifetimes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBookSide;

    fn add_event(order_id: &str, ts: i64) -> OrderbookL3Event {
        OrderbookL3Event {
            side: OrderBookSide::Bid,
            order_id: order_id.to_string(),
            price: 100.0,
            quantity: 1.0,
            action: L3Action::Add,
            timestamp: ts,
        }
    }

    fn delete_event(order_id: &str, ts: i64) -> OrderbookL3Event {
        OrderbookL3Event {
            side: OrderBookSide::Bid,
            order_id: order_id.to_string(),
            price: 100.0,
            quantity: 0.0,
            action: L3Action::Delete,
            timestamp: ts,
        }
    }

    #[test]
    fn lifetime_computed_correctly() {
        let mut ind = QuoteLifecycleTracker::new(10);
        ind.update_orderbook_l3(&add_event("order1", 1000));
        ind.update_orderbook_l3(&delete_event("order1", 1500));
        if let IndicatorValue::Single(v) = ind.value() {
            assert!((v - 500.0).abs() < 1e-9, "lifetime = {v}, expected 500.0");
        }
    }

    #[test]
    fn rolling_average_over_multiple_orders() {
        let mut ind = QuoteLifecycleTracker::new(10);
        // order1: 200ms, order2: 400ms → avg = 300ms
        ind.update_orderbook_l3(&add_event("o1", 1000));
        ind.update_orderbook_l3(&add_event("o2", 1000));
        ind.update_orderbook_l3(&delete_event("o1", 1200));
        ind.update_orderbook_l3(&delete_event("o2", 1400));
        if let IndicatorValue::Single(v) = ind.value() {
            assert!((v - 300.0).abs() < 1e-9, "avg lifetime = {v}, expected 300.0");
        }
    }

    #[test]
    fn orphan_delete_ignored() {
        let mut ind = QuoteLifecycleTracker::new(10);
        ind.update_orderbook_l3(&delete_event("unknown", 1000));
        assert!(!ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = QuoteLifecycleTracker::new(10);
        ind.update_orderbook_l3(&add_event("o1", 0));
        ind.update_orderbook_l3(&delete_event("o1", 100));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
