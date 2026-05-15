//! BestLevelVolatility — rolling standard deviation of best bid size and best ask size.
//!
//! Measures instability of the top-of-book. High values indicate the best level
//! is frequently refreshed with large size swings (aggressive quoting).
//!
//! Output: `Triple(std_bid_size, std_ask_size, max(std_bid, std_ask))`

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;

/// Rolling std-dev of best bid/ask sizes.
#[derive(Clone, Debug)]
pub struct BestLevelVolatility {
    window: usize,
    bid_sizes: VecDeque<f64>,
    ask_sizes: VecDeque<f64>,
    last_std_bid: f64,
    last_std_ask: f64,
    last_max: f64,
}

impl BestLevelVolatility {
    /// Create with rolling window size.
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            bid_sizes: VecDeque::new(),
            ask_sizes: VecDeque::new(),
            last_std_bid: 0.0,
            last_std_ask: 0.0,
            last_max: 0.0,
        }
    }

    fn std_dev(buf: &VecDeque<f64>) -> f64 {
        let n = buf.len();
        if n < 2 {
            return 0.0;
        }
        let mean = buf.iter().sum::<f64>() / n as f64;
        let variance = buf.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        variance.sqrt()
    }
}

impl Default for BestLevelVolatility {
    fn default() -> Self {
        Self::new(20)
    }
}

impl OrderBookConsumer for BestLevelVolatility {
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        if let Some(b) = book.best_bid() {
            self.bid_sizes.push_back(b.size);
            if self.bid_sizes.len() > self.window {
                self.bid_sizes.pop_front();
            }
        }
        if let Some(a) = book.best_ask() {
            self.ask_sizes.push_back(a.size);
            if self.ask_sizes.len() > self.window {
                self.ask_sizes.pop_front();
            }
        }

        self.last_std_bid = Self::std_dev(&self.bid_sizes);
        self.last_std_ask = Self::std_dev(&self.ask_sizes);
        self.last_max = self.last_std_bid.max(self.last_std_ask);

        IndicatorValue::Triple(self.last_std_bid, self.last_std_ask, self.last_max)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_std_bid, self.last_std_ask, self.last_max)
    }

    fn reset(&mut self) {
        self.bid_sizes.clear();
        self.ask_sizes.clear();
        self.last_std_bid = 0.0;
        self.last_std_ask = 0.0;
        self.last_max = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.bid_sizes.len() >= self.window && self.ask_sizes.len() >= self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBook;

    fn make_book(bid_size: f64, ask_size: f64) -> OrderBook {
        OrderBook::from_tuples(&[(100.0, bid_size)], &[(101.0, ask_size)], 0)
    }

    #[test]
    fn constant_sizes_give_zero_std() {
        let mut ind = BestLevelVolatility::new(5);
        for _ in 0..5 {
            ind.update_orderbook(&make_book(50.0, 50.0));
        }
        let v = ind.value();
        let (std_bid, std_ask, max_v) = match v {
            IndicatorValue::Triple(a, b, c) => (a, b, c),
            _ => panic!("expected Triple"),
        };
        assert!(std_bid.abs() < 1e-10);
        assert!(std_ask.abs() < 1e-10);
        assert!(max_v.abs() < 1e-10);
    }

    #[test]
    fn varying_bid_gives_nonzero_std() {
        let mut ind = BestLevelVolatility::new(4);
        for size in [10.0_f64, 20.0, 30.0, 40.0] {
            ind.update_orderbook(&make_book(size, 25.0));
        }
        let v = ind.value();
        let std_bid = v.main();
        assert!(std_bid > 0.0);
    }

    #[test]
    fn max_is_larger_of_two_stds() {
        let mut ind = BestLevelVolatility::new(4);
        // bid std small (all same), ask std large
        for size in [10.0_f64, 40.0, 10.0, 40.0] {
            ind.update_orderbook(&make_book(25.0, size));
        }
        let v = ind.value();
        let (std_bid, std_ask, max_v) = match v {
            IndicatorValue::Triple(a, b, c) => (a, b, c),
            _ => panic!("expected Triple"),
        };
        assert!(std_ask > std_bid);
        assert!((max_v - std_ask).abs() < 1e-10);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BestLevelVolatility::new(3);
        for _ in 0..3 {
            ind.update_orderbook(&make_book(10.0, 20.0));
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
}
