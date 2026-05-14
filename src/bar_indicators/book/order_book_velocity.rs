//! OrderBookVelocity — rate of orderbook change across snapshots.
//!
//! Compares each snapshot to the previous, counting how many price levels
//! changed (added, removed, or resized). Rolling average over N snapshots.
//!
//! High velocity = orderbook changing rapidly (active market).
//! Low velocity = orderbook stable (quiet market).
//!
//! Output: `IndicatorValue::Single(avg_changes_per_snapshot)`

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::{OrderBook, OrderBookLevel};

/// Rolling average rate of orderbook level changes per snapshot.
#[derive(Clone, Debug)]
pub struct OrderBookVelocity {
    window: usize,
    prev_bids: Vec<(f64, f64)>, // (price, size) snapshot
    prev_asks: Vec<(f64, f64)>,
    has_prev: bool,
    changes_history: VecDeque<usize>,
    last_velocity: f64, // avg changes per snapshot
}

impl OrderBookVelocity {
    /// Create with given rolling window size.
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            prev_bids: Vec::new(),
            prev_asks: Vec::new(),
            has_prev: false,
            changes_history: VecDeque::new(),
            last_velocity: 0.0,
        }
    }

    /// Count the number of changed levels between a previous snapshot and current levels.
    ///
    /// A level is "changed" if its price differs, size differs, or it disappeared/appeared.
    fn count_changes(prev: &[(f64, f64)], curr: &[OrderBookLevel]) -> usize {
        let max_len = prev.len().max(curr.len());
        let mut changes = 0usize;
        for i in 0..max_len {
            let prev_entry = prev.get(i);
            let curr_entry = curr.get(i);

            match (prev_entry, curr_entry) {
                (None, Some(_)) | (Some(_), None) => {
                    // Level added or removed
                    changes += 1;
                }
                (Some(&(pp, ps)), Some(cl)) => {
                    if !pp.is_finite() || !cl.price.is_finite() || pp != cl.price {
                        changes += 1;
                    } else if (ps - cl.size).abs() > 1e-9 {
                        changes += 1;
                    }
                }
                (None, None) => {}
            }
        }
        changes
    }
}

impl Default for OrderBookVelocity {
    fn default() -> Self {
        Self::new(10)
    }
}

impl OrderBookConsumer for OrderBookVelocity {
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        if !self.has_prev {
            self.prev_bids = book.bids.iter().map(|l| (l.price, l.size)).collect();
            self.prev_asks = book.asks.iter().map(|l| (l.price, l.size)).collect();
            self.has_prev = true;
            return self.value();
        }

        let bid_changes = Self::count_changes(&self.prev_bids, &book.bids);
        let ask_changes = Self::count_changes(&self.prev_asks, &book.asks);
        let total = bid_changes + ask_changes;

        self.changes_history.push_back(total);
        if self.changes_history.len() > self.window {
            self.changes_history.pop_front();
        }

        let sum: usize = self.changes_history.iter().sum();
        self.last_velocity = sum as f64 / self.changes_history.len() as f64;

        self.prev_bids = book.bids.iter().map(|l| (l.price, l.size)).collect();
        self.prev_asks = book.asks.iter().map(|l| (l.price, l.size)).collect();

        IndicatorValue::Single(self.last_velocity)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_velocity)
    }

    fn reset(&mut self) {
        self.prev_bids.clear();
        self.prev_asks.clear();
        self.has_prev = false;
        self.changes_history.clear();
        self.last_velocity = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.changes_history.len() >= self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{OrderBook, OrderBookLevel};

    fn make_book(bids: &[(f64, f64)], asks: &[(f64, f64)]) -> OrderBook {
        OrderBook {
            bids: bids.iter().map(|&(p, s)| OrderBookLevel::new(p, s)).collect(),
            asks: asks.iter().map(|&(p, s)| OrderBookLevel::new(p, s)).collect(),
            timestamp: 0,
        }
    }

    #[test]
    fn new_not_ready() {
        let obv = OrderBookVelocity::new(5);
        assert!(!obv.is_ready());
        assert_eq!(obv.value().main(), 0.0);
    }

    #[test]
    fn stable_book_gives_zero_velocity() {
        let mut obv = OrderBookVelocity::new(3);
        let book = make_book(&[(100.0, 10.0), (99.0, 8.0)], &[(101.0, 10.0), (102.0, 8.0)]);
        // Feed same book 4 times (1 baseline + 3 diffs = 3 changes_history entries)
        for _ in 0..4 {
            obv.update_orderbook(&book);
        }
        assert!(obv.is_ready());
        assert_eq!(obv.value().main(), 0.0);
    }

    #[test]
    fn changing_book_gives_high_velocity() {
        let mut obv = OrderBookVelocity::new(3);
        // Each update has different prices/sizes
        for i in 0..5 {
            let p = 100.0 + i as f64;
            let book = make_book(&[(p, 10.0)], &[(p + 1.0, 10.0)]);
            obv.update_orderbook(&book);
        }
        assert!(obv.is_ready());
        assert!(obv.value().main() > 0.0);
    }

    #[test]
    fn size_change_counts_as_change() {
        let mut obv = OrderBookVelocity::new(2);
        let book1 = make_book(&[(100.0, 10.0)], &[(101.0, 10.0)]);
        let book2 = make_book(&[(100.0, 20.0)], &[(101.0, 10.0)]); // bid size changed
        obv.update_orderbook(&book1); // baseline
        let v = obv.update_orderbook(&book2);
        // 1 bid change detected
        assert!(v.main() >= 1.0);
    }

    #[test]
    fn reset_clears_state() {
        let mut obv = OrderBookVelocity::new(2);
        let book = make_book(&[(100.0, 10.0)], &[(101.0, 10.0)]);
        for _ in 0..4 {
            obv.update_orderbook(&book);
        }
        assert!(obv.is_ready());
        obv.reset();
        assert!(!obv.is_ready());
        assert_eq!(obv.value().main(), 0.0);
    }
}
