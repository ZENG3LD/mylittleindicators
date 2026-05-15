//! Book Imbalance — L2 orderbook bid/ask volume imbalance.
//!
//! Computes `(bid_depth - ask_depth) / (bid_depth + ask_depth)` over N levels.
//! Range: [-1, +1] where +1 = full bid pressure, -1 = full ask pressure.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;

#[derive(Clone, Debug)]
pub struct BookImbalanceRatio {
    /// Number of L2 levels to aggregate.
    levels: usize,
    last_value: f64,
    bars_seen: usize,
}

impl Default for BookImbalanceRatio {
    fn default() -> Self {
        Self::new()
    }
}

impl BookImbalanceRatio {
    /// Create with default single-level (top-of-book) aggregation.
    pub fn new() -> Self {
        Self::with_levels(1)
    }

    /// Create with explicit level depth.
    pub fn with_levels(levels: usize) -> Self {
        Self {
            levels: levels.max(1),
            last_value: 0.0,
            bars_seen: 0,
        }
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_value)
    }

    pub fn is_ready(&self) -> bool {
        self.bars_seen > 0
    }

    pub fn reset(&mut self) {
        self.last_value = 0.0;
        self.bars_seen = 0;
    }
}

impl OrderBookConsumer for BookImbalanceRatio {
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        let bid = book.bid_depth(self.levels);
        let ask = book.ask_depth(self.levels);
        let total = bid + ask;
        self.last_value = if total > 0.0 {
            (bid - ask) / total
        } else {
            0.0
        };
        self.bars_seen += 1;
        IndicatorValue::Single(self.last_value)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_value)
    }

    fn reset(&mut self) {
        self.last_value = 0.0;
        self.bars_seen = 0;
    }

    fn is_ready(&self) -> bool {
        self.bars_seen > 0
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
            ..Default::default()
        }
    }

    #[test]
    fn new_not_ready() {
        let ind = BookImbalanceRatio::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn balanced_book_gives_zero() {
        let mut imb = BookImbalanceRatio::with_levels(3);
        let book = make_book(
            &[(100.0, 10.0), (99.0, 10.0)],
            &[(101.0, 10.0), (102.0, 10.0)],
        );
        let v = imb.update_orderbook(&book);
        assert_eq!(v, IndicatorValue::Single(0.0));
        assert!(imb.is_ready());
    }

    #[test]
    fn bid_heavy_positive() {
        let mut imb = BookImbalanceRatio::with_levels(3);
        let book = make_book(&[(100.0, 30.0)], &[(101.0, 10.0)]);
        let v = imb.update_orderbook(&book);
        // (30 - 10) / (30 + 10) = 0.5
        assert!((v.main() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn ask_heavy_negative() {
        let mut imb = BookImbalanceRatio::with_levels(3);
        let book = make_book(&[(100.0, 10.0)], &[(101.0, 30.0)]);
        let v = imb.update_orderbook(&book);
        // (10 - 30) / (10 + 30) = -0.5
        assert!((v.main() - (-0.5)).abs() < 1e-10);
    }

    #[test]
    fn empty_book_gives_zero() {
        let mut imb = BookImbalanceRatio::new();
        let book = make_book(&[], &[]);
        let v = imb.update_orderbook(&book);
        assert_eq!(v.main(), 0.0);
    }

    #[test]
    fn reset_clears_state() {
        let mut imb = BookImbalanceRatio::new();
        let book = make_book(&[(100.0, 10.0)], &[(101.0, 5.0)]);
        imb.update_orderbook(&book);
        assert!(imb.is_ready());
        imb.reset();
        assert!(!imb.is_ready());
        assert_eq!(imb.value().main(), 0.0);
    }

    #[test]
    fn multi_level_aggregation() {
        let mut imb = BookImbalanceRatio::with_levels(2);
        // bids: 10 + 8 = 18, asks: 5 + 7 = 12
        let book = make_book(
            &[(100.0, 10.0), (99.0, 8.0), (98.0, 20.0)],
            &[(101.0, 5.0), (102.0, 7.0), (103.0, 20.0)],
        );
        let v = imb.update_orderbook(&book);
        // (18 - 12) / (18 + 12) = 6 / 30 = 0.2
        assert!((v.main() - 0.2).abs() < 1e-10);
    }
}
