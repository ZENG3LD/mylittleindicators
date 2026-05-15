//! InsuranceFundMomentum — EMA slope of insurance fund balance.
//!
//! Tracks an exponential moving average of the fund balance and reports
//! the difference between the new EMA and the previous EMA as slope.
//!
//! `new_ema = α × balance + (1 − α) × prev_ema`
//! `slope   = new_ema − prev_ema`
//!
//! where `α = 2 / (period + 1)`.
//!
//! Output: `Single(slope)`. Returns 0.0 until the EMA is initialized.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::InsuranceFundConsumer;
use crate::core::types::InsuranceFund;

/// EMA slope of insurance fund balance.
#[derive(Clone)]
pub struct InsuranceFundMomentum {
    alpha: f64,
    ema: f64,
    prev_ema: f64,
    last_slope: f64,
    initialized: bool,
}

impl InsuranceFundMomentum {
    /// Create a new indicator. `period` is clamped to at least 2.
    pub fn new(period: usize) -> Self {
        let period = period.max(2);
        let alpha = 2.0 / (period as f64 + 1.0);
        Self {
            alpha,
            ema: 0.0,
            prev_ema: 0.0,
            last_slope: 0.0,
            initialized: false,
        }
    }
}

impl Default for InsuranceFundMomentum {
    fn default() -> Self {
        Self::new(14)
    }
}

impl InsuranceFundConsumer for InsuranceFundMomentum {
    fn update_insurance_fund(&mut self, ins: &InsuranceFund) -> IndicatorValue {
        if !self.initialized {
            self.ema = ins.balance;
            self.prev_ema = ins.balance;
            self.initialized = true;
        } else {
            self.prev_ema = self.ema;
            self.ema = self.alpha * ins.balance + (1.0 - self.alpha) * self.prev_ema;
            self.last_slope = self.ema - self.prev_ema;
        }
        IndicatorValue::Single(self.last_slope)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_slope)
    }

    fn reset(&mut self) {
        self.ema = 0.0;
        self.prev_ema = 0.0;
        self.last_slope = 0.0;
        self.initialized = false;
    }

    fn is_ready(&self) -> bool {
        self.initialized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fund(balance: f64) -> InsuranceFund {
        InsuranceFund { balance, timestamp: 0 }
    }

    #[test]
    fn rising_balance_positive_slope() {
        let mut ind = InsuranceFundMomentum::new(5);
        // Feed increasing values; EMA will follow upward → slope > 0
        for v in [10000.0, 11000.0, 12000.0, 13000.0, 14000.0] {
            ind.update_insurance_fund(&make_fund(v));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s > 0.0, "slope should be positive for rising fund, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn declining_balance_negative_slope() {
        let mut ind = InsuranceFundMomentum::new(5);
        for v in [14000.0, 13000.0, 12000.0, 11000.0, 10000.0] {
            ind.update_insurance_fund(&make_fund(v));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s < 0.0, "slope should be negative for declining fund, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn first_update_zero_slope() {
        let mut ind = InsuranceFundMomentum::new(5);
        let val = ind.update_insurance_fund(&make_fund(10000.0));
        assert_eq!(val.main(), 0.0, "first update should give 0 slope");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = InsuranceFundMomentum::new(5);
        ind.update_insurance_fund(&make_fund(10000.0));
        ind.update_insurance_fund(&make_fund(11000.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
}
