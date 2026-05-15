//! BasisMomentum — rolling linear slope of basis values.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::BasisConsumer;
use crate::core::types::Basis;

/// Computes the linear slope of basis over the last `period` snapshots.
///
/// slope = (latest − oldest) / (n − 1)
///
/// Output: `Single(slope)`. Returns 0.0 until at least two snapshots.
#[derive(Clone)]
pub struct BasisMomentum {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl BasisMomentum {
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
        let oldest = self.history[0];
        let latest = self.history[n - 1];
        (latest - oldest) / (n as f64 - 1.0)
    }
}

impl Default for BasisMomentum {
    fn default() -> Self {
        Self::new(14)
    }
}

impl BasisConsumer for BasisMomentum {
    fn update_basis(&mut self, b: &Basis) -> IndicatorValue {
        self.history.push_back(b.basis);
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

    fn make_basis(v: f64) -> Basis {
        Basis { basis: v, timestamp: 0 }
    }

    #[test]
    fn rising_basis_gives_positive_slope() {
        let mut ind = BasisMomentum::new(5);
        for v in [1.0, 2.0, 3.0, 4.0, 5.0] {
            ind.update_basis(&make_basis(v));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s > 0.0, "slope should be positive, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn falling_basis_gives_negative_slope() {
        let mut ind = BasisMomentum::new(5);
        for v in [5.0, 4.0, 3.0, 2.0, 1.0] {
            ind.update_basis(&make_basis(v));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s < 0.0, "slope should be negative, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BasisMomentum::new(3);
        ind.update_basis(&make_basis(1.0));
        ind.update_basis(&make_basis(2.0));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
