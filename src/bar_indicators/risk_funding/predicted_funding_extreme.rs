//! PredictedFundingExtreme — detects extreme predicted funding rate.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::predicted_funding_consumer::PredictedFundingConsumer;
use crate::core::types::PredictedFunding;

/// Detects when the absolute predicted funding rate exceeds a threshold.
///
/// - `+1` when predicted_rate > threshold (extreme positive funding)
/// - `-1` when predicted_rate < -threshold (extreme negative funding)
/// - `0`  otherwise
///
/// Output: `Signal(i8)`.
///
/// Default threshold: `0.001` (= 0.1%).
#[derive(Clone)]
pub struct PredictedFundingExtreme {
    threshold: f64,
    last_signal: i8,
    has_data: bool,
}

impl PredictedFundingExtreme {
    /// Create a new indicator with a custom threshold.
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            last_signal: 0,
            has_data: false,
        }
    }

    /// Create with the default threshold of 0.001.
    pub fn with_default_threshold() -> Self {
        Self::new(0.001)
    }

    /// Called by `update_bar` passthrough — returns current signal.
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, _close: f64, _volume: f64) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }
}

impl Default for PredictedFundingExtreme {
    fn default() -> Self {
        Self::with_default_threshold()
    }
}

impl PredictedFundingConsumer for PredictedFundingExtreme {
    fn update_predicted_funding(&mut self, pf: &PredictedFunding) -> IndicatorValue {
        self.has_data = true;
        self.last_signal = if pf.predicted_rate > self.threshold {
            1
        } else if pf.predicted_rate < -self.threshold {
            -1
        } else {
            0
        };
        IndicatorValue::Signal(self.last_signal)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    fn reset(&mut self) {
        self.last_signal = 0;
        self.has_data = false;
    }

    fn is_ready(&self) -> bool {
        self.has_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_predicted(rate: f64) -> PredictedFunding {
        PredictedFunding {
            predicted_rate: rate,
            next_funding_time: 0,
            timestamp: 0,
        }
    }

    #[test]
    fn positive_extreme_gives_one() {
        let mut ind = PredictedFundingExtreme::new(0.001);
        let val = ind.update_predicted_funding(&make_predicted(0.005));
        assert_eq!(val, IndicatorValue::Signal(1));
    }

    #[test]
    fn negative_extreme_gives_minus_one() {
        let mut ind = PredictedFundingExtreme::new(0.001);
        let val = ind.update_predicted_funding(&make_predicted(-0.005));
        assert_eq!(val, IndicatorValue::Signal(-1));
    }

    #[test]
    fn normal_range_gives_zero() {
        let mut ind = PredictedFundingExtreme::new(0.001);
        let val = ind.update_predicted_funding(&make_predicted(0.0003));
        assert_eq!(val, IndicatorValue::Signal(0));
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = PredictedFundingExtreme::new(0.001);
        ind.update_predicted_funding(&make_predicted(0.005));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::Signal(0));
    }
}
