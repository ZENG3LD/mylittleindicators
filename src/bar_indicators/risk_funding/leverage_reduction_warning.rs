//! LeverageReductionWarning — detects when exchange tightens leverage limits.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::risk_limit_consumer::RiskLimitConsumer;
use crate::core::types::RiskLimit;

/// Detects exchange leverage tightening (bearish signal) or loosening (bullish).
///
/// - `+1` when new `max_leverage` < previous (exchange tightening)
/// - `-1` when new `max_leverage` > previous (exchange loosening)
/// - `0`  when unchanged or on first update
///
/// Output: `Signal(i8)`.
#[derive(Clone)]
pub struct LeverageReductionWarning {
    prev_max_leverage: f64,
    last_signal: i8,
}

impl LeverageReductionWarning {
    /// Create a new indicator with no prior state.
    pub fn new() -> Self {
        Self {
            prev_max_leverage: f64::NAN,
            last_signal: 0,
        }
    }

    /// Called by `update_bar` passthrough — returns current signal.
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, _close: f64, _volume: f64) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }
}

impl Default for LeverageReductionWarning {
    fn default() -> Self {
        Self::new()
    }
}

impl RiskLimitConsumer for LeverageReductionWarning {
    fn update_risk_limit(&mut self, r: &RiskLimit) -> IndicatorValue {
        if self.prev_max_leverage.is_finite() {
            self.last_signal = if r.max_leverage < self.prev_max_leverage {
                1
            } else if r.max_leverage > self.prev_max_leverage {
                -1
            } else {
                0
            };
        } else {
            self.last_signal = 0;
        }
        self.prev_max_leverage = r.max_leverage;
        IndicatorValue::Signal(self.last_signal)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    fn reset(&mut self) {
        self.prev_max_leverage = f64::NAN;
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        self.prev_max_leverage.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_risk_limit(max_leverage: f64) -> RiskLimit {
        RiskLimit {
            tier: 1,
            max_leverage,
            max_position_value: 100_000.0,
            mmr: 0.005,
            imr: 0.01,
            timestamp: 0,
        }
    }

    #[test]
    fn tightening_gives_positive_signal() {
        let mut ind = LeverageReductionWarning::new();
        ind.update_risk_limit(&make_risk_limit(100.0));
        let val = ind.update_risk_limit(&make_risk_limit(50.0));
        assert_eq!(val, IndicatorValue::Signal(1));
    }

    #[test]
    fn loosening_gives_negative_signal() {
        let mut ind = LeverageReductionWarning::new();
        ind.update_risk_limit(&make_risk_limit(50.0));
        let val = ind.update_risk_limit(&make_risk_limit(100.0));
        assert_eq!(val, IndicatorValue::Signal(-1));
    }

    #[test]
    fn unchanged_gives_zero() {
        let mut ind = LeverageReductionWarning::new();
        ind.update_risk_limit(&make_risk_limit(100.0));
        let val = ind.update_risk_limit(&make_risk_limit(100.0));
        assert_eq!(val, IndicatorValue::Signal(0));
    }

    #[test]
    fn first_update_gives_zero() {
        let mut ind = LeverageReductionWarning::new();
        let val = ind.update_risk_limit(&make_risk_limit(100.0));
        assert_eq!(val, IndicatorValue::Signal(0));
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = LeverageReductionWarning::new();
        ind.update_risk_limit(&make_risk_limit(100.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::Signal(0));
    }
}
