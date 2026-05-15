//! BasisZScore — z-score of current basis relative to rolling mean and std.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::BasisConsumer;
use crate::core::types::Basis;

/// Computes the z-score of the current basis value within a rolling window.
///
/// z = (current - mean) / std
///
/// Output: `Single(z)`. Returns 0.0 until at least 2 snapshots.
#[derive(Clone)]
pub struct BasisZScore {
    period: usize,
    history: VecDeque<f64>,
    last_z: f64,
}

impl BasisZScore {
    /// Create a new indicator. `period` is clamped to at least 2.
    pub fn new(period: usize) -> Self {
        let period = period.max(2);
        Self {
            period,
            history: VecDeque::with_capacity(period),
            last_z: 0.0,
        }
    }

    fn compute_z(&self, current: f64) -> f64 {
        let n = self.history.len();
        if n < 2 {
            return 0.0;
        }
        let mean = self.history.iter().sum::<f64>() / n as f64;
        let variance = self.history.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        let std = variance.sqrt();
        if std < 1e-12 {
            0.0
        } else {
            (current - mean) / std
        }
    }
}

impl Default for BasisZScore {
    fn default() -> Self {
        Self::new(20)
    }
}

impl BasisConsumer for BasisZScore {
    fn update_basis(&mut self, b: &Basis) -> IndicatorValue {
        let current = b.basis;
        self.history.push_back(current);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
        self.last_z = self.compute_z(current);
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

    fn make_basis(v: f64) -> Basis {
        Basis { basis: v, timestamp: 0 }
    }

    #[test]
    fn extreme_value_gives_high_zscore() {
        let mut ind = BasisZScore::new(20);
        for v in 0..20 {
            ind.update_basis(&make_basis(v as f64));
        }
        // Value far above mean should give positive z
        let v = ind.update_basis(&make_basis(1000.0));
        if let IndicatorValue::Single(z) = v {
            assert!(z > 2.0, "z should be high for extreme value, got {z}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn constant_series_gives_zero_zscore() {
        let mut ind = BasisZScore::new(5);
        for _ in 0..5 {
            ind.update_basis(&make_basis(5.0));
        }
        if let IndicatorValue::Single(z) = ind.value() {
            assert!(z.abs() < 1e-9, "z should be 0 for constant series, got {z}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BasisZScore::new(5);
        for v in 0..5 {
            ind.update_basis(&make_basis(v as f64));
        }
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(z) = ind.value() {
            assert_eq!(z, 0.0);
        }
    }
}
