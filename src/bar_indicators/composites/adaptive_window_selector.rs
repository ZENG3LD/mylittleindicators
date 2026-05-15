//! AdaptiveWindowSelector — meta-indicator that selects optimal window based on volatility.
//!
//! Consumer: `TickConsumer` (for volatility tracking via price std).
//!
//! Logic:
//! - Maintains rolling price history in `short_window` and `long_window` sizes.
//! - Computes rolling std in short window.
//! - If std > `volatility_threshold` → returns `short_window` (react faster in volatile regimes).
//! - If std <= threshold → returns `long_window` (smoother signal in calm regimes).
//!
//! Output: `Single(recommended_window_size)` as f64.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Volatility-adaptive window size selector.
///
/// Implements `TickConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Clone)]
pub struct AdaptiveWindowSelector {
    short_window: usize,
    long_window: usize,
    volatility_threshold: f64,
    prices: VecDeque<f64>,
    last_window: f64,
}

impl AdaptiveWindowSelector {
    /// Create a new indicator.
    ///
    /// - `short_window`          — window size returned in high-vol regime (default 10).
    /// - `long_window`           — window size returned in low-vol regime (default 100).
    /// - `volatility_threshold`  — rolling std threshold in price units (default 10.0).
    pub fn new(short_window: usize, long_window: usize, volatility_threshold: f64) -> Self {
        let cap = long_window.max(2);
        Self {
            short_window: short_window.max(2),
            long_window: long_window.max(short_window).max(2),
            volatility_threshold,
            prices: VecDeque::with_capacity(cap),
            last_window: long_window as f64,
        }
    }

    fn rolling_std(&self, n: usize) -> f64 {
        let count = self.prices.len().min(n);
        if count < 2 {
            return 0.0;
        }
        let slice_start = self.prices.len().saturating_sub(count);
        let iter = self.prices.iter().skip(slice_start);
        let mean: f64 = iter.clone().sum::<f64>() / count as f64;
        let variance: f64 = iter.map(|p| (p - mean).powi(2)).sum::<f64>() / (count - 1) as f64;
        variance.sqrt()
    }

    fn recompute(&mut self) {
        let std = self.rolling_std(self.short_window);
        self.last_window = if std > self.volatility_threshold {
            self.short_window as f64
        } else {
            self.long_window as f64
        };
    }

    /// Passthrough for bar pipeline — returns current value.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_window)
    }

    /// True when short_window prices have been received.
    pub fn indicator_is_ready(&self) -> bool {
        self.prices.len() >= self.short_window
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.prices.clear();
        self.last_window = self.long_window as f64;
    }
}

impl Default for AdaptiveWindowSelector {
    fn default() -> Self {
        Self::new(10, 100, 10.0)
    }
}

impl TickConsumer for AdaptiveWindowSelector {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        if self.prices.len() >= self.long_window {
            self.prices.pop_front();
        }
        self.prices.push_back(tick.price);
        self.recompute();
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

    fn tick(price: f64) -> Tick {
        Tick::new(0, price, 1.0, true)
    }

    #[test]
    fn high_vol_returns_short_window() {
        let mut ind = AdaptiveWindowSelector::new(5, 20, 10.0);
        // Feed very volatile prices
        let prices = [100.0, 150.0, 80.0, 200.0, 60.0, 180.0];
        for p in prices {
            ind.update_tick(&tick(p));
        }
        if let IndicatorValue::Single(w) = ind.indicator_value() {
            assert_eq!(w as usize, 5, "expected short window 5, got {w}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn low_vol_returns_long_window() {
        let mut ind = AdaptiveWindowSelector::new(5, 20, 100.0); // very high threshold
        for i in 0..20 {
            ind.update_tick(&tick(100.0 + i as f64 * 0.01)); // tiny moves
        }
        if let IndicatorValue::Single(w) = ind.indicator_value() {
            assert_eq!(w as usize, 20, "expected long window 20, got {w}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_restores_long_window() {
        let mut ind = AdaptiveWindowSelector::new(5, 20, 1.0);
        for i in 0..10 {
            ind.update_tick(&tick(100.0 + i as f64 * 50.0));
        }
        ind.indicator_reset();
        if let IndicatorValue::Single(w) = ind.indicator_value() {
            assert_eq!(w as usize, 20);
        } else {
            panic!("expected Single");
        }
        assert!(!ind.indicator_is_ready());
    }
}
