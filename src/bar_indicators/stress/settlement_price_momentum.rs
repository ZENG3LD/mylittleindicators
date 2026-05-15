//! SettlementPriceMomentum — rolling linear slope of contract settlement price.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::SettlementEventConsumer;
use crate::core::types::SettlementEvent;

/// Rolling linear slope of contract settlement price.
///
/// slope = (latest − oldest) / (n − 1)
///
/// Output: `Single(slope)`. Returns 0.0 until at least two events.
#[derive(Clone)]
pub struct SettlementPriceMomentum {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl SettlementPriceMomentum {
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

impl Default for SettlementPriceMomentum {
    fn default() -> Self {
        Self::new(14)
    }
}

impl SettlementEventConsumer for SettlementPriceMomentum {
    fn update_settlement(&mut self, s: &SettlementEvent) -> IndicatorValue {
        self.history.push_back(s.settlement_price);
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

    fn make_settlement(price: f64) -> SettlementEvent {
        SettlementEvent {
            settlement_price: price,
            settlement_time: 0,
            timestamp: 0,
        }
    }

    #[test]
    fn rising_price_positive_slope() {
        let mut ind = SettlementPriceMomentum::new(5);
        for v in [100.0, 110.0, 120.0, 130.0, 140.0] {
            ind.update_settlement(&make_settlement(v));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s > 0.0, "slope should be positive, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn falling_price_negative_slope() {
        let mut ind = SettlementPriceMomentum::new(5);
        for v in [140.0, 130.0, 120.0, 110.0, 100.0] {
            ind.update_settlement(&make_settlement(v));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            assert!(s < 0.0, "slope should be negative, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = SettlementPriceMomentum::new(3);
        ind.update_settlement(&make_settlement(100.0));
        ind.update_settlement(&make_settlement(200.0));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
