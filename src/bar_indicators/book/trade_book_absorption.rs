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
///
/// Absorption fires when the trade exceeds `ratio` × visible top-of-book size.
/// On deep books (e.g. BTC perps with hundreds of contracts on the top level),
/// a single trade rarely consumes the whole visible level, so a ratio (default
/// 0.5 = 50%) catches meaningful liquidity absorption events instead of only
/// the rare "trade > full top" case.
#[derive(Debug, Clone)]
pub struct TradeBookAbsorption {
    rolling_window: usize,
    /// Fraction of visible top size a trade must exceed to count as absorption.
    ratio: f64,
    /// Ring buffer of (absorbed_volume, side) per recent tick.
    events: VecDeque<(f64, i8)>,
    last_absorbed: f64,
    last_side: i8,
}

impl TradeBookAbsorption {
    /// Create with given rolling window size (number of ticks) and default ratio (0.5).
    pub fn new(window: usize) -> Self {
        Self::with_ratio(window, 0.5)
    }

    /// Create with explicit ratio threshold (0.0..=1.0+; >1.0 means trade must
    /// exceed visible top fully, which mirrors the original strict semantics).
    pub fn with_ratio(window: usize, ratio: f64) -> Self {
        let w = window.max(1);
        Self {
            rolling_window: w,
            ratio: ratio.max(0.0),
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

        // Absorption: tick executes at best level price AND consumes >= ratio of
        // visible size — no price movement. On deep books the strict "tick.size >
        // visible_size" path almost never fires; ratio (default 0.5) catches
        // meaningful liquidity events.
        let price_at_level = (tick.price - level_price).abs() < 1e-9;
        let threshold = visible_size * self.ratio;
        let absorbed = if price_at_level && visible_size > 0.0 && tick.size > threshold {
            (tick.size - threshold).max(0.0)
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
    fn no_absorption_when_tick_below_ratio() {
        // ratio = 0.5, visible = 20 → threshold = 10; tick = 8 → no absorption.
        let mut det = TradeBookAbsorption::new(10);
        let book = book_ask(100.0, 20.0);
        let v = det.update_tick_with_book(&tick(100.0, 8.0, true), &book);
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
        // visible top = 5, ratio = 0.5 → threshold = 2.5, tick = 20 → absorbed = 17.5
        let mut det = TradeBookAbsorption::new(10);
        let book = book_ask(100.0, 5.0);
        let v = det.update_tick_with_book(&tick(100.0, 20.0, true), &book);
        match v {
            IndicatorValue::Triple(side, absorbed, _) => {
                assert!((side - 1.0).abs() < 1e-9, "expected +1 side");
                assert!((absorbed - 17.5).abs() < 1e-9, "absorbed should be 17.5");
            }
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn absorption_detected_on_sell_side() {
        // visible = 3, ratio = 0.5 → threshold = 1.5, tick = 12 → absorbed = 10.5
        let mut det = TradeBookAbsorption::new(10);
        let book = book_bid(100.0, 3.0);
        let v = det.update_tick_with_book(&tick(100.0, 12.0, false), &book);
        match v {
            IndicatorValue::Triple(side, absorbed, _) => {
                assert!((side - (-1.0)).abs() < 1e-9, "expected -1 side");
                assert!((absorbed - 10.5).abs() < 1e-9, "absorbed should be 10.5");
            }
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn strict_ratio_matches_legacy_semantics() {
        // ratio = 1.0 mirrors the old "tick.size > visible_size" check.
        let mut det = TradeBookAbsorption::with_ratio(10, 1.0);
        let book = book_ask(100.0, 5.0);
        let v = det.update_tick_with_book(&tick(100.0, 20.0, true), &book);
        match v {
            IndicatorValue::Triple(side, absorbed, _) => {
                assert!((side - 1.0).abs() < 1e-9);
                assert!((absorbed - 15.0).abs() < 1e-9, "ratio 1.0 → absorbed = tick - visible");
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
        // visible = 1, ratio = 0.5 → threshold = 0.5, tick = 6 → absorbed = 5.5
        let mut det = TradeBookAbsorption::new(3);
        let book = book_ask(100.0, 1.0);
        for _ in 0..3 {
            det.update_tick_with_book(&tick(100.0, 6.0, true), &book);
        }
        match det.value() {
            IndicatorValue::Triple(_, _, cum) => {
                assert!((cum - 16.5).abs() < 1e-9, "cum should be 3 × 5.5 = 16.5");
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
