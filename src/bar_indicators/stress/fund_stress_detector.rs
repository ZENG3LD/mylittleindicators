//! FundStressDetector — detects rapid depletion of the insurance fund.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::InsuranceFundConsumer;
use crate::core::types::InsuranceFund;

/// Detects stress on the insurance fund via rapid balance depletion.
///
/// Computes the rolling linear slope of the balance. Fires `Signal(1)` when
/// `slope < -threshold` (fund depleting faster than threshold per step).
///
/// Output: `Signal(i8)`. Returns 0 until at least two snapshots.
#[derive(Clone)]
pub struct FundStressDetector {
    period: usize,
    threshold: f64,
    history: VecDeque<f64>,
    last_signal: i8,
}

impl FundStressDetector {
    /// Create a new indicator.
    ///
    /// - `period`: rolling window size (clamped to at least 2).
    /// - `threshold`: depletion rate that triggers stress signal (default −1000.0).
    ///   Signal fires when `slope < -threshold.abs()`.
    pub fn new(period: usize, threshold: f64) -> Self {
        let period = period.max(2);
        Self {
            period,
            threshold: threshold.abs(),
            history: VecDeque::with_capacity(period),
            last_signal: 0,
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

impl Default for FundStressDetector {
    fn default() -> Self {
        Self::new(14, 1000.0)
    }
}

impl InsuranceFundConsumer for FundStressDetector {
    fn update_insurance_fund(&mut self, ins: &InsuranceFund) -> IndicatorValue {
        self.history.push_back(ins.balance);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
        let slope = self.compute_slope();
        self.last_signal = if slope < -self.threshold { 1 } else { 0 };
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

    fn make_fund(balance: f64) -> InsuranceFund {
        InsuranceFund { balance, timestamp: 0 }
    }

    #[test]
    fn stress_detected_on_rapid_depletion() {
        let mut ind = FundStressDetector::new(5, 1000.0);
        for v in [100_000.0, 90_000.0, 80_000.0, 70_000.0, 60_000.0] {
            ind.update_insurance_fund(&make_fund(v));
        }
        if let IndicatorValue::Signal(s) = ind.value() {
            assert_eq!(s, 1, "should detect stress for rapid depletion");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn no_stress_on_slow_depletion() {
        let mut ind = FundStressDetector::new(5, 1000.0);
        for v in [100_000.0, 99_900.0, 99_800.0, 99_700.0, 99_600.0] {
            ind.update_insurance_fund(&make_fund(v));
        }
        if let IndicatorValue::Signal(s) = ind.value() {
            assert_eq!(s, 0, "should not detect stress for slow depletion");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = FundStressDetector::new(3, 500.0);
        ind.update_insurance_fund(&make_fund(100_000.0));
        ind.update_insurance_fund(&make_fund(50_000.0));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Signal(s) = ind.value() {
            assert_eq!(s, 0);
        }
    }
}
