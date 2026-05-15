//! Book Churn Rate — rolling average number of level changes per delta.
//!
//! Measures overall orderbook activity: high churn = volatile market maker
//! activity with frequent quotes; low churn = stable, deep book.
//!
//! Output: `Single(avg_changes_per_delta)`.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::orderbook_delta_consumer::OrderbookDeltaConsumer;
use crate::core::types::OrderbookDelta;

/// Rolling average of total level changes (bids + asks) per delta update.
#[derive(Clone)]
pub struct BookChurnRate {
    /// Number of deltas retained in rolling window.
    rolling_window: usize,
    /// Per-delta change count history.
    changes_history: VecDeque<usize>,
    /// Last computed average churn.
    last_churn: f64,
}

impl BookChurnRate {
    /// Create with given rolling window (number of deltas to average over).
    pub fn new(rolling_window: usize) -> Self {
        Self {
            rolling_window: rolling_window.max(1),
            changes_history: VecDeque::new(),
            last_churn: 0.0,
        }
    }
}

impl OrderbookDeltaConsumer for BookChurnRate {
    fn update_delta(&mut self, delta: &OrderbookDelta) -> IndicatorValue {
        let total_changes = delta.total_changes();
        self.changes_history.push_back(total_changes);
        if self.changes_history.len() > self.rolling_window {
            self.changes_history.pop_front();
        }
        let sum: usize = self.changes_history.iter().sum();
        let len = self.changes_history.len();
        self.last_churn = if len > 0 { sum as f64 / len as f64 } else { 0.0 };
        IndicatorValue::Single(self.last_churn)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_churn)
    }

    fn reset(&mut self) {
        self.changes_history.clear();
        self.last_churn = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.changes_history.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBookLevel;

    fn make_delta(bid_count: usize, ask_count: usize) -> OrderbookDelta {
        OrderbookDelta {
            bids: (0..bid_count)
                .map(|i| OrderBookLevel::new(100.0 + i as f64, 5.0))
                .collect(),
            asks: (0..ask_count)
                .map(|i| OrderBookLevel::new(101.0 + i as f64, 5.0))
                .collect(),
            timestamp: 1000,
            first_update_id: None,
            last_update_id: None,
            prev_update_id: None,
            ..Default::default()
        }
    }

    #[test]
    fn not_ready_initially() {
        let churn = BookChurnRate::new(10);
        assert!(!churn.is_ready());
    }

    #[test]
    fn ready_after_first_delta() {
        let mut churn = BookChurnRate::new(10);
        churn.update_delta(&make_delta(2, 3));
        assert!(churn.is_ready());
    }

    #[test]
    fn churn_equals_total_changes_single_delta() {
        let mut churn = BookChurnRate::new(10);
        let v = churn.update_delta(&make_delta(3, 4));
        assert!((v.main() - 7.0).abs() < 1e-9, "3 bids + 4 asks = 7");
    }

    #[test]
    fn rolling_average_over_multiple_deltas() {
        let mut churn = BookChurnRate::new(3);
        churn.update_delta(&make_delta(10, 0)); // 10
        churn.update_delta(&make_delta(0, 20)); // 20
        churn.update_delta(&make_delta(6, 4)); // 10
        // avg = (10 + 20 + 10) / 3 = 13.333...
        let v = churn.value().main();
        assert!((v - 40.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn window_trims_old_deltas() {
        let mut churn = BookChurnRate::new(3);
        for _ in 0..10 {
            churn.update_delta(&make_delta(5, 5));
        }
        assert!(churn.changes_history.len() <= 3);
    }

    #[test]
    fn reset_clears_state() {
        let mut churn = BookChurnRate::new(10);
        churn.update_delta(&make_delta(5, 5));
        churn.reset();
        assert!(!churn.is_ready());
        assert_eq!(churn.value().main(), 0.0);
    }

    #[test]
    fn empty_delta_gives_zero_churn() {
        let mut churn = BookChurnRate::new(10);
        let v = churn.update_delta(&make_delta(0, 0));
        assert_eq!(v.main(), 0.0);
    }
}
