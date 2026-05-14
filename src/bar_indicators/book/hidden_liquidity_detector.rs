//! HiddenLiquidityDetector — detects hidden liquidity by comparing trade size
//! against visible order book size at the traded price level.
//!
//! Algorithm (trade-vs-book mismatch):
//! - Tick on price=X with size=Q
//! - Look up visible size at price=X in book (within `price_bucket` tolerance)
//! - If Q > visible_size → hidden = Q - visible_size
//!
//! Output: `Triple(side, last_hidden_vol, cumulative_hidden_vol)`
//!   - side:                +1.0 = buy aggressor hit hidden ask, -1.0 = sell hit hidden bid, 0.0 = no hidden
//!   - last_hidden_vol:     hidden volume on the most recent tick
//!   - cumulative_hidden:   sum of hidden volume over rolling window

use std::collections::VecDeque;

use crate::bar_indicators::hybrid_tick_book_consumer::HybridTickBookConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::types::{OrderBook, Tick};

/// Detects hidden liquidity (iceberg orders) via trade-vs-visible-book mismatch.
#[derive(Debug, Clone)]
pub struct HiddenLiquidityDetector {
    /// Price bucket size for level matching. Two prices are in the same bucket
    /// when `floor(|p1 - p2| / price_bucket) < 1`.
    price_bucket: f64,
    /// Number of most-recent ticks to include in cumulative window.
    rolling_window: usize,
    /// Ring buffer of hidden volume per recent tick.
    hidden_history: VecDeque<f64>,
    last_hidden_vol: f64,
    /// +1 = buy aggressor hit hidden ask, -1 = sell aggressor hit hidden bid, 0 = none
    last_hidden_side: i8,
    cumulative_hidden: f64,
}

impl HiddenLiquidityDetector {
    /// Create with the given price bucket tolerance and rolling window size.
    pub fn new(price_bucket: f64, window: usize) -> Self {
        let w = window.max(1);
        Self {
            price_bucket: price_bucket.max(1e-12),
            rolling_window: w,
            hidden_history: VecDeque::with_capacity(w),
            last_hidden_vol: 0.0,
            last_hidden_side: 0,
            cumulative_hidden: 0.0,
        }
    }
}

impl HybridTickBookConsumer for HiddenLiquidityDetector {
    fn update_tick_with_book(&mut self, tick: &Tick, book: &OrderBook) -> IndicatorValue {
        // Buys aggress the ask side; sells aggress the bid side.
        let target_levels = if tick.is_buy { &book.asks } else { &book.bids };

        let visible_size: f64 = target_levels
            .iter()
            .filter(|l| {
                (l.price - tick.price).abs() / self.price_bucket < 1.0
            })
            .map(|l| l.size)
            .sum();

        let hidden = (tick.size - visible_size).max(0.0);

        self.hidden_history.push_back(hidden);
        if self.hidden_history.len() > self.rolling_window {
            self.hidden_history.pop_front();
        }

        self.last_hidden_vol = hidden;
        self.last_hidden_side = if hidden > 0.0 {
            if tick.is_buy { 1 } else { -1 }
        } else {
            0
        };
        self.cumulative_hidden = self.hidden_history.iter().sum();

        IndicatorValue::Triple(
            self.last_hidden_side as f64,
            self.last_hidden_vol,
            self.cumulative_hidden,
        )
    }

    fn update_book_only(&mut self, _book: &OrderBook) {
        // No-op — state updates only on trades.
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(
            self.last_hidden_side as f64,
            self.last_hidden_vol,
            self.cumulative_hidden,
        )
    }

