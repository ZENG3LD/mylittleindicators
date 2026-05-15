//! VegaExposureFlow — rolling linear slope of option vega.
//!
//! Mirrors the approach of `DeltaExposureFlow` but tracks vega instead of delta.
//! A rising slope indicates increasing vega (market pricing more vol risk).
//!
//! Output: `Single(slope)`.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::OptionGreeksConsumer;
use crate::core::types::OptionGreeks;

/// Rolling linear slope of option vega over the last `period` snapshots.
///
/// `slope = (latest - oldest) / (n - 1)`
///
/// Returns 0.0 until at least two snapshots are available.
#[derive(Clone)]
pub struct VegaExposureFlow {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl VegaExposureFlow {
    /// Create a new indicator. `period` is clamped to at least 2.
    pub fn new(period: usize) -> Self {
        let period = period.max(2);
        Self {
            period,
            history: VecDeque::with_capacity(period),
            last_slope: 0.0,
        }
    }

    fn compute_slope(&self) -> f64 {
        let n = self.history.len();
        if n < 2 {
            return 0.0;
        }
        (self.history[n - 1] - self.history[0]) / (n as f64 - 1.0)
    }
}

impl Default for VegaExposureFlow {
    fn default() -> Self {
        Self::new(14)
    }
}

impl OptionGreeksConsumer for VegaExposureFlow {
    fn update_option_greeks(&mut self, g: &OptionGreeks) -> IndicatorValue {
        self.history.push_back(g.vega);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
        self.last_slope = self.compute_slope();
        IndicatorValue::Single(self.last_slope)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_slope)
    }

    fn reset(&mut self) {
        self.history.clear();
        self.last_slope = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_greeks(vega: f64) -> OptionGreeks {
        OptionGreeks {
            delta: 0.0,
            gamma: 0.0,
            vega,
            theta: 0.0,
            rho: 0.0,
            mark_iv: 0.0,
            bid_iv: None,
            ask_iv: None,
            timestamp: 0,
        }
    }

    #[test]
    fn rising_vega_positive_slope() {
        let mut ind = VegaExposureFlow::new(5);
        for v in [10.0, 20.0, 30.0, 40.0, 50.0] {
            ind.update_option_greeks(&make_greeks(v));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s > 0.0, "slope should be positive, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn falling_vega_negative_slope() {
        let mut ind = VegaExposureFlow::new(5);
        for v in [50.0, 40.0, 30.0, 20.0, 10.0] {
            ind.update_option_greeks(&make_greeks(v));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s < 0.0, "slope should be negative, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn not_ready_until_two_snapshots() {
        let mut ind = VegaExposureFlow::new(5);
        ind.update_option_greeks(&make_greeks(10.0));
        assert!(!ind.is_ready());
        ind.update_option_greeks(&make_greeks(20.0));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = VegaExposureFlow::new(3);
        ind.update_option_greeks(&make_greeks(10.0));
        ind.update_option_greeks(&make_greeks(20.0));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
