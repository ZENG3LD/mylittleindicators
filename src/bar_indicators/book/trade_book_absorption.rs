//! TradeBookAbsorption — detects absorption using real order book state.
//!
//! Similar to `AbsorptionDetector` (tick-only) but uses the actual visible
//! top-of-book size to determine whether a trade was absorbed at the best level.
//!
//! Definition of absorption:
//! - The tick executes at exactly the best ask (buy) or best bid (sell) price
//! - The trade size exceeds the visible top-of-book size at that level
//! - Yet price did not move (the level price matched tick price exactly)
//! → Someone had size beyond what was visible (absorption / iceberg).
//!
//! Output: `Triple(side, last_absorbed_vol, cumulative_absorbed_vol)`
//!   - side:                +1.0 = buy absorption at ask, -1.0 = sell absorption at bid, 0.0 = none
//!   - last_absorbed_vol:   absorbed volume on most recent tick
//!   - cumulative:          sum over rolling window

use std::collections::VecDeque;

use crate::bar_indicators::hybrid_tick_book_consumer::HybridTickBookConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::types::{OrderBook, Tick};

/// Detects absorption using real top-of-book size vs actual trade size.
#[derive(Debug, Clone)]
pub struct TradeBookAbsorption {
    rolling_window: usize,
    /// Ring buffer of (absorbed_volume, side) per recent tick.
    events: VecDeque<(f64, i8)>,
    last_absorbed: f64,
    last_side: i8,
}

impl TradeBookAbsorption {
    /// Create with given rolling window size (number of ticks).
    pub fn new(window: usize) -> Self {
        let w = window.max(1);
        Self {
            rolling_window: w,
            events: VecDeque::with_capacity(w),
            last_absorbed: 0.0,
            last_side: 0,
        }
    }
}

impl HybridTickBookConsumer for TradeBookAbsorption {
    fn update_tick_with_book(&mut self, tick: &Tick, book: &OrderBook) -> IndicatorValue {
        let (target_level, side) = if tick.is_buy {
            (book.best_ask(), 1i8)
        } else {
            (book.best_bid(), -1i8)
        };

        let visible_size = target_level.map(|l| l.size).unwrap_or(0.0);
        let level_price = target_level.map(|l| l.price).unwrap_or(tick.price);

        // Absorption: tick executes at best level price AND size > visible — no price movement.
        let price_at_level = (tick.price - level_price).abs() < 1e-9;
        let absorbed = if price_at_level && tick.size > visible_size {
            tick.size - visible_size
        } else {
            0.0
        };

        self.events
            .push_back((absorbed, if absorbed > 0.0 { side } else { 0 }));
        if self.events.len() > self.rolling_window {
            self.events.pop_front();
        }

        self.last_absorbed = absorbed;
        self.last_side = if absorbed > 0.0 { side } else { 0 };

        let cumulative: f64 = self.events.iter().map(|&(v, _)| v).sum();
        IndicatorValue::Triple(self.last_side as f64, self.last_absorbed, cumulative)
    }

    fn update_book_only(&mut self, _book: &OrderBook) {}

    fn value(&self) -> IndicatorValue {
        let cumulative: f64 = self.events.iter().map(|&(v, _)| v).sum();
        IndicatorValue::Triple(self.last_side as f64, self.last_absorbed, cumulative)
    }

    fn reset(&mut self) {
        self.events.clear();
        self.last_absorbed = 0.0;
        self.last_side = 0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{OrderBook, Tick};

    fn tick(price: f64, size: f64, is_buy: bool) -> Tick {
        Tick::new(0, price, size, is_buy)
    }

    fn book_ask(price: f64, size: f64) -> OrderBook {
        OrderBook::from_tuples(&[(price - 1.0, 10.0)], &[(price, size)], 0)
    }

    fn book_bid(price: f64, size: f64) -> OrderBook {
        OrderBook::from_tuples(&[(price, size)], &[(price + 1.0, 10.0)], 0)
    }

    #[test]
    fn no_absorption_when_tick_fits_visible() {
        let mut det = TradeBookAbsorption::new(10);
        let book = book_ask(100.0, 20.0);
        let v = det.update_tick_with_book(&tick(100.0, 15.0, true), &book);
        match v {
            IndicatorValue::Triple(side, absorbed, _) => {
                assert_eq!(side, 0.0);
                assert_eq!(absorbed, 0.0);
            }
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn absorption_detected_when_trade_exceeds_visible_ask() {
        let mut det = TradeBookAbsorption::new(10);
        let book = book_ask(100.0, 5.0); // visible top ask = 5
        let v = det.update_tick_with_book(&tick(100.0, 20.0, true), &book);
        match v {
            IndicatorValue::Triple(side, absorbed, _) => {
                assert!((side - 1.0).abs() < 1e-9, "expected +1 side");
                assert!((absorbed - 15.0).abs() < 1e-9, "absorbed should be 15");
            }
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn absorption_detected_on_sell_side() {
        let mut det = TradeBookAbsorption::new(10);
        let book = book_bid(100.0, 3.0);
        let v = det.update_tick_with_book(&tick(100.0, 12.0, false), &book);
        match v {
            IndicatorValue::Triple(side, absorbed, _) => {
                assert!((side - (-1.0)).abs() < 1e-9, "expected -1 side");
                assert!((absorbed - 9.0).abs() < 1e-9, "absorbed should be 9");
            }
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn no_absorption_when_tick_price_not_at_best_level() {
        let mut det = TradeBookAbsorption::new(10);
        // Best ask at 101, tick at 100 → price_at_level = false
        let book = book_ask(101.0, 2.0);
        let v = det.update_tick_with_book(&tick(100.0, 20.0, true), &book);
        match v {
            IndicatorValue::Triple(side, absorbed, _) => {
                assert_eq!(side, 0.0);
                assert_eq!(absorbed, 0.0);
            }
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn cumulative_tracks_window() {
        let mut det = TradeBookAbsorption::new(3);
        let book = book_ask(100.0, 1.0);
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
    fn reset_clears_state() {
        let mut det = TradeBookAbsorption::new(5);
        let book = book_ask(100.0, 1.0);
        det.update_tick_with_book(&tick(100.0, 10.0, true), &book);
        assert!(det.is_ready());
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }
}
