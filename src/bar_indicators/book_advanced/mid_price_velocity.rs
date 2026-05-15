//! MidPriceVelocity — rate of change of mid price over a rolling window.
//!
//! Uses `OrderBook::timestamp` (milliseconds) when available for time-based velocity
//! (units: price per second). Falls back to count-based velocity (price per update)
//! when two consecutive timestamps are identical.
//!
//! velocity = (latest_mid - oldest_mid) / time_span
//!
//! where time_span is in seconds (ms / 1000.0) if timestamps differ, or in updates otherwise.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;

/// Rolling mid-price velocity.
#[derive(Clone, Debug)]
pub struct MidPriceVelocity {
    window: usize,
    /// Circular buffer of (mid_price, timestamp_ms) pairs.
    history: VecDeque<(f64, i64)>,
    last_velocity: f64,
}

impl MidPriceVelocity {
    /// Create with rolling window size (number of orderbook updates).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            history: VecDeque::new(),
            last_velocity: 0.0,
        }
    }

    fn compute_velocity(oldest: (f64, i64), latest: (f64, i64)) -> f64 {
        let price_delta = latest.0 - oldest.0;
        let time_delta_ms = latest.1 - oldest.1;
        if time_delta_ms > 0 {
            // time-based: price per second
            price_delta / (time_delta_ms as f64 / 1000.0)
        } else {
            // count-based: price per update (time_delta_ms == 0 means same/no timestamp)
            price_delta
        }
    }
}

impl Default for MidPriceVelocity {
    fn default() -> Self {
        Self::new(10)
    }
}

impl OrderBookConsumer for MidPriceVelocity {
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        let mid = match book.mid_price() {
            Some(m) => m,
            None => return IndicatorValue::Single(self.last_velocity),
        };

        self.history.push_back((mid, book.timestamp));
        if self.history.len() > self.window {
            self.history.pop_front();
        }

        if self.history.len() < 2 {
            return IndicatorValue::Single(0.0);
        }

        let oldest = *self.history.front().expect("len >= 2 checked above");
        let latest = *self.history.back().expect("len >= 2 checked above");
        self.last_velocity = Self::compute_velocity(oldest, latest);
        IndicatorValue::Single(self.last_velocity)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_velocity)
    }

    fn reset(&mut self) {
        self.history.clear();
        self.last_velocity = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBook;

    fn make_book(bid: f64, ask: f64, ts_ms: i64) -> OrderBook {
        OrderBook::from_tuples(&[(bid, 1.0)], &[(ask, 1.0)], ts_ms)
    }

    #[test]
    fn count_based_velocity_zero_timestamps() {
        // All timestamps = 0 → count-based: velocity = price_delta / 1 (latest - oldest)
        let mut ind = MidPriceVelocity::new(3);
        // mid prices: 100, 101, 102
        ind.update_orderbook(&make_book(99.5, 100.5, 0));
        ind.update_orderbook(&make_book(100.5, 101.5, 0));
        let val = ind.update_orderbook(&make_book(101.5, 102.5, 0));
        // oldest_mid=100.0, latest_mid=102.0, delta=2.0, count-based → velocity=2.0
        assert!((val.main() - 2.0).abs() < 1e-10);
        assert!(ind.is_ready());
    }

    #[test]
    fn time_based_velocity_with_timestamps() {
        let mut ind = MidPriceVelocity::new(2);
        // mid prices: 100.0 at t=0ms, 110.0 at t=1000ms → velocity=10.0 price/sec
        ind.update_orderbook(&make_book(99.5, 100.5, 0));
        let val = ind.update_orderbook(&make_book(109.5, 110.5, 1000));
        assert!((val.main() - 10.0).abs() < 1e-10);
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut ind = MidPriceVelocity::new(5);
        for i in 0..4 {
            ind.update_orderbook(&make_book(100.0 + i as f64, 101.0 + i as f64, i as i64 * 100));
        }
        assert!(!ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = MidPriceVelocity::new(3);
        for i in 0..3 {
            ind.update_orderbook(&make_book(100.0 + i as f64, 101.0 + i as f64, i as i64 * 500));
        }
        assert!(ind.is_ready());
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
}
