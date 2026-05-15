//! OiMomentum — linear slope of open interest over a rolling window.
//!
//! slope = (latest_oi - oldest_oi) / (n - 1) where n = number of samples in window.
//! Positive = OI growing (positions accumulating).
//! Negative = OI shrinking (positions unwinding).
//!
//! Output: `Single(slope)`. Zero when fewer than 2 samples.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::open_interest_consumer::OpenInterestConsumer;
use crate::core::types::OpenInterest;

/// Rolling linear slope of open interest.
#[derive(Clone)]
pub struct OiMomentum {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl OiMomentum {
    /// Create with given period (minimum 2).
    pub fn new(period: usize) -> Self {
        Self {
            period: period.max(2),
            history: VecDeque::with_capacity(period.max(2)),
            last_slope: 0.0,
        }
    }
}

impl Default for OiMomentum {
    fn default() -> Self {
        Self::new(14)
    }
}

impl OpenInterestConsumer for OiMomentum {
    fn update_oi(&mut self, oi: &OpenInterest) -> IndicatorValue {
        if self.history.len() == self.period {
            self.history.pop_front();
        }
        self.history.push_back(oi.open_interest);

        let n = self.history.len();
        if n >= 2 {
            let oldest = *self.history.front().expect("history non-empty checked above");
            let latest = *self.history.back().expect("history non-empty checked above");
            self.last_slope = (latest - oldest) / (n - 1) as f64;
        }
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

    fn make_oi(oi: f64) -> OpenInterest {
        OpenInterest {
            symbol: "BTCUSDT".to_string(),
            open_interest: oi,
            open_interest_value: None,
            timestamp: 0,
        }
    }

    #[test]
    fn not_ready_on_single_sample() {
        let mut ind = OiMomentum::new(5);
        ind.update_oi(&make_oi(100.0));
        assert!(!ind.is_ready());
    }

    #[test]
    fn positive_slope_on_rising_oi() {
        // 100 → 200 over 3 bars: slope = (200-100)/(2) = 50
        let mut ind = OiMomentum::new(3);
        ind.update_oi(&make_oi(100.0));
        ind.update_oi(&make_oi(150.0));
        let v = ind.update_oi(&make_oi(200.0));
        if let IndicatorValue::Single(s) = v {
            assert!((s - 50.0).abs() < 1e-9, "expected slope=50, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn negative_slope_on_falling_oi() {
        let mut ind = OiMomentum::new(3);
        ind.update_oi(&make_oi(200.0));
        ind.update_oi(&make_oi(150.0));
        let v = ind.update_oi(&make_oi(100.0));
        if let IndicatorValue::Single(s) = v {
            assert!(s < 0.0, "slope should be negative, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn window_slides_correctly() {
        // period=2: always (latest - oldest) / 1
        let mut ind = OiMomentum::new(2);
        ind.update_oi(&make_oi(100.0));
        ind.update_oi(&make_oi(110.0));
        // slope = 10
        let v1 = ind.value();
        if let IndicatorValue::Single(s) = v1 {
            assert!((s - 10.0).abs() < 1e-9);
        }
        // Next: oldest becomes 110, latest = 90 → slope = -20
        let v2 = ind.update_oi(&make_oi(90.0));
        if let IndicatorValue::Single(s) = v2 {
            assert!((s - (-20.0)).abs() < 1e-9, "expected -20, got {s}");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = OiMomentum::new(5);
        ind.update_oi(&make_oi(100.0));
        ind.update_oi(&make_oi(200.0));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(s) = ind.value() {
            assert_eq!(s, 0.0);
        }
    }
}
