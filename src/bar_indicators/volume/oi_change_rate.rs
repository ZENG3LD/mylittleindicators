//! OI Change Rate — rate of change of open interest per unit time.
//!
//! Measures how fast open interest is growing or shrinking.
//! Positive = OI increasing (new positions opening).
//! Negative = OI decreasing (positions closing / liquidations).
//!
//! Rate = (oi_now - oi_prev) / (dt_seconds)
//!
//! Output: `Double(oi_change_rate, oi_current)`.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::open_interest_consumer::OpenInterestConsumer;
use crate::core::types::OpenInterest;

/// Open interest change rate per second.
#[derive(Clone)]
pub struct OiChangeRate {
    prev_oi: f64,
    prev_ts: i64,
    last_rate: f64,
    last_oi: f64,
    has_prev: bool,
}

impl Default for OiChangeRate {
    fn default() -> Self {
        Self::new()
    }
}

impl OiChangeRate {
    pub fn new() -> Self {
        Self {
            prev_oi: 0.0,
            prev_ts: 0,
            last_rate: 0.0,
            last_oi: 0.0,
            has_prev: false,
        }
    }
}

impl OpenInterestConsumer for OiChangeRate {
    fn update_oi(&mut self, oi: &OpenInterest) -> IndicatorValue {
        self.last_oi = oi.open_interest;
        if self.has_prev {
            let dt_ms = (oi.timestamp - self.prev_ts).max(1);
            let dt_sec = dt_ms as f64 / 1000.0;
            let delta = oi.open_interest - self.prev_oi;
            self.last_rate = delta / dt_sec;
        }
        self.prev_oi = oi.open_interest;
        self.prev_ts = oi.timestamp;
        self.has_prev = true;
        IndicatorValue::Double(self.last_rate, self.last_oi)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.last_rate, self.last_oi)
    }

    fn reset(&mut self) {
        self.prev_oi = 0.0;
        self.prev_ts = 0;
        self.last_rate = 0.0;
        self.last_oi = 0.0;
        self.has_prev = false;
    }

    fn is_ready(&self) -> bool {
        self.has_prev
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_oi(oi: f64, ts: i64) -> OpenInterest {
        OpenInterest { symbol: "BTCUSDT".to_string(), open_interest: oi, open_interest_value: None, timestamp: ts }
    }

    #[test]
    fn not_ready_initially() {
        let oicr = OiChangeRate::new();
        assert!(!oicr.is_ready());
    }

    #[test]
    fn ready_after_first_update() {
        let mut oicr = OiChangeRate::new();
        oicr.update_oi(&make_oi(1000.0, 0));
        assert!(oicr.is_ready());
    }

    #[test]
    fn rate_zero_on_first_update() {
        let mut oicr = OiChangeRate::new();
        let v = oicr.update_oi(&make_oi(1000.0, 0));
        if let IndicatorValue::Double(rate, _) = v {
            assert_eq!(rate, 0.0);
        }
    }

    #[test]
    fn positive_rate_on_growing_oi() {
        let mut oicr = OiChangeRate::new();
        oicr.update_oi(&make_oi(1000.0, 0));
        // 200 OI increase over 2 seconds = 100 per second
        let v = oicr.update_oi(&make_oi(1200.0, 2000));
        if let IndicatorValue::Double(rate, oi) = v {
            assert!((rate - 100.0).abs() < 1e-6, "expected 100/s, got {}", rate);
            assert!((oi - 1200.0).abs() < 1e-9);
        }
    }

    #[test]
    fn negative_rate_on_shrinking_oi() {
        let mut oicr = OiChangeRate::new();
        oicr.update_oi(&make_oi(1000.0, 0));
        let v = oicr.update_oi(&make_oi(800.0, 1000));
        if let IndicatorValue::Double(rate, _) = v {
            assert!(rate < 0.0, "rate should be negative when OI shrinks");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut oicr = OiChangeRate::new();
        oicr.update_oi(&make_oi(1000.0, 0));
        oicr.reset();
        assert!(!oicr.is_ready());
    }
}
