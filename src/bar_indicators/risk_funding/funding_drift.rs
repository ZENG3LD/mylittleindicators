//! FundingDrift — spread between predicted and actual funding rate.

use crate::bar_indicators::funding_rate_consumer::FundingRateConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::predicted_funding_consumer::PredictedFundingConsumer;
use crate::core::types::FundingRate;
use crate::core::types::PredictedFunding;

/// Drift between the exchange-predicted funding rate and the actual settled rate.
///
/// `drift = predicted_rate - actual_rate`
///
/// Implements both `PredictedFundingConsumer` and `FundingRateConsumer`.
/// Inherent methods (`indicator_value`, `indicator_is_ready`, `indicator_reset`)
/// are used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
///
/// Output: `Single(drift)`.
#[derive(Clone)]
pub struct FundingDrift {
    last_predicted: f64,
    last_actual: f64,
    last_drift: f64,
}

impl FundingDrift {
    /// Create a new indicator with zeroed state.
    pub fn new() -> Self {
        Self {
            last_predicted: 0.0,
            last_actual: 0.0,
            last_drift: 0.0,
        }
    }

    /// Called by `update_bar` passthrough — returns current drift.
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, _close: f64, _volume: f64) -> IndicatorValue {
        IndicatorValue::Single(self.last_drift)
    }

    /// Current indicator value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_drift)
    }

    /// True if at least one stream has delivered data.
    pub fn indicator_is_ready(&self) -> bool {
        self.last_predicted != 0.0 || self.last_actual != 0.0
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.last_predicted = 0.0;
        self.last_actual = 0.0;
        self.last_drift = 0.0;
    }
}

impl Default for FundingDrift {
    fn default() -> Self {
        Self::new()
    }
}

impl PredictedFundingConsumer for FundingDrift {
    fn update_predicted_funding(&mut self, pf: &PredictedFunding) -> IndicatorValue {
        self.last_predicted = pf.predicted_rate;
        self.last_drift = self.last_predicted - self.last_actual;
        IndicatorValue::Single(self.last_drift)
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
    }

    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl FundingRateConsumer for FundingDrift {
    fn update_funding(&mut self, fr: &FundingRate) -> IndicatorValue {
        self.last_actual = fr.rate;
        self.last_drift = self.last_predicted - self.last_actual;
        IndicatorValue::Single(self.last_drift)
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
    }

    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
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

    fn make_funding_rate(rate: f64) -> FundingRate {
        FundingRate {
            symbol: "BTCUSDT".to_string(),
            rate,
            next_funding_time: None,
            timestamp: 0,
        }
    }

    #[test]
    fn drift_is_predicted_minus_actual() {
        let mut ind = FundingDrift::new();
        ind.update_predicted_funding(&make_predicted(0.001));
        ind.update_funding(&make_funding_rate(0.0003));
        if let IndicatorValue::Single(d) = ind.indicator_value() {
            assert!((d - 0.0007).abs() < 1e-12, "drift = {d}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn negative_drift_when_actual_exceeds_predicted() {
        let mut ind = FundingDrift::new();
        ind.update_predicted_funding(&make_predicted(0.0003));
        ind.update_funding(&make_funding_rate(0.001));
        if let IndicatorValue::Single(d) = ind.indicator_value() {
            assert!(d < 0.0, "drift should be negative, got {d}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = FundingDrift::new();
        ind.update_predicted_funding(&make_predicted(0.001));
        ind.update_funding(&make_funding_rate(0.0003));
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        if let IndicatorValue::Single(v) = ind.indicator_value() {
            assert_eq!(v, 0.0);
        }
    }
}
