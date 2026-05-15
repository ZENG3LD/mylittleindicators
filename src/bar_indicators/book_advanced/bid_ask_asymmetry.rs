//! BidAskAsymmetry — normalized depth imbalance between top-N bid and ask levels.
//!
//! asymmetry = (bid_depth - ask_depth) / (bid_depth + ask_depth) ∈ [-1, 1]
//!
//! Positive → more volume on bid side (demand pressure).
//! Negative → more volume on ask side (supply pressure).

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;

/// Normalized bid/ask depth asymmetry over top-N levels.
#[derive(Clone, Debug)]
pub struct BidAskAsymmetry {
    top_n: usize,
    last_asymmetry: f64,
}

impl BidAskAsymmetry {
    /// Create with the number of price levels to aggregate on each side.
    pub fn new(top_n: usize) -> Self {
        Self {
            top_n: top_n.max(1),
            last_asymmetry: 0.0,
        }
    }
}

impl Default for BidAskAsymmetry {
    fn default() -> Self {
        Self::new(5)
    }
}

impl OrderBookConsumer for BidAskAsymmetry {
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        let bid_depth: f64 = book.bids.iter().take(self.top_n).map(|l| l.size).sum();
        let ask_depth: f64 = book.asks.iter().take(self.top_n).map(|l| l.size).sum();
        let total = bid_depth + ask_depth;
        self.last_asymmetry = if total > 0.0 {
            (bid_depth - ask_depth) / total
        } else {
            0.0
        };
        IndicatorValue::Single(self.last_asymmetry)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_asymmetry)
    }

    fn reset(&mut self) {
        self.last_asymmetry = 0.0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBook;

    fn make_book(bids: &[(f64, f64)], asks: &[(f64, f64)]) -> OrderBook {
        OrderBook::from_tuples(bids, asks, 0)
    }

    #[test]
    fn bid_heavy_gives_positive() {
        let mut ind = BidAskAsymmetry::new(5);
        // bid_depth=200, ask_depth=100 → asymmetry = (200-100)/(200+100) = 100/300 ≈ 0.333
        let bids = [(100.0, 200.0)];
        let asks = [(101.0, 100.0)];
        let val = ind.update_orderbook(&make_book(&bids, &asks));
        let expected = (200.0 - 100.0) / (200.0 + 100.0);
        assert!((val.main() - expected).abs() < 1e-10);
        assert!(val.main() > 0.0);
    }

    #[test]
    fn ask_heavy_gives_negative() {
        let mut ind = BidAskAsymmetry::new(5);
        let bids = [(100.0, 50.0)];
        let asks = [(101.0, 150.0)];
        let val = ind.update_orderbook(&make_book(&bids, &asks));
        assert!(val.main() < 0.0);
    }

    #[test]
    fn balanced_gives_zero() {
        let mut ind = BidAskAsymmetry::new(5);
        let bids = [(100.0, 100.0)];
        let asks = [(101.0, 100.0)];
        let val = ind.update_orderbook(&make_book(&bids, &asks));
        assert!((val.main()).abs() < 1e-10);
    }

    #[test]
    fn empty_book_gives_zero() {
        let mut ind = BidAskAsymmetry::new(5);
        let val = ind.update_orderbook(&make_book(&[], &[]));
        assert_eq!(val.main(), 0.0);
    }

    #[test]
    fn top_n_limits_levels() {
        let mut ind = BidAskAsymmetry::new(2);
        // 3 bid levels: 100+50+200 = 350 total, but top_n=2 → 100+50=150
        let bids = [(103.0, 100.0), (102.0, 50.0), (101.0, 200.0)];
        let asks = [(104.0, 150.0)];
        let val = ind.update_orderbook(&make_book(&bids, &asks));
        let expected = (150.0 - 150.0) / (150.0 + 150.0);
        assert!((val.main() - expected).abs() < 1e-10);
    }

    #[test]
    fn is_ready_from_start() {
        let ind = BidAskAsymmetry::new(5);
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_value() {
        let mut ind = BidAskAsymmetry::new(5);
        let bids = [(100.0, 200.0)];
        let asks = [(101.0, 100.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        assert!(ind.value().main().abs() > 0.0);
        ind.reset();
        assert_eq!(ind.value().main(), 0.0);
    }
}
