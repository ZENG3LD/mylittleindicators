//! Queue Imbalance — bid vs ask size ratio at the top of book.
//!
//! Primary path: `update_orderbook(&OrderBook)` — real queue imbalance:
//!   QI = bid_size / (bid_size + ask_size)
//!   Range [0, 1]: >0.5 = bid-heavy (buy pressure), <0.5 = ask-heavy (sell pressure).
//!   Centred to [-1, 1] as output: 2*QI - 1.
//!
//! `update_bar(o,h,l,c,v)` — no-op (returns current value).
//! OHLCV bars carry no queue data; synthetic price-position approximations are
//! meaningless as queue imbalance.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;

/// Number of top-of-book levels used for the imbalance calculation.
const DEFAULT_LEVELS: usize = 5;

/// Queue Imbalance indicator.
#[derive(Debug, Clone)]
pub struct QueueImbalance {
    value: f64,
    levels: usize,
}

impl Default for QueueImbalance {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueImbalance {
    pub fn new() -> Self {
        Self { value: 0.0, levels: DEFAULT_LEVELS }
    }

    pub fn with_levels(levels: usize) -> Self {
        Self { value: 0.0, levels: levels.max(1) }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// No-op: OHLCV bars carry no queue data. Returns current value unchanged.
    #[inline]
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> f64 {
        // OHLCV path: no queue information available, value unchanged.
        self.value
    }
}

impl OrderBookConsumer for QueueImbalance {
    /// Real queue imbalance: bid_depth / (bid_depth + ask_depth) centred to [-1, 1].
    /// Uses top `levels` levels of the orderbook.
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        let bid = book.bid_depth(self.levels);
        let ask = book.ask_depth(self.levels);
        let total = bid + ask;
        self.value = if total > 0.0 {
            2.0 * (bid / total) - 1.0
        } else {
            0.0
        };
        IndicatorValue::Single(self.value)
    }

    fn value(&self) -> IndicatorValue { self.value() }
    fn reset(&mut self) { self.reset() }
    fn is_ready(&self) -> bool { self.is_ready() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBook;

    #[test]
    fn test_queue_imbalance_creation() {
        let ind = QueueImbalance::new();
        assert!(ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_update_bar_noop() {
        let mut ind = QueueImbalance::new();
        let before = ind.value().main();
        ind.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        assert_eq!(ind.value().main(), before);
    }

    #[test]
    fn test_balanced_book() {
        let mut ind = QueueImbalance::new();
        let book = OrderBook::from_tuples(
            &[(100.0, 10.0), (99.0, 10.0)],
            &[(101.0, 10.0), (102.0, 10.0)],
            1000,
        );
        let val = ind.update_orderbook(&book);
        assert!((val.main()).abs() < 1e-9, "balanced book should give 0");
    }

    #[test]
    fn test_bid_heavy_book() {
        let mut ind = QueueImbalance::new();
        let book = OrderBook::from_tuples(
            &[(100.0, 100.0)],
            &[(101.0, 1.0)],
            1000,
        );
        let val = ind.update_orderbook(&book);
        assert!(val.main() > 0.0, "bid-heavy book should give positive value");
    }

    #[test]
    fn test_ask_heavy_book() {
        let mut ind = QueueImbalance::new();
        let book = OrderBook::from_tuples(
            &[(100.0, 1.0)],
            &[(101.0, 100.0)],
            1000,
        );
        let val = ind.update_orderbook(&book);
        assert!(val.main() < 0.0, "ask-heavy book should give negative value");
    }

    #[test]
    fn test_queue_imbalance_reset() {
        let mut ind = QueueImbalance::new();
        let book = OrderBook::from_tuples(
            &[(100.0, 100.0)],
            &[(101.0, 1.0)],
            1000,
        );
        ind.update_orderbook(&book);
        ind.reset();
        assert_eq!(ind.value().main(), 0.0);
    }
}
