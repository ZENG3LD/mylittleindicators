//! OiPercentile — rolling percentile rank of current open interest.
//!
//! rank = count_of_window_values_strictly_below_current / window_size ∈ [0, 1)
//! where window_size = number of historic values (excluding current bar).
//!
//! If no history yet: returns 0.0.
//!
//! Output: `Single(percentile_rank)` ∈ [0, 1].

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::open_interest_consumer::OpenInterestConsumer;
use crate::core::types::OpenInterest;

/// Rolling percentile rank of current OI versus its recent history.
#[derive(Clone)]
pub struct OiPercentile {
    window: usize,
    history: VecDeque<f64>,
    last_rank: f64,
}

impl OiPercentile {
    /// Create with given window size (minimum 1).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            history: VecDeque::with_capacity(window.max(1)),
            last_rank: 0.0,
        }
    }
}

impl Default for OiPercentile {
    fn default() -> Self {
        Self::new(50)
    }
}

impl OpenInterestConsumer for OiPercentile {
    fn update_oi(&mut self, oi: &OpenInterest) -> IndicatorValue {
        let current = oi.open_interest;

        let n = self.history.len();
        if n > 0 {
            let count_below = self.history.iter().filter(|&&v| v < current).count();
            self.last_rank = count_below as f64 / n as f64;
        } else {
            self.last_rank = 0.0;
        }

        if self.history.len() == self.window {
            self.history.pop_front();
        }
        self.history.push_back(current);

        IndicatorValue::Single(self.last_rank)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_rank)
    }

    fn reset(&mut self) {
        self.history.clear();
        self.last_rank = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.history.is_empty()
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
    fn not_ready_initially() {
        let ind = OiPercentile::new(5);
        assert!(!ind.is_ready());
    }

    #[test]
    fn first_update_returns_zero() {
        let mut ind = OiPercentile::new(5);
        let v = ind.update_oi(&make_oi(100.0));
        if let IndicatorValue::Single(r) = v {
            // no history before first bar
            assert_eq!(r, 0.0, "first bar has no history to compare");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn rank_one_when_above_all() {
        let mut ind = OiPercentile::new(50);
        // push 10 values of 100
        for _ in 0..10 {
            ind.update_oi(&make_oi(100.0));
        }
        // value 999 is above all 10 history entries
        let v = ind.update_oi(&make_oi(999.0));
        if let IndicatorValue::Single(r) = v {
            assert!((r - 1.0).abs() < 1e-9, "expected rank=1.0, got {r}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn rank_zero_when_below_all() {
        let mut ind = OiPercentile::new(50);
        for _ in 0..5 {
            ind.update_oi(&make_oi(200.0));
        }
        let v = ind.update_oi(&make_oi(1.0));
        if let IndicatorValue::Single(r) = v {
            assert_eq!(r, 0.0, "expected rank=0 when below all history");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn rank_midpoint() {
        let mut ind = OiPercentile::new(50);
        // history: [100, 200, 300, 400]
        ind.update_oi(&make_oi(100.0));
        ind.update_oi(&make_oi(200.0));
        ind.update_oi(&make_oi(300.0));
        ind.update_oi(&make_oi(400.0));
        // current = 250: count_below = 2 (100, 200) out of 4 → 0.5
        let v = ind.update_oi(&make_oi(250.0));
        if let IndicatorValue::Single(r) = v {
            assert!((r - 0.5).abs() < 1e-9, "expected 0.5, got {r}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears() {
        let mut ind = OiPercentile::new(5);
        ind.update_oi(&make_oi(100.0));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(r) = ind.value() {
            assert_eq!(r, 0.0);
        }
    }
}
