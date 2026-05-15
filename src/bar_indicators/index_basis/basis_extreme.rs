//! BasisExtreme — detects when basis is at extreme percentile levels.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::BasisConsumer;
use crate::core::types::Basis;

/// Detects basis at percentile extremes within a rolling window.
///
/// Computes the 95th and 5th percentile of the rolling history.
/// If current basis > p95 → +1, < p5 → -1, otherwise 0.
///
/// Output: `Signal(i8)`.
#[derive(Clone)]
pub struct BasisExtreme {
    period: usize,
    history: VecDeque<f64>,
    last_signal: i8,
}

impl BasisExtreme {
    /// Create a new indicator. `period` is clamped to at least 3.
    pub fn new(period: usize) -> Self {
        let period = period.max(3);
        Self {
            period,
            history: VecDeque::with_capacity(period),
            last_signal: 0,
        }
    }

    fn compute_signal(&self, current: f64) -> i8 {
        let n = self.history.len();
        if n < 2 {
            return 0;
        }
        let mut sorted: Vec<f64> = self.history.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p95_idx = ((n as f64 * 0.95) as usize).min(n.saturating_sub(1));
        let p5_idx = ((n as f64 * 0.05) as usize).min(n.saturating_sub(1));

        let p95 = sorted[p95_idx];
        let p5 = sorted[p5_idx];

        if current > p95 {
            1
        } else if current < p5 {
            -1
        } else {
            0
        }
    }
}

impl Default for BasisExtreme {
    fn default() -> Self {
        Self::new(20)
    }
}

impl BasisConsumer for BasisExtreme {
    fn update_basis(&mut self, b: &Basis) -> IndicatorValue {
        let current = b.basis;
        // Compute signal against existing history before inserting new value
        self.last_signal = self.compute_signal(current);
        self.history.push_back(current);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
        IndicatorValue::Signal(self.last_signal)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    fn reset(&mut self) {
        self.history.clear();
        self.last_signal = 0;
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
    fn extreme_high_gives_plus_one() {
        let mut ind = BasisExtreme::new(20);
        for v in 0..20 {
            ind.update_basis(&make_basis(v as f64));
        }
        // Push a value well above p95
        let v = ind.update_basis(&make_basis(1000.0));
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, 1, "should be +1 for extreme high");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn extreme_low_gives_minus_one() {
        let mut ind = BasisExtreme::new(20);
        for v in 0..20 {
            ind.update_basis(&make_basis(v as f64));
        }
        let v = ind.update_basis(&make_basis(-1000.0));
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, -1, "should be -1 for extreme low");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BasisExtreme::new(5);
        for v in 0..5 {
            ind.update_basis(&make_basis(v as f64));
        }
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Signal(s) = ind.value() {
            assert_eq!(s, 0);
        }
    }
}
