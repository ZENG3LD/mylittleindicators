//! RelativeStrengthCross — relative strength of primary vs secondary symbol.
//!
//! Consumer: `TickConsumer` (primary via `update_tick`).
//! Secondary: `update_secondary_price(price, timestamp)`.
//!
//! Formula: RS = (last_A / first_A) / (last_B / first_B) - 1
//! where first_A / first_B are the first prices seen in the rolling window.
//!
//! Output: `Single(relative_strength)` — positive = A outperforms, negative = B outperforms.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Relative strength cross-asset indicator.
///
/// Primary prices arrive via `TickConsumer::update_tick` (or `update_bar`).
/// Secondary prices arrive via `update_secondary_price(price, timestamp)`.
#[derive(Clone)]
pub struct RelativeStrengthCross {
    window: usize,
    primary_prices: VecDeque<f64>,
    secondary_prices: VecDeque<f64>,
    last_rs: f64,
}

impl RelativeStrengthCross {
    /// Create a new indicator.
    ///
    /// - `window` — rolling window size for price history (default 50).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            primary_prices: VecDeque::with_capacity(window.max(2)),
            secondary_prices: VecDeque::with_capacity(window.max(2)),
            last_rs: 0.0,
        }
    }

    /// Update secondary symbol price.
    pub fn update_secondary_price(&mut self, price: f64, _timestamp: i64) -> IndicatorValue {
        if self.secondary_prices.len() >= self.window {
            self.secondary_prices.pop_front();
        }
        self.secondary_prices.push_back(price);
        self.recompute();
        self.indicator_value()
    }

    fn push_primary(&mut self, price: f64) {
        if self.primary_prices.len() >= self.window {
            self.primary_prices.pop_front();
        }
        self.primary_prices.push_back(price);
        self.recompute();
    }

    fn recompute(&mut self) {
        let n_a = self.primary_prices.len();
        let n_b = self.secondary_prices.len();
        if n_a < 2 || n_b < 2 {
            self.last_rs = 0.0;
            return;
        }
        let first_a = *self.primary_prices.front().unwrap();
        let last_a = *self.primary_prices.back().unwrap();
        let first_b = *self.secondary_prices.front().unwrap();
        let last_b = *self.secondary_prices.back().unwrap();

        if first_a.abs() < 1e-12 || first_b.abs() < 1e-12 {
            self.last_rs = 0.0;
            return;
        }
        let perf_a = last_a / first_a;
        let perf_b = last_b / first_b;
        if perf_b.abs() < 1e-12 {
            self.last_rs = 0.0;
        } else {
            self.last_rs = perf_a / perf_b - 1.0;
        }
    }

    /// Passthrough for bar pipeline — uses close as primary.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> IndicatorValue {
        self.push_primary(c);
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_rs)
    }

    /// True when both streams have at least 2 samples.
    pub fn indicator_is_ready(&self) -> bool {
        self.primary_prices.len() >= 2 && self.secondary_prices.len() >= 2
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.primary_prices.clear();
        self.secondary_prices.clear();
        self.last_rs = 0.0;
    }
}

impl Default for RelativeStrengthCross {
    fn default() -> Self {
        Self::new(50)
    }
}

impl TickConsumer for RelativeStrengthCross {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        self.push_primary(tick.price);
        self.indicator_value()
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
    }

    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tick(price: f64) -> Tick {
        Tick::new(0, price, 1.0, true)
    }

    #[test]
    fn positive_rs_when_a_outperforms() {
        // A: 100 → 120 (+20%), B: 100 → 105 (+5%)
        let mut ind = RelativeStrengthCross::new(10);
        ind.update_tick(&make_tick(100.0));
        ind.update_secondary_price(100.0, 0);
        ind.update_tick(&make_tick(120.0));
        ind.update_secondary_price(105.0, 0);
        if let IndicatorValue::Single(rs) = ind.indicator_value() {
            assert!(rs > 0.0, "A outperforms B, rs={rs}");
            // (120/100) / (105/100) - 1 = 1.2/1.05 - 1 ≈ 0.143
            assert!((rs - 0.143).abs() < 0.01, "rs={rs}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn negative_rs_when_b_outperforms() {
        // A: 100 → 105 (+5%), B: 100 → 120 (+20%)
        let mut ind = RelativeStrengthCross::new(10);
        ind.update_tick(&make_tick(100.0));
        ind.update_secondary_price(100.0, 0);
        ind.update_tick(&make_tick(105.0));
        ind.update_secondary_price(120.0, 0);
        if let IndicatorValue::Single(rs) = ind.indicator_value() {
            assert!(rs < 0.0, "B outperforms A, rs={rs}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn zero_rs_for_equal_performance() {
        let mut ind = RelativeStrengthCross::new(10);
        ind.update_tick(&make_tick(100.0));
        ind.update_secondary_price(100.0, 0);
        ind.update_tick(&make_tick(110.0));
        ind.update_secondary_price(110.0, 0);
        if let IndicatorValue::Single(rs) = ind.indicator_value() {
            assert!(rs.abs() < 1e-9, "equal performance, rs={rs}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = RelativeStrengthCross::default();
        ind.update_tick(&make_tick(100.0));
        ind.update_secondary_price(100.0, 0);
        ind.update_tick(&make_tick(110.0));
        ind.update_secondary_price(105.0, 0);
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        assert_eq!(ind.indicator_value(), IndicatorValue::Single(0.0));
    }
}
