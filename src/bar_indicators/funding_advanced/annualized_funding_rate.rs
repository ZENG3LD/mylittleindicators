//! AnnualizedFundingRate — converts the raw funding rate to annualized percentage.
//!
//! Formula: `annualized = rate × periods_per_day × 365 × 100`
//!
//! For 8-hour funding intervals the default is `periods_per_day = 3.0`.
//!
//! Output: `Single(annualized_pct)`.

use crate::bar_indicators::funding_rate_consumer::FundingRateConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::types::FundingRate;

/// Converts per-snapshot funding rate to annualized percentage.
///
/// `annualized_pct = rate × funding_periods_per_day × 365 × 100`
#[derive(Clone)]
pub struct AnnualizedFundingRate {
    funding_periods_per_day: f64,
    last_value: f64,
    ready: bool,
}

impl AnnualizedFundingRate {
    /// Create a new indicator.
    ///
    /// `funding_periods_per_day`: number of funding events per day.
    /// For 8-hour intervals use `3.0` (default).
    pub fn new(funding_periods_per_day: f64) -> Self {
        Self {
            funding_periods_per_day: funding_periods_per_day.max(0.0),
            last_value: 0.0,
            ready: false,
        }
    }

    fn compute(&self, rate: f64) -> f64 {
        rate * self.funding_periods_per_day * 365.0 * 100.0
    }
}

impl Default for AnnualizedFundingRate {
    fn default() -> Self {
        Self::new(3.0)
    }
}

impl FundingRateConsumer for AnnualizedFundingRate {
    fn update_funding(&mut self, fr: &FundingRate) -> IndicatorValue {
        self.last_value = self.compute(fr.rate);
        self.ready = true;
        IndicatorValue::Single(self.last_value)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_value)
    }

    fn reset(&mut self) {
        self.last_value = 0.0;
        self.ready = false;
    }

    fn is_ready(&self) -> bool {
        self.ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate {
            symbol: "BTCUSDT".to_string(),
            rate,
            next_funding_time: None,
            timestamp: 1000,
        }
    }

    #[test]
    fn annualized_formula_default_periods() {
        // rate=0.0001, periods=3, days=365 → 0.0001 × 3 × 365 × 100 = 10.95
        let mut ind = AnnualizedFundingRate::default();
        let v = ind.update_funding(&make_fr(0.0001));
        if let IndicatorValue::Single(val) = v {
            let expected = 0.0001_f64 * 3.0 * 365.0 * 100.0;
            assert!((val - expected).abs() < 1e-10, "expected {expected}, got {val}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn zero_rate_gives_zero() {
        let mut ind = AnnualizedFundingRate::default();
        let v = ind.update_funding(&make_fr(0.0));
        if let IndicatorValue::Single(val) = v {
            assert_eq!(val, 0.0);
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn negative_rate_gives_negative_annualized() {
        let mut ind = AnnualizedFundingRate::default();
        let v = ind.update_funding(&make_fr(-0.0001));
        if let IndicatorValue::Single(val) = v {
            assert!(val < 0.0, "negative rate should give negative annualized");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn not_ready_before_first_update() {
        let ind = AnnualizedFundingRate::default();
        assert!(!ind.is_ready());
    }

    #[test]
    fn ready_after_first_update() {
        let mut ind = AnnualizedFundingRate::default();
        ind.update_funding(&make_fr(0.0001));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = AnnualizedFundingRate::default();
        ind.update_funding(&make_fr(0.0001));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
