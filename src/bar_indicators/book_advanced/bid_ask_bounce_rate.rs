//! BidAskBounceRate — rolling rate of best bid/ask changes in the order book.
//!
//! Counts "bounces" (changes in best bid or best ask) over a rolling history window
//! and divides by the time span in seconds. If all timestamps are zero, divides
//! by the count of snapshots instead (tick-based rate).
//!
//! Output: `Single(bounce_rate)`.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;

/// Rolling rate of best bid / best ask changes.
///
/// A bounce is counted whenever the best bid **or** best ask differs
/// from the previous snapshot's best bid / best ask.
///
/// `rate = bounces / time_span_seconds` when timestamps are non-zero,
/// `rate = bounces / (n - 1)` when all timestamps are zero.
#[derive(Clone, Debug)]
pub struct BidAskBounceRate {
    window: usize,
    history: VecDeque<(i64, f64, f64)>, // (timestamp_ms, best_bid, best_ask)
    last_rate: f64,
}

impl BidAskBounceRate {
    /// Create a new indicator. `window` is clamped to at least 2.
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            history: VecDeque::new(),
            last_rate: 0.0,
        }
    }

    fn compute_rate(&self) -> f64 {
        let n = self.history.len();
        if n < 2 {
            return 0.0;
        }
        let mut bounces = 0usize;
        for i in 1..n {
            let (_, prev_bid, prev_ask) = self.history[i - 1];
            let (_, cur_bid, cur_ask) = self.history[i];
            if (cur_bid - prev_bid).abs() > f64::EPSILON || (cur_ask - prev_ask).abs() > f64::EPSILON {
                bounces += 1;
            }
        }
        let first_ts = self.history[0].0;
        let last_ts = self.history[n - 1].0;
        let time_span = (last_ts - first_ts) as f64 / 1000.0; // convert ms → seconds
        if time_span > 1e-9 {
            bounces as f64 / time_span
        } else {
            // tick-based fallback when timestamps are zero or identical
            bounces as f64 / (n - 1) as f64
        }
    }
}

impl Default for BidAskBounceRate {
    fn default() -> Self {
        Self::new(20)
    }
}

impl OrderBookConsumer for BidAskBounceRate {
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        let best_bid = book.bids.first().map(|l| l.price).unwrap_or(0.0);
        let best_ask = book.asks.first().map(|l| l.price).unwrap_or(0.0);
        self.history.push_back((book.timestamp, best_bid, best_ask));
        while self.history.len() > self.window {
            self.history.pop_front();
        }
        self.last_rate = self.compute_rate();
        IndicatorValue::Single(self.last_rate)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_rate)
    }

    fn reset(&mut self) {
        self.history.clear();
        self.last_rate = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBook;

    fn make_book(best_bid: f64, best_ask: f64, ts: i64) -> OrderBook {
        let bids = [(best_bid, 100.0)];
        let asks = [(best_ask, 100.0)];
        let mut book = OrderBook::from_tuples(&bids, &asks, ts);
        book.timestamp = ts;
        book
    }

    #[test]
    fn all_same_prices_zero_rate() {
        let mut ind = BidAskBounceRate::new(5);
        for i in 0..5 {
            ind.update_orderbook(&make_book(100.0, 101.0, i as i64 * 1000));
        }
        // No bounces → rate = 0
        if let IndicatorValue::Single(v) = ind.value() {
            assert!((v).abs() < 1e-9, "expected 0.0 rate, got {v}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn alternating_best_bid_gives_positive_rate() {
        let mut ind = BidAskBounceRate::new(4);
        // t=0ms bid=100, t=1000ms bid=101, t=2000ms bid=100 → 2 bounces / 2s = 1.0
        ind.update_orderbook(&make_book(100.0, 101.0, 0));
        ind.update_orderbook(&make_book(101.0, 101.0, 1000));
        ind.update_orderbook(&make_book(100.0, 101.0, 2000));
        if let IndicatorValue::Single(v) = ind.value() {
            assert!(v > 0.0, "rate should be positive, got {v}");
            assert!((v - 1.0).abs() < 1e-9, "expected rate=1.0, got {v}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn zero_timestamps_tick_based_fallback() {
        let mut ind = BidAskBounceRate::new(5);
        // All timestamps 0, 3 bounces in 4 transitions → rate = 3/4
        let prices = [(100.0, 101.0), (101.0, 102.0), (100.0, 101.0), (101.0, 102.0), (100.0, 101.0)];
        for (bid, ask) in prices {
            ind.update_orderbook(&make_book(bid, ask, 0));
        }
        if let IndicatorValue::Single(v) = ind.value() {
            assert!(v > 0.0, "tick-based rate should be positive, got {v}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn not_ready_with_one_sample() {
        let mut ind = BidAskBounceRate::new(5);
        ind.update_orderbook(&make_book(100.0, 101.0, 0));
        assert!(!ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BidAskBounceRate::new(4);
        ind.update_orderbook(&make_book(100.0, 101.0, 0));
        ind.update_orderbook(&make_book(101.0, 102.0, 1000));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
}
