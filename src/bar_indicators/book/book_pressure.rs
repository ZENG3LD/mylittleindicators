//! BookPressure — slope momentum of bid/ask depth changes.
//!
//! Tracks rolling N snapshots of bid_depth and ask_depth.
//! `pressure = bid_slope - ask_slope`
//!
//! Positive = bid pressure growing faster (bullish).
//! Negative = ask pressure growing faster (bearish).

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;

/// Slope momentum of bid vs ask depth changes over a rolling window.
#[derive(Clone, Debug)]
pub struct BookPressure {
    window: usize,
    levels_to_aggregate: usize,
    bid_history: VecDeque<f64>,
    ask_history: VecDeque<f64>,
    last_pressure: f64,
}

impl BookPressure {
    /// Create with given window size and number of price levels to aggregate.
    ///
    /// - `window`: number of snapshots for slope computation (min 2)
    /// - `levels`: number of L2 depth levels to sum
    pub fn new(window: usize, levels: usize) -> Self {
        Self {
            window: window.max(2),
            levels_to_aggregate: levels.max(1),
            bid_history: VecDeque::new(),
            ask_history: VecDeque::new(),
            last_pressure: 0.0,
        }
    }
}

impl Default for BookPressure {
    fn default() -> Self {
        Self::new(10, 5)
    }
}

impl OrderBookConsumer for BookPressure {
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        let bid = book.bid_depth(self.levels_to_aggregate);
        let ask = book.ask_depth(self.levels_to_aggregate);

        self.bid_history.push_back(bid);
        self.ask_history.push_back(ask);
        if self.bid_history.len() > self.window {
            self.bid_history.pop_front();
            self.ask_history.pop_front();
        }

        if self.bid_history.len() < 2 {
            return IndicatorValue::Single(0.0);
        }

        let n = self.bid_history.len() as f64;
        let bid_slope = (self.bid_history.back().copied().unwrap_or(0.0)
            - self.bid_history.front().copied().unwrap_or(0.0))
            / n;
        let ask_slope = (self.ask_history.back().copied().unwrap_or(0.0)
            - self.ask_history.front().copied().unwrap_or(0.0))
            / n;

        self.last_pressure = bid_slope - ask_slope;
        IndicatorValue::Single(self.last_pressure)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_pressure)
    }

    fn reset(&mut self) {
        self.bid_history.clear();
        self.ask_history.clear();
        self.last_pressure = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.bid_history.len() >= self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{OrderBook, OrderBookLevel};

    fn make_book(bid_size: f64, ask_size: f64) -> OrderBook {
        OrderBook {
            bids: vec![OrderBookLevel::new(100.0, bid_size)],
            asks: vec![OrderBookLevel::new(101.0, ask_size)],
            timestamp: 0,
        }
    }

    #[test]
    fn new_not_ready() {
        let bp = BookPressure::new(5, 1);
        assert!(!bp.is_ready());
        assert_eq!(bp.value().main(), 0.0);
    }

    #[test]
    fn bid_pressure_growing_gives_positive() {
        let mut bp = BookPressure::new(5, 1);
        // bid_depth grows 100 → 200, ask stays 100
        for i in 0..5 {
            let bid = 100.0 + i as f64 * 25.0; // 100, 125, 150, 175, 200
            bp.update_orderbook(&make_book(bid, 100.0));
        }
        assert!(bp.is_ready());
        assert!(bp.value().main() > 0.0);
    }

    #[test]
    fn ask_pressure_growing_gives_negative() {
        let mut bp = BookPressure::new(5, 1);
        // ask_depth grows 100 → 200, bid stays 100
        for i in 0..5 {
            let ask = 100.0 + i as f64 * 25.0;
            bp.update_orderbook(&make_book(100.0, ask));
        }
        assert!(bp.is_ready());
        assert!(bp.value().main() < 0.0);
    }

    #[test]
    fn stable_book_gives_zero_pressure() {
        let mut bp = BookPressure::new(5, 1);
        for _ in 0..6 {
            bp.update_orderbook(&make_book(100.0, 100.0));
        }
        assert!(bp.is_ready());
        assert!((bp.value().main()).abs() < 1e-10);
    }

    #[test]
    fn reset_clears_state() {
        let mut bp = BookPressure::new(3, 1);
        for _ in 0..4 {
            bp.update_orderbook(&make_book(50.0, 50.0));
        }
        assert!(bp.is_ready());
        bp.reset();
        assert!(!bp.is_ready());
        assert_eq!(bp.value().main(), 0.0);
    }
}
