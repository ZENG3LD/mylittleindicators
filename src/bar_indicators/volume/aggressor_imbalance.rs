//! AggressorImbalance — rolling buy-side vs sell-side tick frequency ratio.
//!
//! Counts buy ticks and sell ticks in a rolling window.
//! Output = `(buy_count - sell_count) / total_count` ∈ [-1.0, 1.0].
//!   +1.0 = all ticks are aggressor buys
//!   -1.0 = all ticks are aggressor sells
//!
//! Output: `IndicatorValue::Single(imbalance)`

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Rolling aggressor trade-count imbalance (not volume-weighted — pure frequency).
#[derive(Debug, Clone)]
pub struct AggressorImbalance {
    window: usize,
    /// Ring buffer: `true` = buy tick, `false` = sell tick.
    history: VecDeque<bool>,
    last_imbalance: f64,
}

impl AggressorImbalance {
    /// Create with `window` ticks rolling lookback.
    pub fn new(window: usize) -> Self {
        let cap = window.max(1);
        Self {
            window: cap,
            history: VecDeque::with_capacity(cap),
            last_imbalance: 0.0,
        }
    }
}

impl TickConsumer for AggressorImbalance {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        self.history.push_back(tick.is_buy);
        if self.history.len() > self.window {
            self.history.pop_front();
        }

        let total = self.history.len();
        let buy = self.history.iter().filter(|&&b| b).count();
        let sell = total - buy;

        self.last_imbalance = if total > 0 {
            (buy as f64 - sell as f64) / total as f64
        } else {
            0.0
        };

        IndicatorValue::Single(self.last_imbalance)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_imbalance)
    }

    fn reset(&mut self) {
        self.history.clear();
        self.last_imbalance = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.history.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Tick;

    fn tick(is_buy: bool) -> Tick {
        Tick::new(0, 100.0, 1.0, is_buy)
    }

    #[test]
    fn all_buys_give_plus_one() {
        let mut ind = AggressorImbalance::new(10);
        for _ in 0..10 {
            ind.update_tick(&tick(true));
        }
        assert!((ind.last_imbalance - 1.0).abs() < 1e-9);
    }

    #[test]
    fn all_sells_give_minus_one() {
        let mut ind = AggressorImbalance::new(10);
        for _ in 0..10 {
            ind.update_tick(&tick(false));
        }
        assert!((ind.last_imbalance - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn equal_buys_sells_give_zero() {
        let mut ind = AggressorImbalance::new(4);
        ind.update_tick(&tick(true));
        ind.update_tick(&tick(false));
        ind.update_tick(&tick(true));
        ind.update_tick(&tick(false));
        assert!(ind.last_imbalance.abs() < 1e-9);
    }

    #[test]
    fn rolling_window_evicts_old_ticks() {
        let mut ind = AggressorImbalance::new(2);
        ind.update_tick(&tick(false));
        ind.update_tick(&tick(false));
        // Now push 2 buys — they fill the window
        ind.update_tick(&tick(true));
        ind.update_tick(&tick(true));
        assert!((ind.last_imbalance - 1.0).abs() < 1e-9);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = AggressorImbalance::new(5);
        for _ in 0..5 {
            ind.update_tick(&tick(true));
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::Single(0.0));
    }
}
