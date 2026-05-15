//! MarkPriceMomentum — rolling linear slope of mark price.
//!
//! slope = (latest − oldest) / (n − 1)
//!
//! Analogous to BasisMomentum but consuming MarkPrice snapshots.
//!
//! Output: `Single(slope)`. Returns 0.0 until at least two snapshots.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::mark_price_consumer::MarkPriceConsumer;
use crate::core::types::MarkPrice;

/// Rolling linear slope of mark price.
#[derive(Clone)]
pub struct MarkPriceMomentum {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl MarkPriceMomentum {
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
        let oldest = self.history[0];
        let latest = self.history[n - 1];
        (latest - oldest) / (n as f64 - 1.0)
    }
}

impl Default for MarkPriceMomentum {
    fn default() -> Self {
        Self::new(14)
    }
}

impl MarkPriceConsumer for MarkPriceMomentum {
    fn update_mark(&mut self, mp: &MarkPrice) -> IndicatorValue {
        self.history.push_back(mp.mark_price);
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

    fn make_mp(mark_price: f64) -> MarkPrice {
        MarkPrice {
            symbol: "BTCUSDT".to_string(),
            mark_price,
            index_price: None,
            funding_rate: None,
            timestamp: 0,
        }
    }

    #[test]
    fn rising_price_gives_positive_slope() {
        let mut ind = MarkPriceMomentum::new(5);
        for p in [50000.0, 50100.0, 50200.0, 50300.0, 50400.0] {
            ind.update_mark(&make_mp(p));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s > 0.0, "slope should be positive, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn falling_price_gives_negative_slope() {
        let mut ind = MarkPriceMomentum::new(5);
        for p in [50400.0, 50300.0, 50200.0, 50100.0, 50000.0] {
            ind.update_mark(&make_mp(p));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s < 0.0, "slope should be negative, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn exact_slope_calculation() {
        let mut ind = MarkPriceMomentum::new(3);
        // Values: 100, 102, 106 → slope = (106 - 100) / (3 - 1) = 3.0
        for p in [100.0, 102.0, 106.0] {
            ind.update_mark(&make_mp(p));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!((s - 3.0).abs() < 1e-9, "expected slope=3.0, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn not_ready_with_single_sample() {
        let mut ind = MarkPriceMomentum::new(5);
        ind.update_mark(&make_mp(50000.0));
        assert!(!ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = MarkPriceMomentum::new(5);
        for p in [50000.0, 50100.0, 50200.0] {
            ind.update_mark(&make_mp(p));
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
}
