//! FundingDirectionShift — detects sign changes in the funding rate.
//!
//! Fires `Signal(+1)` when funding flips from negative → positive.
//! Fires `Signal(-1)` when funding flips from positive → negative.
//! Otherwise `Signal(0)`.
//!
//! Output: `Signal(i8)`.

use crate::bar_indicators::funding_rate_consumer::FundingRateConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::types::FundingRate;

/// Detects sign flip in funding rate direction.
///
/// `+1` = flipped to positive (longs pay shorts).
/// `-1` = flipped to negative (shorts pay longs).
/// `0`  = no flip.
#[derive(Clone)]
pub struct FundingDirectionShift {
    prev_rate: Option<f64>,
    last_signal: i8,
}

impl FundingDirectionShift {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self {
            prev_rate: None,
            last_signal: 0,
        }
    }
}

impl Default for FundingDirectionShift {
    fn default() -> Self {
        Self::new()
    }
}

impl FundingRateConsumer for FundingDirectionShift {
    fn update_funding(&mut self, fr: &FundingRate) -> IndicatorValue {
        let current = fr.rate;
        self.last_signal = match self.prev_rate {
            Some(prev) if prev > 0.0 && current < 0.0 => -1,
            Some(prev) if prev < 0.0 && current > 0.0 => 1,
            _ => 0,
        };
        self.prev_rate = Some(current);
        IndicatorValue::Signal(self.last_signal)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    fn reset(&mut self) {
        self.prev_rate = None;
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        self.prev_rate.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate {
            rate,
            next_funding_time: None,
            timestamp: 1000,
        }
    }

    #[test]
    fn pos_to_neg_gives_minus_one() {
        let mut ind = FundingDirectionShift::new();
        ind.update_funding(&make_fr(0.0001));
        let v = ind.update_funding(&make_fr(-0.0001));
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, -1);
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn neg_to_pos_gives_plus_one() {
        let mut ind = FundingDirectionShift::new();
        ind.update_funding(&make_fr(-0.0001));
        let v = ind.update_funding(&make_fr(0.0001));
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, 1);
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn same_sign_gives_zero() {
        let mut ind = FundingDirectionShift::new();
        ind.update_funding(&make_fr(0.0001));
        let v = ind.update_funding(&make_fr(0.0002));
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, 0);
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn not_ready_before_first_update() {
        let ind = FundingDirectionShift::new();
        assert!(!ind.is_ready());
    }

    #[test]
    fn ready_after_first_update() {
        let mut ind = FundingDirectionShift::new();
        ind.update_funding(&make_fr(0.0001));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears() {
        let mut ind = FundingDirectionShift::new();
        ind.update_funding(&make_fr(0.0001));
        ind.update_funding(&make_fr(-0.0001));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Signal(s) = ind.value() {
            assert_eq!(s, 0);
        }
    }
}
