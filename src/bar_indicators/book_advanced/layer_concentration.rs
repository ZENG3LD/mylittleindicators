//! LayerConcentration — Gini coefficient of depth distribution across price levels.
//!
//! High Gini → liquidity concentrated on a few levels (wall-like).
//! Low Gini → liquidity distributed evenly across levels.
//!
//! Output: `Triple(gini_bid, gini_ask, max(gini_bid, gini_ask))`

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;

/// Gini-coefficient concentration of order book depth per side.
#[derive(Clone, Debug)]
pub struct LayerConcentration {
    top_n: usize,
    last_gini_bid: f64,
    last_gini_ask: f64,
    last_max: f64,
}

impl LayerConcentration {
    /// Create with the number of price levels to sample on each side.
    pub fn new(top_n: usize) -> Self {
        Self {
            top_n: top_n.max(2),
            last_gini_bid: 0.0,
            last_gini_ask: 0.0,
            last_max: 0.0,
        }
    }

    /// Compute Gini coefficient for a slice of values (sorted ascending internally).
    ///
    /// Formula: G = Σ(2i - n - 1) * x_i / (n * Σx_i), where i is 1-based.
    fn gini(sizes: &[f64]) -> f64 {
        let n = sizes.len();
        if n < 2 {
            return 0.0;
        }
        let sum: f64 = sizes.iter().sum();
        if sum <= 0.0 {
            return 0.0;
        }
        // Sort ascending (copy into stack-allocated small vec)
        let mut sorted: Vec<f64> = sizes.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let weighted_sum: f64 = sorted
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                // i is 0-based; formula uses 1-based i
                let rank = (i + 1) as f64;
                (2.0 * rank - n as f64 - 1.0) * x
            })
            .sum();

        weighted_sum / (n as f64 * sum)
    }
}

impl Default for LayerConcentration {
    fn default() -> Self {
        Self::new(10)
    }
}

impl OrderBookConsumer for LayerConcentration {
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        let bid_sizes: Vec<f64> = book.bids.iter().take(self.top_n).map(|l| l.size).collect();
        let ask_sizes: Vec<f64> = book.asks.iter().take(self.top_n).map(|l| l.size).collect();

        self.last_gini_bid = Self::gini(&bid_sizes);
        self.last_gini_ask = Self::gini(&ask_sizes);
        self.last_max = self.last_gini_bid.max(self.last_gini_ask);

        IndicatorValue::Triple(self.last_gini_bid, self.last_gini_ask, self.last_max)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_gini_bid, self.last_gini_ask, self.last_max)
    }

    fn reset(&mut self) {
        self.last_gini_bid = 0.0;
        self.last_gini_ask = 0.0;
        self.last_max = 0.0;
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
    fn uniform_distribution_gives_low_gini() {
        let mut ind = LayerConcentration::new(4);
        // All levels equal size → Gini = 0
        let bids = [(104.0, 100.0), (103.0, 100.0), (102.0, 100.0), (101.0, 100.0)];
        let asks = [(105.0, 100.0), (106.0, 100.0), (107.0, 100.0), (108.0, 100.0)];
        let v = ind.update_orderbook(&make_book(&bids, &asks));
        let (g_bid, g_ask, _) = match v {
            IndicatorValue::Triple(a, b, c) => (a, b, c),
            _ => panic!("expected Triple"),
        };
        assert!(g_bid.abs() < 1e-10, "uniform bid gini should be ~0, got {}", g_bid);
        assert!(g_ask.abs() < 1e-10, "uniform ask gini should be ~0, got {}", g_ask);
    }

    #[test]
    fn concentrated_distribution_gives_high_gini() {
        let mut ind = LayerConcentration::new(4);
        // One level dominates: [1, 1, 1, 1000] → high Gini
        let bids = [(104.0, 1.0), (103.0, 1.0), (102.0, 1.0), (101.0, 1000.0)];
        let asks = [(105.0, 1.0), (106.0, 1.0), (107.0, 1.0), (108.0, 1000.0)];
        let v = ind.update_orderbook(&make_book(&bids, &asks));
        let (g_bid, _, _) = match v {
            IndicatorValue::Triple(a, b, c) => (a, b, c),
            _ => panic!("expected Triple"),
        };
        assert!(g_bid > 0.5, "concentrated bid gini should be high, got {}", g_bid);
    }

    #[test]
    fn gini_bounded_zero_to_one() {
        let mut ind = LayerConcentration::new(5);
        let bids = [(105.0, 5.0), (104.0, 10.0), (103.0, 2.0), (102.0, 80.0), (101.0, 3.0)];
        let asks = [(106.0, 20.0), (107.0, 1.0), (108.0, 50.0), (109.0, 5.0), (110.0, 24.0)];
        let v = ind.update_orderbook(&make_book(&bids, &asks));
        let (g_bid, g_ask, max_g) = match v {
            IndicatorValue::Triple(a, b, c) => (a, b, c),
            _ => panic!("expected Triple"),
        };
        assert!(g_bid >= 0.0 && g_bid <= 1.0);
        assert!(g_ask >= 0.0 && g_ask <= 1.0);
        assert!((max_g - g_bid.max(g_ask)).abs() < 1e-10);
    }

    #[test]
    fn reset_clears_values() {
        let mut ind = LayerConcentration::new(4);
        let bids = [(104.0, 1000.0), (103.0, 1.0), (102.0, 1.0), (101.0, 1.0)];
        let asks = [(105.0, 1.0), (106.0, 1.0), (107.0, 1.0), (108.0, 1.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        ind.reset();
        let v = ind.value();
        assert_eq!(v.main(), 0.0);
    }
}
