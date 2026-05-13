//! Microprice — bid-ask weighted mid price.
//!
//! `microprice = (bid_size * ask_price + ask_size * bid_price) / (bid_size + ask_size)`
//!
//! Weights each side by the opposing side's quantity, producing a price
//! prediction that is more accurate than the simple mid for next-trade price.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;

#[derive(Clone, Debug)]
pub struct Microprice {
    last_value: f64,
    ready: bool,
}

impl Microprice {
    pub fn new() -> Self {
        Self { last_value: 0.0, ready: false }
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_value)
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn reset(&mut self) {
        self.last_value = 0.0;
        self.ready = false;
    }
}

impl Default for Microprice {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderBookConsumer for Microprice {
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        if let (Some(bid), Some(ask)) = (book.best_bid(), book.best_ask()) {
            let total_size = bid.size + ask.size;
            if total_size > 0.0 {
                self.last_value =
                    (bid.size * ask.price + ask.size * bid.price) / total_size;
                self.ready = true;
            }
        }
        IndicatorValue::Single(self.last_value)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_value)
    }

    fn reset(&mut self) {
        self.last_value = 0.0;
        self.ready = false;
    }

    fn is_ready(&self) -> bool {
        self.ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{OrderBook, OrderBookLevel};

    fn make_book(bid_p: f64, bid_s: f64, ask_p: f64, ask_s: f64) -> OrderBook {
        OrderBook {
            bids: vec![OrderBookLevel::new(bid_p, bid_s)],
            asks: vec![OrderBookLevel::new(ask_p, ask_s)],
            timestamp: 0,
        }
    }

    #[test]
    fn new_not_ready() {
        let mp = Microprice::new();
        assert!(!mp.is_ready());
        assert_eq!(mp.value().main(), 0.0);
    }

    #[test]
    fn equal_sizes_gives_mid() {
        let mut mp = Microprice::new();
        // bid=100 size=10, ask=102 size=10
        // microprice = (10*102 + 10*100) / 20 = 2020/20 = 101
        let book = make_book(100.0, 10.0, 102.0, 10.0);
        let v = mp.update_orderbook(&book);
        assert!((v.main() - 101.0).abs() < 1e-10);
        assert!(mp.is_ready());
    }

    #[test]
    fn bid_heavy_pulls_price_up() {
        let mut mp = Microprice::new();
        // bid=100 size=30, ask=102 size=10
        // microprice = (30*102 + 10*100) / 40 = (3060 + 1000)/40 = 4060/40 = 101.5
        let book = make_book(100.0, 30.0, 102.0, 10.0);
        let v = mp.update_orderbook(&book);
        assert!((v.main() - 101.5).abs() < 1e-10);
    }

    #[test]
    fn ask_heavy_pulls_price_down() {
        let mut mp = Microprice::new();
        // bid=100 size=10, ask=102 size=30
        // microprice = (10*102 + 30*100) / 40 = (1020 + 3000)/40 = 4020/40 = 100.5
        let book = make_book(100.0, 10.0, 102.0, 30.0);
        let v = mp.update_orderbook(&book);
        assert!((v.main() - 100.5).abs() < 1e-10);
    }

    #[test]
    fn empty_book_stays_zero() {
        let mut mp = Microprice::new();
        let book = OrderBook { bids: vec![], asks: vec![], timestamp: 0 };
        let v = mp.update_orderbook(&book);
        assert_eq!(v.main(), 0.0);
        assert!(!mp.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut mp = Microprice::new();
        let book = make_book(100.0, 10.0, 102.0, 10.0);
        mp.update_orderbook(&book);
        assert!(mp.is_ready());
        mp.reset();
        assert!(!mp.is_ready());
        assert_eq!(mp.value().main(), 0.0);
    }
}
