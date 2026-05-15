//! FundDepletionRate — rolling linear slope of insurance fund balance.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::InsuranceFundConsumer;
use crate::core::types::InsuranceFund;

/// Rolling linear slope of insurance fund balance.
///
/// slope = (latest − oldest) / (n − 1)
///
/// Output: `Single(slope)`. Returns 0.0 until at least two snapshots.
#[derive(Clone)]
pub struct FundDepletionRate {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl FundDepletionRate {
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

impl Default for FundDepletionRate {
    fn default() -> Self {
        Self::new(14)
    }
}

impl InsuranceFundConsumer for FundDepletionRate {
    fn update_insurance_fund(&mut self, ins: &InsuranceFund) -> IndicatorValue {
        self.history.push_back(ins.balance);
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

    fn make_fund(balance: f64) -> InsuranceFund {
        InsuranceFund { balance, timestamp: 0 }
    }

    #[test]
    fn declining_balance_negative_slope() {
        let mut ind = FundDepletionRate::new(5);
        for v in [10000.0, 9000.0, 8000.0, 7000.0, 6000.0] {
            ind.update_insurance_fund(&make_fund(v));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s < 0.0, "slope should be negative for declining fund, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn rising_balance_positive_slope() {
        let mut ind = FundDepletionRate::new(5);
        for v in [6000.0, 7000.0, 8000.0, 9000.0, 10000.0] {
            ind.update_insurance_fund(&make_fund(v));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s > 0.0, "slope should be positive for rising fund, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn not_ready_with_one_sample() {
        let mut ind = FundDepletionRate::new(5);
        ind.update_insurance_fund(&make_fund(1000.0));
        assert!(!ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = FundDepletionRate::new(3);
        ind.update_insurance_fund(&make_fund(100.0));
        ind.update_insurance_fund(&make_fund(200.0));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
