//! SettledFundingMomentum — rolling linear slope of settled funding rates.

use std::collections::VecDeque;

use crate::bar_indicators::funding_settlement_consumer::FundingSettlementConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::types::FundingSettlement;

/// Rolling linear slope of confirmed funding settlement rates.
///
/// slope = (latest − oldest) / (n − 1)
///
/// Returns 0.0 until at least two settlements have been received.
///
/// Output: `Single(slope)`.
#[derive(Clone)]
pub struct SettledFundingMomentum {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl SettledFundingMomentum {
    /// Create a new indicator. `period` is clamped to at least 2.
    pub fn new(period: usize) -> Self {
        let period = period.max(2);
        Self {
            period,
            history: VecDeque::with_capacity(period),
            last_slope: 0.0,
        }
    }

    /// Called by `update_bar` passthrough — returns current slope.
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, _close: f64, _volume: f64) -> IndicatorValue {
        IndicatorValue::Single(self.last_slope)
    }

    fn compute_slope(&self) -> f64 {
        let n = self.history.len();
        if n < 2 {
            return 0.0;
        }
        (self.history[n - 1] - self.history[0]) / (n as f64 - 1.0)
    }
}

impl Default for SettledFundingMomentum {
    fn default() -> Self {
        Self::new(8)
    }
}

impl FundingSettlementConsumer for SettledFundingMomentum {
    fn update_funding_settlement(&mut self, fs: &FundingSettlement) -> IndicatorValue {
        self.history.push_back(fs.settled_rate);
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

    fn make_settlement(rate: f64) -> FundingSettlement {
        FundingSettlement {
            settled_rate: rate,
            settlement_time: 0,
            timestamp: 0,
        }
    }

    #[test]
    fn rising_rates_give_positive_slope() {
        let mut ind = SettledFundingMomentum::new(5);
        for v in [0.001, 0.002, 0.003, 0.004, 0.005] {
            ind.update_funding_settlement(&make_settlement(v));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s > 0.0, "slope should be positive, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn falling_rates_give_negative_slope() {
        let mut ind = SettledFundingMomentum::new(5);
        for v in [0.005, 0.004, 0.003, 0.002, 0.001] {
            ind.update_funding_settlement(&make_settlement(v));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s < 0.0, "slope should be negative, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn not_ready_before_two_events() {
        let mut ind = SettledFundingMomentum::new(5);
        assert!(!ind.is_ready());
        ind.update_funding_settlement(&make_settlement(0.001));
        assert!(!ind.is_ready());
        ind.update_funding_settlement(&make_settlement(0.002));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = SettledFundingMomentum::new(3);
        ind.update_funding_settlement(&make_settlement(0.001));
        ind.update_funding_settlement(&make_settlement(0.002));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
