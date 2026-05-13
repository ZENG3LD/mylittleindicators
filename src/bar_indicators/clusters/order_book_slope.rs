//! Order Book Slope — price impact curve steepness from L2 levels.
//!
//! Primary path: `update_orderbook(&OrderBook)` — fits a linear regression of
//! cumulative size (x) vs price distance from mid (y) for each side. The slope
//! (price / size) quantifies how quickly price moves per unit of depth consumed.
//! Higher slope = thinner book = larger price impact.
//!
//! Output: average of bid slope and ask slope (both positive, larger = thinner).
//!
//! `update_bar(o,h,l,c,v)` — no-op (returns current value).
//! OHLCV range/ln(volume) was a synthetic proxy; it is not retained.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;

/// Default number of levels used for slope estimation.
const DEFAULT_LEVELS: usize = 10;

/// Order Book Slope indicator.
#[derive(Debug, Clone)]
pub struct OrderBookSlope {
    value: f64,
    levels: usize,
}

impl Default for OrderBookSlope {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderBookSlope {
    pub fn new() -> Self {
        Self { value: 0.0, levels: DEFAULT_LEVELS }
    }

    pub fn with_levels(levels: usize) -> Self {
        Self { value: 0.0, levels: levels.max(2) }
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

    /// Linear regression slope: cumulative_size (x) → price_distance_from_mid (y).
    /// Returns slope ≥ 0. Larger = steeper book = less depth per tick.
    fn side_slope(mid: f64, levels: &[crate::core::types::OrderBookLevel], n_levels: usize) -> f64 {
        let n = levels.len().min(n_levels);
        if n < 2 {
            return 0.0;
        }

        let mut cumulative = 0.0;
        let mut xs = Vec::with_capacity(n);
        let mut ys = Vec::with_capacity(n);

        for level in levels.iter().take(n) {
            cumulative += level.size;
            xs.push(cumulative);
            ys.push((level.price - mid).abs());
        }

        // OLS: slope = (n * Σxy - Σx * Σy) / (n * Σx² - (Σx)²)
        let n_f = n as f64;
        let sum_x: f64 = xs.iter().sum();
        let sum_y: f64 = ys.iter().sum();
        let sum_xy: f64 = xs.iter().zip(ys.iter()).map(|(x, y)| x * y).sum();
        let sum_x2: f64 = xs.iter().map(|x| x * x).sum();

        let denom = n_f * sum_x2 - sum_x * sum_x;
        if denom.abs() < 1e-12 {
            return 0.0;
        }
        ((n_f * sum_xy - sum_x * sum_y) / denom).max(0.0)
    }

    /// No-op: OHLCV bars carry no book-slope data. Returns current value unchanged.
    #[inline]
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> f64 {
        // OHLCV path: no orderbook data available, value unchanged.
        self.value
    }
}

impl OrderBookConsumer for OrderBookSlope {
    /// Real book slope from linear regression over N levels on each side.
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        let mid = match book.mid_price() {
            Some(m) => m,
            None => return IndicatorValue::Single(self.value),
        };

        let bid_slope = Self::side_slope(mid, &book.bids, self.levels);
        let ask_slope = Self::side_slope(mid, &book.asks, self.levels);

        self.value = (bid_slope + ask_slope) / 2.0;
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
    fn test_order_book_slope_creation() {
        let ind = OrderBookSlope::new();
        assert!(ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_update_bar_noop() {
        let mut ind = OrderBookSlope::new();
        ind.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_order_book_slope_nonzero_with_book() {
        let mut ind = OrderBookSlope::new();
        let book = OrderBook::from_tuples(
            &[(100.0, 10.0), (99.0, 10.0), (98.0, 10.0)],
            &[(101.0, 10.0), (102.0, 10.0), (103.0, 10.0)],
            1000,
        );
        let val = ind.update_orderbook(&book);
        assert!(val.main() >= 0.0);
        assert!(val.main().is_finite());
    }

    #[test]
    fn test_thinner_book_steeper_slope() {
        let mut ind_thin = OrderBookSlope::new();
        let mut ind_thick = OrderBookSlope::new();

        // Thin book: small sizes, prices spread wide
        let thin = OrderBook::from_tuples(
            &[(100.0, 1.0), (95.0, 1.0), (90.0, 1.0)],
            &[(101.0, 1.0), (106.0, 1.0), (111.0, 1.0)],
            1000,
        );
        // Thick book: large sizes, prices close together
        let thick = OrderBook::from_tuples(
            &[(100.0, 100.0), (99.9, 100.0), (99.8, 100.0)],
            &[(100.1, 100.0), (100.2, 100.0), (100.3, 100.0)],
            1000,
        );

        let val_thin = ind_thin.update_orderbook(&thin).main();
        let val_thick = ind_thick.update_orderbook(&thick).main();
        assert!(val_thin > val_thick, "thin book (few large gaps) should have steeper slope");
    }

    #[test]
    fn test_order_book_slope_reset() {
        let mut ind = OrderBookSlope::new();
        let book = OrderBook::from_tuples(
            &[(100.0, 10.0)],
            &[(101.0, 10.0)],
            1000,
        );
        ind.update_orderbook(&book);
        ind.reset();
        assert_eq!(ind.value().main(), 0.0);
    }
}
