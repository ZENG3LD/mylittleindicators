//! ThetaDecayTracker — rolling cumulative theta (time decay).
//!
//! Sums theta values over a rolling window to track accumulated time decay.
//! Theta is typically negative for long options, so the cumulative sum will be
//! negative when holding long options over time.
//!
//! Output: `Single(cumulative_theta)`.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::OptionGreeksConsumer;
use crate::core::types::OptionGreeks;

/// Tracks cumulative theta decay over a rolling window.
///
/// `cumulative_theta = sum(theta[i] for i in window)`
#[derive(Clone)]
pub struct ThetaDecayTracker {
    window: usize,
    history: VecDeque<f64>,
    last_cumulative: f64,
}

impl ThetaDecayTracker {
    /// Create a new indicator with given rolling window (min 1).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            history: VecDeque::new(),
            last_cumulative: 0.0,
        }
    }
}

impl Default for ThetaDecayTracker {
    fn default() -> Self {
        Self::new(20)
    }
}

impl OptionGreeksConsumer for ThetaDecayTracker {
    fn update_option_greeks(&mut self, g: &OptionGreeks) -> IndicatorValue {
        self.history.push_back(g.theta);
        if self.history.len() > self.window {
            self.history.pop_front();
        }
        self.last_cumulative = self.history.iter().sum();
        IndicatorValue::Single(self.last_cumulative)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_cumulative)
    }

    fn reset(&mut self) {
        self.history.clear();
        self.last_cumulative = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_greeks(theta: f64) -> OptionGreeks {
        OptionGreeks {
            delta: 0.0,
            gamma: 0.0,
            vega: 0.0,
            theta,
            rho: 0.0,
            mark_iv: 0.0,
            bid_iv: None,
            ask_iv: None,
            timestamp: 0,
        }
    }

    #[test]
    fn cumulative_sums_correctly() {
        let mut ind = ThetaDecayTracker::new(5);
        for _ in 0..5 {
            ind.update_option_greeks(&make_greeks(-0.5));
        }
        if let IndicatorValue::Single(v) = ind.value() {
            assert!((v - (-2.5)).abs() < 1e-10, "expected -2.5, got {v}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn rolling_window_drops_oldest() {
        let mut ind = ThetaDecayTracker::new(3);
        // Push 4 values: -1, -2, -3, -4
        // After 4 pushes window holds [-2, -3, -4] → sum = -9
        for i in 1..=4 {
            ind.update_option_greeks(&make_greeks(-(i as f64)));
        }
        if let IndicatorValue::Single(v) = ind.value() {
            assert!((v - (-9.0)).abs() < 1e-10, "expected -9.0, got {v}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut ind = ThetaDecayTracker::new(5);
        for i in 0..4 {
            ind.update_option_greeks(&make_greeks(-(i as f64) * 0.1));
        }
        assert!(!ind.is_ready());
        ind.update_option_greeks(&make_greeks(-0.5));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = ThetaDecayTracker::new(3);
        for _ in 0..3 {
            ind.update_option_greeks(&make_greeks(-0.5));
        }
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
