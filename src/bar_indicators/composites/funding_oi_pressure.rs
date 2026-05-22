//! FundingOiPressure — cross-stream composite of funding rate × OI delta.
//!
//! Dual consumer: `FundingRateConsumer` + `OpenInterestConsumer`.
//!
//! Logic:
//! - `funding` = last funding rate
//! - `oi_delta` = current_oi - prev_oi
//! - `pressure` = funding × oi_delta
//!   - Both same sign → growing directional pressure (positive)
//!   - Opposite signs → declining pressure (negative)
//!
//! Output: `Triple(funding, oi_delta, pressure)`.

use crate::bar_indicators::funding_rate_consumer::FundingRateConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::open_interest_consumer::OpenInterestConsumer;
use crate::core::types::FundingRate;
use crate::core::types::OpenInterest;

/// Cross-stream pressure composite.
///
/// Implements both `FundingRateConsumer` and `OpenInterestConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Clone)]
pub struct FundingOiPressure {
    last_funding: f64,
    last_oi: f64,
    prev_oi: f64,
    oi_seen: usize,
    last_pressure: f64,
    last_oi_delta: f64,
}

impl FundingOiPressure {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self {
            last_funding: 0.0,
            last_oi: 0.0,
            prev_oi: 0.0,
            oi_seen: 0,
            last_pressure: 0.0,
            last_oi_delta: 0.0,
        }
    }

    fn recompute(&mut self) {
        self.last_pressure = self.last_funding * self.last_oi_delta;
    }

    /// Passthrough for bar pipeline — returns current value.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_funding, self.last_oi_delta, self.last_pressure)
    }

    /// True when both streams have delivered at least one update.
    pub fn indicator_is_ready(&self) -> bool {
        self.last_funding != 0.0 && self.oi_seen >= 2
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.last_funding = 0.0;
        self.last_oi = 0.0;
        self.prev_oi = 0.0;
        self.oi_seen = 0;
        self.last_pressure = 0.0;
        self.last_oi_delta = 0.0;
    }
}

impl Default for FundingOiPressure {
    fn default() -> Self {
        Self::new()
    }
}

impl FundingRateConsumer for FundingOiPressure {
    fn update_funding(&mut self, fr: &FundingRate) -> IndicatorValue {
        self.last_funding = fr.rate;
        self.recompute();
        self.indicator_value()
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

impl OpenInterestConsumer for FundingOiPressure {
    fn update_oi(&mut self, oi: &OpenInterest) -> IndicatorValue {
        self.prev_oi = self.last_oi;
        self.last_oi = oi.open_interest;
        self.oi_seen += 1;
        if self.oi_seen >= 2 {
            self.last_oi_delta = self.last_oi - self.prev_oi;
        }
        self.recompute();
        self.indicator_value()
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

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { rate, next_funding_time: None, timestamp: 1000 }
    }

    fn make_oi(open_interest: f64, ts: i64) -> OpenInterest {
        OpenInterest { open_interest, open_interest_value: None, timestamp: ts }
    }

    #[test]
    fn positive_funding_growing_oi_gives_positive_pressure() {
        let mut ind = FundingOiPressure::new();
        ind.update_funding(&make_fr(0.001));
        ind.update_oi(&make_oi(1000.0, 1000));
        ind.update_oi(&make_oi(1100.0, 2000)); // delta = +100
        if let IndicatorValue::Triple(funding, delta, pressure) = ind.indicator_value() {
            assert!((funding - 0.001).abs() < 1e-9);
            assert!((delta - 100.0).abs() < 1e-9);
            assert!(pressure > 0.0);
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn positive_funding_falling_oi_gives_negative_pressure() {
        let mut ind = FundingOiPressure::new();
        ind.update_funding(&make_fr(0.001));
        ind.update_oi(&make_oi(1100.0, 1000));
        ind.update_oi(&make_oi(1000.0, 2000)); // delta = -100
        if let IndicatorValue::Triple(_, _, pressure) = ind.indicator_value() {
            assert!(pressure < 0.0);
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn not_ready_before_two_oi_updates() {
        let mut ind = FundingOiPressure::new();
        ind.update_funding(&make_fr(0.001));
        ind.update_oi(&make_oi(1000.0, 1000));
        assert!(!ind.indicator_is_ready());
        ind.update_oi(&make_oi(1100.0, 2000));
        // funding non-zero, oi_seen >= 2
        assert!(ind.indicator_is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = FundingOiPressure::new();
        ind.update_funding(&make_fr(0.001));
        ind.update_oi(&make_oi(1000.0, 1000));
        ind.update_oi(&make_oi(1100.0, 2000));
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        if let IndicatorValue::Triple(f, d, p) = ind.indicator_value() {
            assert_eq!(f, 0.0);
            assert_eq!(d, 0.0);
            assert_eq!(p, 0.0);
        } else {
            panic!("expected Triple");
        }
    }
}
