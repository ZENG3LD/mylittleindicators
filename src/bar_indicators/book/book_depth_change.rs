//! BookDepthChange — delta of bid/ask depth between consecutive snapshots.
//!
//! Tracks how the total size at the top N bid/ask levels changed since
//! the previous snapshot. Positive bid_change = book deepening on bid side.
//!
//! Output: `IndicatorValue::Double(bid_depth_change, ask_depth_change)`

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;

/// Measures change in aggregated bid/ask depth between consecutive snapshots.
#[derive(Debug, Clone)]
pub struct BookDepthChange {
    /// Number of levels to aggregate on each side.
    levels_to_aggregate: usize,
    prev_bid_depth: f64,
    prev_ask_depth: f64,
    has_prev: bool,
    last_bid_change: f64,
    last_ask_change: f64,
}

impl BookDepthChange {
    /// Create with `levels` levels aggregated per side.
    pub fn new(levels: usize) -> Self {
        Self {
            levels_to_aggregate: levels.max(1),
            prev_bid_depth: 0.0,
            prev_ask_depth: 0.0,
            has_prev: false,
            last_bid_change: 0.0,
            last_ask_change: 0.0,
        }
    }
}

impl OrderBookConsumer for BookDepthChange {
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        let bid = book.bid_depth(self.levels_to_aggregate);
        let ask = book.ask_depth(self.levels_to_aggregate);

        if self.has_prev {
            self.last_bid_change = bid - self.prev_bid_depth;
            self.last_ask_change = ask - self.prev_ask_depth;
        }

        self.prev_bid_depth = bid;
        self.prev_ask_depth = ask;
        self.has_prev = true;

        IndicatorValue::Double(self.last_bid_change, self.last_ask_change)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.last_bid_change, self.last_ask_change)
    }

    fn reset(&mut self) {
        self.prev_bid_depth = 0.0;
        self.prev_ask_depth = 0.0;
        self.has_prev = false;
        self.last_bid_change = 0.0;
        self.last_ask_change = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.has_prev
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBook;

    fn book(bid_size: f64, ask_size: f64) -> OrderBook {
        OrderBook::from_tuples(
            &[(100.0, bid_size)],
            &[(101.0, ask_size)],
            0,
        )
    }

    #[test]
    fn first_snapshot_gives_zero_change() {
        let mut ind = BookDepthChange::new(5);
        let v = ind.update_orderbook(&book(10.0, 10.0));
        assert_eq!(v, IndicatorValue::Double(0.0, 0.0));
    }

    #[test]
    fn second_snapshot_gives_correct_delta() {
        let mut ind = BookDepthChange::new(5);
        ind.update_orderbook(&book(10.0, 10.0));
        let v = ind.update_orderbook(&book(15.0, 8.0));
        match v {
            IndicatorValue::Double(bd, ad) => {
                assert!((bd - 5.0).abs() < 1e-9, "bid change: {}", bd);
                assert!((ad - (-2.0)).abs() < 1e-9, "ask change: {}", ad);
            }
            other => panic!("expected Double, got {:?}", other),
        }
    }

    #[test]
    fn is_ready_after_first_snapshot() {
        let mut ind = BookDepthChange::new(5);
        assert!(!ind.is_ready());
        ind.update_orderbook(&book(10.0, 10.0));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BookDepthChange::new(5);
        ind.update_orderbook(&book(10.0, 10.0));
        ind.update_orderbook(&book(20.0, 5.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::Double(0.0, 0.0));
    }
}
