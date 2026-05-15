//! VolIdxMomentum — rolling linear slope of volatility index values.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::VolatilityIndexConsumer;
use crate::core::types::VolatilityIndex;

/// Computes the linear slope of the volatility index over the last `period` snapshots.
///
/// slope = (latest − oldest) / (n − 1)
///
/// Output: `Single(slope)`. Returns 0.0 until at least two snapshots.
#[derive(Clone)]
pub struct VolIdxMomentum {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl VolIdxMomentum {
    /// Create a new indicator. `period` is clamped to at least 2.
    pub fn new(period: usize) -> Self {
        let period = period.max(2);
        Self {
            period,
            history: VecDeque::with_capacity(period),
            last_slope: 0.0,
        }
    }

    fn compute_slope(&self) -> f64 {
        let n = self.history.len();
        if n < 2 {
            return 0.0;
        }
        (self.history[n - 1] - self.history[0]) / (n as f64 - 1.0)
    }
}

impl Default for VolIdxMomentum {
    fn default() -> Self {
        Self::new(14)
    }
}

impl VolatilityIndexConsumer for VolIdxMomentum {
    fn update_volatility_index(&mut self, vi: &VolatilityIndex) -> IndicatorValue {
        self.history.push_back(vi.value);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
        self.last_slope = self.compute_slope();
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

    fn make_vi(v: f64) -> VolatilityIndex {
        VolatilityIndex { value: v, timestamp: 0 }
    }

    #[test]
    fn rising_vol_index_positive_slope() {
        let mut ind = VolIdxMomentum::new(5);
        for v in [10.0, 20.0, 30.0, 40.0, 50.0] {
            ind.update_volatility_index(&make_vi(v));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s > 0.0, "slope should be positive, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn falling_vol_index_negative_slope() {
        let mut ind = VolIdxMomentum::new(5);
        for v in [50.0, 40.0, 30.0, 20.0, 10.0] {
            ind.update_volatility_index(&make_vi(v));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s < 0.0, "slope should be negative, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears() {
        let mut ind = VolIdxMomentum::new(3);
        ind.update_volatility_index(&make_vi(10.0));
        ind.update_volatility_index(&make_vi(20.0));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
