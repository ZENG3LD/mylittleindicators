//! SpreadDistribution — rolling spread percentile rank.
//!
//! Tracks the last N spread values and computes where the current spread
//! falls within that distribution.
//!
//! Output: `IndicatorValue::Double(spread, percentile)` where
//! - `spread` = current best_ask - best_bid
//! - `percentile` = 0-100, where 100 = tightest (current spread ≤ all historical)
//!   and 0 = widest (current spread > all historical)
//!
//! Note: percentile is inverted vs raw rank — higher means tighter spread (better liquidity).

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;

/// Rolling percentile rank of bid-ask spread.
#[derive(Clone, Debug)]
pub struct SpreadDistribution {
    window: usize,
    history: VecDeque<f64>,
    last_spread: f64,
    last_percentile: f64,
}

impl SpreadDistribution {
    /// Create with given rolling window size.
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            history: VecDeque::new(),
            last_spread: 0.0,
            last_percentile: 0.0,
        }
    }
}

impl Default for SpreadDistribution {
    fn default() -> Self {
        Self::new(50)
    }
}

impl OrderBookConsumer for SpreadDistribution {
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        let spread = match book.spread() {
            Some(s) if s.is_finite() && s > 0.0 => s,
            _ => return self.value(),
        };

        self.last_spread = spread;
        self.history.push_back(spread);
        if self.history.len() > self.window {
            self.history.pop_front();
        }

        // Percentile rank: % of historical spreads that are >= current spread
        // (wider spreads = lower percentile; tighter spread = higher percentile)
        let count_ge = self.history.iter().filter(|&&s| s >= spread).count();
        self.last_percentile = (count_ge as f64 / self.history.len() as f64) * 100.0;

        IndicatorValue::Double(spread, self.last_percentile)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.last_spread, self.last_percentile)
    }

    fn reset(&mut self) {
        self.history.clear();
        self.last_spread = 0.0;
        self.last_percentile = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.history.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{OrderBook, OrderBookLevel};

    fn make_book_spread(bid: f64, ask: f64) -> OrderBook {
        OrderBook {
            bids: vec![OrderBookLevel::new(bid, 10.0)],
            asks: vec![OrderBookLevel::new(ask, 10.0)],
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn new_not_ready() {
        let sd = SpreadDistribution::new(10);
        assert!(!sd.is_ready());
        assert_eq!(sd.value(), IndicatorValue::Double(0.0, 0.0));
    }

    #[test]
    fn first_update_is_ready() {
        let mut sd = SpreadDistribution::new(10);
        sd.update_orderbook(&make_book_spread(100.0, 101.0));
        assert!(sd.is_ready());
    }

    #[test]
    fn tightest_spread_gets_100_percentile() {
        let mut sd = SpreadDistribution::new(10);
        // Fill with spread=1.0
        for _ in 0..5 {
            sd.update_orderbook(&make_book_spread(100.0, 101.0));
        }
        // 6th snapshot with tighter spread=0.5
        let v = sd.update_orderbook(&make_book_spread(100.0, 100.5));
        if let IndicatorValue::Double(spread, pct) = v {
            assert!((spread - 0.5).abs() < 1e-10);
            // 0.5 is tightest, all 6 entries >= 0.5, so percentile = 100
            assert!((pct - 100.0).abs() < 1e-6);
        } else {
            panic!("expected Double");
        }
    }

    #[test]
    fn widest_spread_gets_low_percentile() {
        let mut sd = SpreadDistribution::new(10);
        // Fill with spread=1.0
        for _ in 0..5 {
            sd.update_orderbook(&make_book_spread(100.0, 101.0));
        }
        // 6th snapshot with wider spread=2.0
        let v = sd.update_orderbook(&make_book_spread(100.0, 102.0));
        if let IndicatorValue::Double(spread, pct) = v {
            assert!((spread - 2.0).abs() < 1e-10);
            // Only the current entry (2.0) >= 2.0, so percentile = 1/6 * 100
            assert!(pct < 30.0);
        } else {
            panic!("expected Double");
        }
    }

    #[test]
    fn zero_spread_skipped() {
        let mut sd = SpreadDistribution::new(5);
        let bad_book = OrderBook { bids: vec![], asks: vec![], timestamp: 0, ..Default::default() };
        sd.update_orderbook(&bad_book);
        assert!(!sd.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut sd = SpreadDistribution::new(5);
        sd.update_orderbook(&make_book_spread(100.0, 101.0));
        assert!(sd.is_ready());
        sd.reset();
        assert!(!sd.is_ready());
        assert_eq!(sd.value(), IndicatorValue::Double(0.0, 0.0));
    }
}
