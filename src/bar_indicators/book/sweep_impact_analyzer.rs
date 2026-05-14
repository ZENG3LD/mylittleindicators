//! SweepImpactAnalyzer — measures how many book levels a trade consumed and
//! the resulting price slippage (sweep depth / impact).
//!
//! A "sweep" is any trade whose size exceeds the first order book level, forcing
//! the fill to cross into deeper levels. The indicator counts:
//! - `levels_swept` — how many distinct price levels were touched to fill the trade
//! - `slippage` — absolute price distance from the best level to the last level touched
//!
//! For trades that fit within the top level only, `levels_swept = 0` and `slippage = 0`.
//!
//! Output: `Triple(side, levels_swept, slippage)`
//!   - side:          +1.0 = buy sweep (ask side), -1.0 = sell sweep (bid side), 0.0 = no sweep
//!   - levels_swept:  number of levels touched (0 = no sweep, 1 = hit exactly best level, ≥2 = real sweep)
//!   - slippage:      |last_level_price - best_level_price|

use std::collections::VecDeque;

use crate::bar_indicators::hybrid_tick_book_consumer::HybridTickBookConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::types::{OrderBook, Tick};

/// Measures how many book levels a trade consumed and the resulting slippage.
#[derive(Debug, Clone)]
pub struct SweepImpactAnalyzer {
    rolling_window: usize,
    /// Ring buffer of (levels_swept, slippage, side) for recent sweeps only.
    impacts: VecDeque<(usize, f64, i8)>,
    last_levels_swept: usize,
    last_slippage: f64,
    last_side: i8,
}

impl SweepImpactAnalyzer {
    /// Create with given rolling window (number of ticks, not sweeps).
    pub fn new(window: usize) -> Self {
        let w = window.max(1);
        Self {
            rolling_window: w,
            impacts: VecDeque::with_capacity(w),
            last_levels_swept: 0,
            last_slippage: 0.0,
            last_side: 0,
        }
    }
}

impl HybridTickBookConsumer for SweepImpactAnalyzer {
    fn update_tick_with_book(&mut self, tick: &Tick, book: &OrderBook) -> IndicatorValue {
        let (levels, side, initial_price) = if tick.is_buy {
            let best = book.best_ask().map(|l| l.price).unwrap_or(tick.price);
            (&book.asks, 1i8, best)
        } else {
            let best = book.best_bid().map(|l| l.price).unwrap_or(tick.price);
            (&book.bids, -1i8, best)
        };

        let mut remaining = tick.size;
        let mut levels_swept = 0usize;
        let mut final_price = initial_price;

        for level in levels.iter() {
            if remaining <= 0.0 {
                break;
            }
            remaining -= level.size;
            levels_swept += 1;
            final_price = level.price;
        }

        let slippage = (final_price - initial_price).abs();
        let is_sweep = levels_swept > 1;

        if is_sweep {
            self.impacts.push_back((levels_swept, slippage, side));
            if self.impacts.len() > self.rolling_window {
                self.impacts.pop_front();
            }
            self.last_levels_swept = levels_swept;
            self.last_slippage = slippage;
            self.last_side = side;
        } else {
            // No sweep — still need to push a zero entry so the ring-buffer and
            // is_ready() logic work correctly (every tick contributes one slot).
            self.impacts.push_back((0, 0.0, 0));
            if self.impacts.len() > self.rolling_window {
                self.impacts.pop_front();
            }
            self.last_levels_swept = 0;
            self.last_slippage = 0.0;
            self.last_side = 0;
        }

        IndicatorValue::Triple(
            self.last_side as f64,
            self.last_levels_swept as f64,
            self.last_slippage,
        )
    }

    fn update_book_only(&mut self, _book: &OrderBook) {}

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(
            self.last_side as f64,
            self.last_levels_swept as f64,
            self.last_slippage,
        )
    }

    fn reset(&mut self) {
        self.impacts.clear();
        self.last_levels_swept = 0;
        self.last_slippage = 0.0;
        self.last_side = 0;
    }

    fn is_ready(&self) -> bool {
        !self.impacts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{OrderBook, Tick};

    fn tick(price: f64, size: f64, is_buy: bool) -> Tick {
        Tick::new(0, price, size, is_buy)
    }

    /// Build a book with three ask levels at 100, 101, 102 (size=5 each).
    fn deep_ask_book() -> OrderBook {
        OrderBook::from_tuples(
            &[(99.0, 100.0)],
            &[(100.0, 5.0), (101.0, 5.0), (102.0, 5.0)],
            0,
        )
    }

    /// Build a book with three bid levels at 100, 99, 98 (size=5 each).
    fn deep_bid_book() -> OrderBook {
        OrderBook::from_tuples(
            &[(100.0, 5.0), (99.0, 5.0), (98.0, 5.0)],
            &[(101.0, 100.0)],
            0,
        )
    }

    #[test]
    fn no_sweep_when_trade_fits_first_level() {
        let mut det = SweepImpactAnalyzer::new(10);
        let book = deep_ask_book();
        let v = det.update_tick_with_book(&tick(100.0, 4.0, true), &book);
        match v {
            IndicatorValue::Triple(side, levels, slippage) => {
                assert_eq!(side, 0.0, "no sweep → side=0");
                assert_eq!(levels, 0.0);
                assert_eq!(slippage, 0.0);
            }
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn sweep_two_levels_buy() {
        let mut det = SweepImpactAnalyzer::new(10);
        let book = deep_ask_book(); // levels: 100@5, 101@5, 102@5
        // size=8 → fills all of level 100 (5) and part of 101 → swept 2 levels
        let v = det.update_tick_with_book(&tick(100.0, 8.0, true), &book);
        match v {
            IndicatorValue::Triple(side, levels, slippage) => {
                assert!((side - 1.0).abs() < 1e-9, "buy sweep → side=+1");
                assert!((levels - 2.0).abs() < 1e-9, "2 levels swept");
                assert!((slippage - 1.0).abs() < 1e-9, "slippage = 101-100 = 1");
            }
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn sweep_three_levels_buy() {
        let mut det = SweepImpactAnalyzer::new(10);
        let book = deep_ask_book();
        // size=12 → 5+5=10, still need 2 more → touches level 102 → 3 levels
        let v = det.update_tick_with_book(&tick(100.0, 12.0, true), &book);
        match v {
            IndicatorValue::Triple(side, levels, slippage) => {
                assert!((side - 1.0).abs() < 1e-9);
                assert!((levels - 3.0).abs() < 1e-9, "3 levels swept");
                assert!((slippage - 2.0).abs() < 1e-9, "slippage = 102-100 = 2");
            }
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn sweep_sell_side() {
        let mut det = SweepImpactAnalyzer::new(10);
        let book = deep_bid_book(); // levels: 100@5, 99@5, 98@5
        let v = det.update_tick_with_book(&tick(100.0, 8.0, false), &book);
        match v {
            IndicatorValue::Triple(side, levels, slippage) => {
                assert!((side - (-1.0)).abs() < 1e-9, "sell sweep → side=-1");
                assert!((levels - 2.0).abs() < 1e-9);
                assert!((slippage - 1.0).abs() < 1e-9);
            }
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn is_ready_after_first_tick() {
        let mut det = SweepImpactAnalyzer::new(5);
        assert!(!det.is_ready());
        let book = deep_ask_book();
        det.update_tick_with_book(&tick(100.0, 1.0, true), &book);
        assert!(det.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut det = SweepImpactAnalyzer::new(5);
        let book = deep_ask_book();
        det.update_tick_with_book(&tick(100.0, 12.0, true), &book);
        assert!(det.is_ready());
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }
}
