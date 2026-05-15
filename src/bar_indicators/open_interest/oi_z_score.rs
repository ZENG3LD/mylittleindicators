//! OiZScore — rolling z-score of open interest.
//!
//! Measures how many standard deviations the current OI is from its rolling mean.
//! z = (current_oi - mean) / std
//!
//! Output: `Single(z_score)`. Zero when std = 0 or fewer than 2 samples.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::open_interest_consumer::OpenInterestConsumer;
use crate::core::types::OpenInterest;

/// Rolling z-score of open interest over a configurable window.
#[derive(Clone)]
pub struct OiZScore {
    window: usize,
    history: VecDeque<f64>,
    last_z: f64,
}

impl OiZScore {
    /// Create with given rolling window size (minimum 2).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            history: VecDeque::with_capacity(window.max(2)),
            last_z: 0.0,
        }
    }

    /// Compute z-score of `current` relative to the window (which already includes `current`).
    /// Requires at least 2 values in the window.
    fn compute_z_in_window(window: &VecDeque<f64>) -> f64 {
        let n = window.len();
        if n < 2 {
            return 0.0;
        }
        let nf = n as f64;
        let current = *window.back().expect("window non-empty");
        let mean = window.iter().sum::<f64>() / nf;
        // Population std over the window (consistent with history-based normalisation)
        let variance = window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / nf;
        let std = variance.sqrt();
        if std == 0.0 {
            0.0
        } else {
            (current - mean) / std
        }
    }
}

impl Default for OiZScore {
    fn default() -> Self {
        Self::new(50)
    }
}

impl OpenInterestConsumer for OiZScore {
    fn update_oi(&mut self, oi: &OpenInterest) -> IndicatorValue {
        let current = oi.open_interest;
        // Push first, then compute z within the window (includes current bar)
        if self.history.len() == self.window {
            self.history.pop_front();
        }
        self.history.push_back(current);
        self.last_z = Self::compute_z_in_window(&self.history);
        IndicatorValue::Single(self.last_z)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_z)
    }

    fn reset(&mut self) {
        self.history.clear();
        self.last_z = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_oi(oi: f64) -> OpenInterest {
        OpenInterest {
            symbol: "BTCUSDT".to_string(),
            open_interest: oi,
            open_interest_value: None,
            timestamp: 0,
        }
    }

    #[test]
    fn not_ready_initially() {
        let ind = OiZScore::new(5);
        assert!(!ind.is_ready());
    }

    #[test]
    fn z_positive_on_spike() {
        let mut ind = OiZScore::new(50);
        // Push identical values: mean=100, std=0 at first then with variance
        for _ in 0..10 {
            ind.update_oi(&make_oi(100.0));
        }
        // Now push a spike: z should be positive
        let v = ind.update_oi(&make_oi(150.0));
        if let IndicatorValue::Single(z) = v {
            assert!(z > 0.0, "z should be positive on spike above mean, got {z}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn z_negative_on_dip() {
        let mut ind = OiZScore::new(50);
        for _ in 0..10 {
            ind.update_oi(&make_oi(100.0));
        }
        let v = ind.update_oi(&make_oi(50.0));
        if let IndicatorValue::Single(z) = v {
            assert!(z < 0.0, "z should be negative on dip below mean, got {z}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn z_zero_on_constant_series() {
        let mut ind = OiZScore::new(5);
        for _ in 0..4 {
            ind.update_oi(&make_oi(100.0));
        }
        // After 4 identical pushes, history is [100,100,100,100], next compute std=0
        let v = ind.update_oi(&make_oi(100.0));
        if let IndicatorValue::Single(z) = v {
            assert_eq!(z, 0.0, "z should be 0 when std=0");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = OiZScore::new(5);
        ind.update_oi(&make_oi(100.0));
        ind.update_oi(&make_oi(150.0));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(z) = ind.value() {
            assert_eq!(z, 0.0);
        }
    }

    #[test]
    fn deterministic_sequence() {
        // [100,100,100,100,150]: z for last > 0
        let mut ind = OiZScore::new(50);
        let values = [100.0f64, 100.0, 100.0, 100.0];
        for &v in &values {
            ind.update_oi(&make_oi(v));
        }
        let result = ind.update_oi(&make_oi(150.0));
        if let IndicatorValue::Single(z) = result {
            assert!(z > 0.0, "z should be > 0 for value above mean, got {z}");
        } else {
            panic!("expected Single");
        }
    }
}
