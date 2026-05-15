//! RiskLimitProximity — proxy score for how restrictive the exchange margin tier is.
//!
//! Cannot compute "proximity to limit" without current position data, which is
//! account-level and not available in indicator streams.
//!
//! SIMPLIFIED: uses (mmr + imr) / 2.0 as a proxy — higher values mean the exchange
//! tier is more restrictive (risk-off environment).

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::risk_limit_consumer::RiskLimitConsumer;
use crate::core::types::RiskLimit;

/// Proxy score for exchange margin requirement restrictiveness.
///
/// score = (mmr + imr) / 2.0
///
/// Higher values → exchange demands more margin → tighter risk environment.
///
/// Note: this is NOT a proximity-to-limit in the position sense — that requires
/// current position data which is account-level. This indicator tracks how
/// restrictive the current tier is relative to its own margin requirements.
///
/// Output: `Single(score)`.
#[derive(Clone)]
pub struct RiskLimitProximity {
    last_score: f64,
    has_data: bool,
}

impl RiskLimitProximity {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self {
            last_score: 0.0,
            has_data: false,
        }
    }

    /// Called by `update_bar` passthrough — returns current score.
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, _close: f64, _volume: f64) -> IndicatorValue {
        IndicatorValue::Single(self.last_score)
    }
}

impl Default for RiskLimitProximity {
    fn default() -> Self {
        Self::new()
    }
}

impl RiskLimitConsumer for RiskLimitProximity {
    fn update_risk_limit(&mut self, r: &RiskLimit) -> IndicatorValue {
        self.last_score = (r.mmr + r.imr) / 2.0;
        self.has_data = true;
        IndicatorValue::Single(self.last_score)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_score)
    }

    fn reset(&mut self) {
        self.last_score = 0.0;
        self.has_data = false;
    }

    fn is_ready(&self) -> bool {
        self.has_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_risk_limit(mmr: f64, imr: f64) -> RiskLimit {
        RiskLimit {
            tier: 1,
            max_leverage: 100.0,
            max_position_value: 100_000.0,
            mmr,
            imr,
            timestamp: 0,
        }
    }

    #[test]
    fn score_is_average_of_mmr_imr() {
        let mut ind = RiskLimitProximity::new();
        let val = ind.update_risk_limit(&make_risk_limit(0.005, 0.01));
        if let IndicatorValue::Single(s) = val {
            let expected = (0.005 + 0.01) / 2.0;
            assert!((s - expected).abs() < 1e-12, "score = {s}, expected {expected}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn score_increases_with_tighter_requirements() {
        let mut ind = RiskLimitProximity::new();
        ind.update_risk_limit(&make_risk_limit(0.005, 0.01));
        let v1 = if let IndicatorValue::Single(v) = ind.value() { v } else { panic!() };
        ind.update_risk_limit(&make_risk_limit(0.01, 0.02));
        let v2 = if let IndicatorValue::Single(v) = ind.value() { v } else { panic!() };
        assert!(v2 > v1, "tighter requirements should give higher score");
    }

    #[test]
    fn not_ready_before_first_update() {
        let ind = RiskLimitProximity::new();
        assert!(!ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = RiskLimitProximity::new();
        ind.update_risk_limit(&make_risk_limit(0.005, 0.01));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
