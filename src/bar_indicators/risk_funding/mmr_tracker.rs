//! MmrTracker — tracks the current maintenance margin ratio.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::risk_limit_consumer::RiskLimitConsumer;
use crate::core::types::RiskLimit;

/// Tracks the current maintenance margin ratio (MMR) from exchange risk limit tiers.
///
/// Returns the latest MMR as received, or 0.0 before any update.
///
/// Output: `Single(mmr)`.
#[derive(Clone)]
pub struct MmrTracker {
    current_mmr: f64,
    has_data: bool,
}

impl MmrTracker {
    /// Create a new indicator with no prior state.
    pub fn new() -> Self {
        Self {
            current_mmr: 0.0,
            has_data: false,
        }
    }

    /// Called by `update_bar` passthrough — returns current MMR.
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, _close: f64, _volume: f64) -> IndicatorValue {
        IndicatorValue::Single(self.current_mmr)
    }
}

impl Default for MmrTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl RiskLimitConsumer for MmrTracker {
    fn update_risk_limit(&mut self, r: &RiskLimit) -> IndicatorValue {
        self.current_mmr = r.mmr;
        self.has_data = true;
        IndicatorValue::Single(self.current_mmr)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_mmr)
    }

    fn reset(&mut self) {
        self.current_mmr = 0.0;
        self.has_data = false;
    }

    fn is_ready(&self) -> bool {
        self.has_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_risk_limit(mmr: f64) -> RiskLimit {
        RiskLimit {
            tier: 1,
            max_leverage: 100.0,
            max_position_value: 100_000.0,
            mmr,
            imr: 0.01,
            timestamp: 0,
        }
    }

    #[test]
    fn tracks_mmr_correctly() {
        let mut ind = MmrTracker::new();
        assert!(!ind.is_ready());
        ind.update_risk_limit(&make_risk_limit(0.005));
        assert!(ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert!((v - 0.005).abs() < 1e-12);
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn updates_on_new_risk_limit() {
        let mut ind = MmrTracker::new();
        ind.update_risk_limit(&make_risk_limit(0.005));
        ind.update_risk_limit(&make_risk_limit(0.01));
        if let IndicatorValue::Single(v) = ind.value() {
            assert!((v - 0.01).abs() < 1e-12);
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = MmrTracker::new();
        ind.update_risk_limit(&make_risk_limit(0.005));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
