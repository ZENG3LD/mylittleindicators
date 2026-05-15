//! PriceLevelDensity — count of price levels per unit of price range.
//!
//! Measures how tightly packed the order book levels are.
//! High density → narrow price range, thin book structure.
//! Low density → levels spread wide, coarse book.
//!
//! density_bid = count_bid / (max_bid_price - min_bid_price)
//! density_ask = count_ask / (max_ask_price - min_ask_price)
//!
//! Output: `Triple(density_bid, density_ask, avg(density_bid, density_ask))`

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;

/// Price-level density (levels per price unit) for top-N bid and ask levels.
#[derive(Clone, Debug)]
pub struct PriceLevelDensity {
    top_n: usize,
    last_density_bid: f64,
    last_density_ask: f64,
    last_avg: f64,
}

impl PriceLevelDensity {
    /// Create with the number of price levels to consider on each side.
    pub fn new(top_n: usize) -> Self {
        Self {
            top_n: top_n.max(2),
            last_density_bid: 0.0,
            last_density_ask: 0.0,
            last_avg: 0.0,
        }
    }

    fn compute_density(prices: &[f64]) -> f64 {
        let n = prices.len();
        if n < 2 {
            return 0.0;
        }
        let min = prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;
        if range <= 0.0 {
            return 0.0;
        }
        n as f64 / range
    }
}

impl Default for PriceLevelDensity {
    fn default() -> Self {
        Self::new(10)
    }
}

impl OrderBookConsumer for PriceLevelDensity {
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        let bid_prices: Vec<f64> = book.bids.iter().take(self.top_n).map(|l| l.price).collect();
        let ask_prices: Vec<f64> = book.asks.iter().take(self.top_n).map(|l| l.price).collect();

        self.last_density_bid = Self::compute_density(&bid_prices);
        self.last_density_ask = Self::compute_density(&ask_prices);
        self.last_avg = (self.last_density_bid + self.last_density_ask) / 2.0;

        IndicatorValue::Triple(self.last_density_bid, self.last_density_ask, self.last_avg)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_density_bid, self.last_density_ask, self.last_avg)
    }

    fn reset(&mut self) {
        self.last_density_bid = 0.0;
        self.last_density_ask = 0.0;
        self.last_avg = 0.0;
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
    fn narrow_spacing_gives_high_density() {
        let mut ind = PriceLevelDensity::new(3);
        // bids at 100.0, 99.9, 99.8 → range = 0.2, count = 3 → density = 15.0
        let bids = [(100.0, 10.0), (99.9, 10.0), (99.8, 10.0)];
        let asks = [(100.1, 10.0), (100.2, 10.0), (100.3, 10.0)];
        let v = ind.update_orderbook(&make_book(&bids, &asks));
        let density_bid = v.main();
        assert!((density_bid - 15.0).abs() < 1e-8, "density_bid should be 15.0, got {}", density_bid);
    }

    #[test]
    fn wide_spacing_gives_low_density() {
        let mut ind = PriceLevelDensity::new(3);
        // bids at 100.0, 90.0, 80.0 → range = 20.0, count = 3 → density = 0.15
        let bids = [(100.0, 10.0), (90.0, 10.0), (80.0, 10.0)];
        let asks = [(101.0, 10.0), (111.0, 10.0), (121.0, 10.0)];
        let v = ind.update_orderbook(&make_book(&bids, &asks));
        let density_bid = v.main();
        assert!((density_bid - 0.15).abs() < 1e-10, "density_bid should be 0.15, got {}", density_bid);
    }

    #[test]
    fn single_level_gives_zero_density() {
        let mut ind = PriceLevelDensity::new(3);
        // Only 1 bid level available → can't compute range
        let bids = [(100.0, 10.0)];
        let asks = [(101.0, 10.0)];
        let v = ind.update_orderbook(&make_book(&bids, &asks));
        // density_bid = 0 (only 1 level), density_ask = 0 (only 1 level)
        assert_eq!(v.main(), 0.0);
    }

    #[test]
    fn avg_is_mean_of_bid_ask_density() {
        let mut ind = PriceLevelDensity::new(2);
        // bids: 100.0, 99.0 → range=1.0, density=2.0
        // asks: 101.0, 103.0 → range=2.0, density=1.0
        let bids = [(100.0, 1.0), (99.0, 1.0)];
        let asks = [(101.0, 1.0), (103.0, 1.0)];
        let v = ind.update_orderbook(&make_book(&bids, &asks));
        let (d_bid, d_ask, avg) = match v {
            IndicatorValue::Triple(a, b, c) => (a, b, c),
            _ => panic!("expected Triple"),
        };
        assert!((d_bid - 2.0).abs() < 1e-10);
        assert!((d_ask - 1.0).abs() < 1e-10);
        assert!((avg - 1.5).abs() < 1e-10);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = PriceLevelDensity::new(3);
        let bids = [(100.0, 10.0), (99.0, 10.0), (98.0, 10.0)];
        let asks = [(101.0, 10.0), (102.0, 10.0), (103.0, 10.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        ind.reset();
        assert_eq!(ind.value().main(), 0.0);
    }
}
