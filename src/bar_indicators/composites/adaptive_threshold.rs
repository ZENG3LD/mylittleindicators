//! AdaptiveThreshold — dynamic threshold based on rolling mean + N×std.
//!
//! Consumer: `TickConsumer`.
//!
//! Logic: threshold = rolling_mean(prices) + N × rolling_std(prices).
//! Uses a fixed rolling window of price observations.
//!
//! Output: `Triple(mean, std, threshold)`.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Dynamic threshold meta-indicator for z-score based detectors.
///
/// Implements `TickConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Clone)]
pub struct AdaptiveThreshold {
    window: usize,
    multiplier: f64,
    prices: VecDeque<f64>,
    last_mean: f64,
    last_std: f64,
    last_threshold: f64,
}

impl AdaptiveThreshold {
    /// Create a new indicator.
    ///
    /// - `window`     — rolling window size (default 50).
    /// - `multiplier` — std multiplier N (default 2.0).
    pub fn new(window: usize, multiplier: f64) -> Self {
        Self {
            window: window.max(2),
            multiplier,
            prices: VecDeque::with_capacity(window.max(2)),
            last_mean: 0.0,
            last_std: 0.0,
            last_threshold: 0.0,
        }
    }

    fn recompute(&mut self) {
        let n = self.prices.len();
        if n == 0 {
            return;
        }
        let mean: f64 = self.prices.iter().sum::<f64>() / n as f64;
        let std = if n >= 2 {
            let var: f64 = self.prices.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
            var.sqrt()
        } else {
            0.0
        };
        self.last_mean = mean;
        self.last_std = std;
        self.last_threshold = mean + self.multiplier * std;
    }

    /// Passthrough for bar pipeline — returns current value.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_mean, self.last_std, self.last_threshold)
    }

    /// True when window is filled.
    pub fn indicator_is_ready(&self) -> bool {
        self.prices.len() >= self.window
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.prices.clear();
        self.last_mean = 0.0;
        self.last_std = 0.0;
        self.last_threshold = 0.0;
    }
}

impl Default for AdaptiveThreshold {
    fn default() -> Self {
        Self::new(50, 2.0)
    }
}

impl TickConsumer for AdaptiveThreshold {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        if self.prices.len() >= self.window {
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
    fn threshold_above_mean() {
        let mut ind = AdaptiveThreshold::new(5, 2.0);
        for p in [100.0, 102.0, 98.0, 101.0, 99.0] {
            ind.update_tick(&tick(p));
        }
        if let IndicatorValue::Triple(mean, std, threshold) = ind.indicator_value() {
            assert!(threshold > mean, "threshold={threshold} mean={mean}");
            assert!((threshold - mean).abs() - 2.0 * std < 1e-9);
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn threshold_equals_mean_with_zero_std() {
        let mut ind = AdaptiveThreshold::new(3, 2.0);
        for _ in 0..3 {
            ind.update_tick(&tick(100.0)); // all same → std=0
        }
        if let IndicatorValue::Triple(mean, std, threshold) = ind.indicator_value() {
            assert!((mean - 100.0).abs() < 1e-9);
            assert!(std.abs() < 1e-9);
            assert!((threshold - 100.0).abs() < 1e-9);
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = AdaptiveThreshold::default();
        for p in [100.0, 105.0, 95.0] {
            ind.update_tick(&tick(p));
        }
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        assert_eq!(ind.indicator_value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }
}
