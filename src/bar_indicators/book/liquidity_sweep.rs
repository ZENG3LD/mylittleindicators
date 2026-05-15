//! LiquiditySweep — detects large orders consuming multiple price levels.
//!
//! Tracks best bid/ask movement between snapshots.
//! A "buy sweep" occurs when best_ask moves up (asks were eaten).
//! A "sell sweep" occurs when best_bid moves down (bids were eaten).
//!
//! Output: `IndicatorValue::Double(direction, magnitude)` where
//! - `direction` = +1.0 (buy sweep), -1.0 (sell sweep), 0.0 (no sweep)
//! - `magnitude` = price distance swept (always >= 0)

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;

/// Detects liquidity sweep events from consecutive orderbook snapshots.
#[derive(Clone, Debug)]
pub struct LiquiditySweep {
    prev_best_bid: f64,
    prev_best_ask: f64,
    has_prev: bool,
    last_sweep_direction: i8,   // +1 = buy sweep, -1 = sell sweep, 0 = none
    last_sweep_magnitude: f64,  // price distance swept (>= 0)
}

impl LiquiditySweep {
    /// Create a new LiquiditySweep detector.
    pub fn new() -> Self {
        Self {
            prev_best_bid: 0.0,
            prev_best_ask: 0.0,
            has_prev: false,
            last_sweep_direction: 0,
            last_sweep_magnitude: 0.0,
        }
    }
}

impl Default for LiquiditySweep {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderBookConsumer for LiquiditySweep {
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        let (curr_bid, curr_ask) = match (book.best_bid(), book.best_ask()) {
            (Some(b), Some(a)) => (b.price, a.price),
            _ => return self.value(),
        };

        if !self.has_prev {
            self.prev_best_bid = curr_bid;
            self.prev_best_ask = curr_ask;
            self.has_prev = true;
            return self.value();
        }

        let mut direction = 0i8;
        let mut magnitude = 0.0f64;

        // Buy sweep: best_ask moved up (asks were eaten)
        if curr_ask > self.prev_best_ask {
            direction = 1;
            magnitude = curr_ask - self.prev_best_ask;
        }
        // Sell sweep: best_bid moved down (bids were eaten)
        else if curr_bid < self.prev_best_bid {
            direction = -1;
            magnitude = self.prev_best_bid - curr_bid;
        }

        self.last_sweep_direction = direction;
        self.last_sweep_magnitude = magnitude;
        self.prev_best_bid = curr_bid;
        self.prev_best_ask = curr_ask;

        IndicatorValue::Double(direction as f64, magnitude)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.last_sweep_direction as f64, self.last_sweep_magnitude)
    }

    fn reset(&mut self) {
        self.has_prev = false;
        self.last_sweep_direction = 0;
        self.last_sweep_magnitude = 0.0;
        self.prev_best_bid = 0.0;
        self.prev_best_ask = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.has_prev
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
        let ls = LiquiditySweep::new();
        assert!(!ls.is_ready());
        assert_eq!(ls.value(), IndicatorValue::Double(0.0, 0.0));
    }

    #[test]
    fn first_update_initializes_and_no_sweep() {
        let mut ls = LiquiditySweep::new();
        let book = make_book(&[(100.0, 10.0)], &[(101.0, 10.0)]);
        let v = ls.update_orderbook(&book);
        assert!(ls.is_ready());
        // First update sets baseline, direction stays 0
        assert_eq!(v, IndicatorValue::Double(0.0, 0.0));
    }

    #[test]
    fn buy_sweep_detected() {
        let mut ls = LiquiditySweep::new();
        // First snapshot: bid=95, ask=100
        let book1 = make_book(&[(95.0, 10.0)], &[(100.0, 10.0)]);
        ls.update_orderbook(&book1);
        // Second snapshot: ask moved up to 105 (buy sweep)
        let book2 = make_book(&[(95.0, 10.0)], &[(105.0, 10.0)]);
        let v = ls.update_orderbook(&book2);
        assert_eq!(v.main(), 1.0);
        if let IndicatorValue::Double(dir, mag) = v {
            assert!((dir - 1.0).abs() < 1e-10);
            assert!((mag - 5.0).abs() < 1e-10);
        } else {
            panic!("expected Double");
        }
    }

    #[test]
    fn sell_sweep_detected() {
        let mut ls = LiquiditySweep::new();
        // First snapshot: bid=95, ask=100
        let book1 = make_book(&[(95.0, 10.0)], &[(100.0, 10.0)]);
        ls.update_orderbook(&book1);
        // Second snapshot: bid moved down to 90 (sell sweep)
        let book2 = make_book(&[(90.0, 10.0)], &[(100.0, 10.0)]);
        let v = ls.update_orderbook(&book2);
        if let IndicatorValue::Double(dir, mag) = v {
            assert!((dir - (-1.0)).abs() < 1e-10);
            assert!((mag - 5.0).abs() < 1e-10);
        } else {
            panic!("expected Double");
        }
    }

    #[test]
    fn no_sweep_when_stable() {
        let mut ls = LiquiditySweep::new();
        let book = make_book(&[(95.0, 10.0)], &[(100.0, 10.0)]);
        ls.update_orderbook(&book);
        // Same book again
        let v = ls.update_orderbook(&book);
        assert_eq!(v, IndicatorValue::Double(0.0, 0.0));
    }

    #[test]
    fn reset_clears_state() {
        let mut ls = LiquiditySweep::new();
        let book = make_book(&[(95.0, 10.0)], &[(100.0, 10.0)]);
        ls.update_orderbook(&book);
        assert!(ls.is_ready());
        ls.reset();
        assert!(!ls.is_ready());
        assert_eq!(ls.value(), IndicatorValue::Double(0.0, 0.0));
    }
}