    fn reset(&mut self) {
        self.hidden_history.clear();
        self.last_hidden_vol = 0.0;
        self.last_hidden_side = 0;
        self.cumulative_hidden = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.hidden_history.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{OrderBook, Tick};

    fn tick(price: f64, size: f64, is_buy: bool) -> Tick {
        Tick::new(0, price, size, is_buy)
    }

    fn book_with_ask(price: f64, size: f64) -> OrderBook {
        OrderBook::from_tuples(
            &[(price - 1.0, 10.0)],   // bid
            &[(price, size)],          // ask at exact price
            0,
        )
    }

    fn book_with_bid(price: f64, size: f64) -> OrderBook {
        OrderBook::from_tuples(
            &[(price, size)],          // bid at exact price
            &[(price + 1.0, 10.0)],   // ask
            0,
        )
    }

    #[test]
    fn no_hidden_when_trade_fits_visible_ask() {
        let mut det = HiddenLiquidityDetector::new(1.0, 10);
        let book = book_with_ask(100.0, 20.0); // visible ask = 20
        let v = det.update_tick_with_book(&tick(100.0, 15.0, true), &book);
        // 15 <= 20 → no hidden
        match v {
            IndicatorValue::Triple(side, hidden, _cum) => {
                assert_eq!(side, 0.0);
                assert_eq!(hidden, 0.0);
            }
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn detects_hidden_ask_liquidity_on_buy() {
        let mut det = HiddenLiquidityDetector::new(1.0, 10);
        let book = book_with_ask(100.0, 5.0); // visible ask = 5
        let v = det.update_tick_with_book(&tick(100.0, 20.0, true), &book);
        // 20 > 5 → hidden = 15, side = +1
        match v {
            IndicatorValue::Triple(side, hidden, _cum) => {
                assert!((side - 1.0).abs() < 1e-9);
                assert!((hidden - 15.0).abs() < 1e-9);
            }
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn detects_hidden_bid_liquidity_on_sell() {
        let mut det = HiddenLiquidityDetector::new(1.0, 10);
        let book = book_with_bid(100.0, 3.0); // visible bid = 3
        let v = det.update_tick_with_book(&tick(100.0, 10.0, false), &book);
        // 10 > 3 → hidden = 7, side = -1
        match v {
            IndicatorValue::Triple(side, hidden, _cum) => {
                assert!((side - (-1.0)).abs() < 1e-9);
                assert!((hidden - 7.0).abs() < 1e-9);
            }
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn cumulative_accumulates_over_window() {
        let mut det = HiddenLiquidityDetector::new(1.0, 3);
        let book = book_with_ask(100.0, 1.0);
        // Each tick: size=6, visible=1 → hidden=5
        for _ in 0..3 {
            det.update_tick_with_book(&tick(100.0, 6.0, true), &book);
        }
        match det.value() {
            IndicatorValue::Triple(_, _, cum) => {
                assert!((cum - 15.0).abs() < 1e-9);
            }
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn window_evicts_old_ticks() {
        let mut det = HiddenLiquidityDetector::new(1.0, 2);
        let book_hidden = book_with_ask(100.0, 1.0);
        let book_no_hidden = book_with_ask(100.0, 100.0);
        // 2 ticks with hidden=5 each
        det.update_tick_with_book(&tick(100.0, 6.0, true), &book_hidden);
        det.update_tick_with_book(&tick(100.0, 6.0, true), &book_hidden);
        // Now 2 more ticks with no hidden → evicts the old ones
        det.update_tick_with_book(&tick(100.0, 1.0, true), &book_no_hidden);
        det.update_tick_with_book(&tick(100.0, 1.0, true), &book_no_hidden);
        match det.value() {
            IndicatorValue::Triple(_, _, cum) => {
                assert!((cum - 0.0).abs() < 1e-9, "cum should be 0 after window eviction: {}", cum);
            }
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut det = HiddenLiquidityDetector::new(1.0, 5);
        let book = book_with_ask(100.0, 1.0);
        det.update_tick_with_book(&tick(100.0, 10.0, true), &book);
        assert!(det.is_ready());
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }

    #[test]
    fn not_ready_until_first_tick() {
        let det = HiddenLiquidityDetector::new(1.0, 5);
        assert!(!det.is_ready());
    }
}
