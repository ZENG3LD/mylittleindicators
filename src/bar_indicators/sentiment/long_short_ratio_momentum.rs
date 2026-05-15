//! LongShortRatioMomentum — rolling slope of long_ratio over a window.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::LongShortRatioConsumer;
use crate::core::types::LongShortRatio;

/// Computes the linear slope of `long_ratio` over the last `period` snapshots.
///
/// slope = (latest − oldest) / (n − 1)
///
/// Output: `IndicatorValue::Single(slope)`.
#[derive(Clone)]
pub struct LongShortRatioMomentum {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl LongShortRatioMomentum {
    /// Create a new indicator. `period` is clamped to at least 2.
    pub fn new(period: usize) -> Self {
        let period = period.max(2);
        Self {
            period,
            history: VecDeque::with_capacity(period),
            last_slope: 0.0,
        }
    }

    /// Passthrough for bar events — returns last computed slope unchanged.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        IndicatorValue::Single(self.last_slope)
    }
}

impl LongShortRatioConsumer for LongShortRatioMomentum {
    fn update_long_short_ratio(&mut self, lsr: &LongShortRatio) -> IndicatorValue {
        self.history.push_back(lsr.long_ratio);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
        if self.history.len() >= 2 {
            let oldest = self.history[0];
            let latest = self.history[self.history.len() - 1];
            self.last_slope = (latest - oldest) / (self.history.len() as f64 - 1.0);
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

    fn make_lsr(long_ratio: f64) -> LongShortRatio {
        LongShortRatio {
            symbol: String::new(),
            ratio_type: "global_account".to_string(),
            long_ratio,
            short_ratio: 1.0 - long_ratio,
            ratio: if (1.0 - long_ratio).abs() > 1e-12 {
                Some(long_ratio / (1.0 - long_ratio))
            } else {
                None
            },
            timestamp: 0,
        }
    }

    #[test]
    fn rising_ratio_gives_positive_slope() {
        let mut ind = LongShortRatioMomentum::new(5);
        for v in [0.4, 0.5, 0.6, 0.7, 0.8] {
            ind.update_long_short_ratio(&make_lsr(v));
        }
        if let IndicatorValue::Single(slope) = ind.value() {
            assert!(slope > 0.0, "slope should be positive, got {slope}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn falling_ratio_gives_negative_slope() {
        let mut ind = LongShortRatioMomentum::new(5);
        for v in [0.8, 0.7, 0.6, 0.5, 0.4] {
            ind.update_long_short_ratio(&make_lsr(v));
        }
        if let IndicatorValue::Single(slope) = ind.value() {
            assert!(slope < 0.0, "slope should be negative, got {slope}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn not_ready_until_two_samples() {
        let mut ind = LongShortRatioMomentum::new(5);
        assert!(!ind.is_ready());
        ind.update_long_short_ratio(&make_lsr(0.5));
        assert!(!ind.is_ready());
        ind.update_long_short_ratio(&make_lsr(0.6));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = LongShortRatioMomentum::new(3);
        ind.update_long_short_ratio(&make_lsr(0.4));
        ind.update_long_short_ratio(&make_lsr(0.8));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
