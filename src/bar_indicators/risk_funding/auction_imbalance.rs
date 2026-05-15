//! AuctionImbalance — rolling imbalance of indicative auction quantities.

use std::collections::VecDeque;

use crate::bar_indicators::auction_event_consumer::AuctionEventConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::types::AuctionEvent;

/// Rolling imbalance of auction indicative quantities relative to their rolling average.
///
/// `imbalance = current_qty / rolling_avg_qty`
///
/// Returns 1.0 (neutral) until the window fills. Returns 0.0 before any event.
///
/// Output: `Single(imbalance)`.
#[derive(Clone)]
pub struct AuctionImbalance {
    window_size: usize,
    qty_history: VecDeque<f64>,
    last_imbalance: f64,
}

impl AuctionImbalance {
    /// Create a new indicator. `window_size` is clamped to at least 1.
    pub fn new(window_size: usize) -> Self {
        let window_size = window_size.max(1);
        Self {
            window_size,
            qty_history: VecDeque::with_capacity(window_size),
            last_imbalance: 0.0,
        }
    }

    /// Called by `update_bar` passthrough — returns current imbalance.
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, _close: f64, _volume: f64) -> IndicatorValue {
        IndicatorValue::Single(self.last_imbalance)
    }

    fn rolling_avg(&self) -> f64 {
        if self.qty_history.is_empty() {
            return 0.0;
        }
        self.qty_history.iter().sum::<f64>() / self.qty_history.len() as f64
    }
}

impl Default for AuctionImbalance {
    fn default() -> Self {
        Self::new(20)
    }
}

impl AuctionEventConsumer for AuctionImbalance {
    fn update_auction(&mut self, a: &AuctionEvent) -> IndicatorValue {
        self.qty_history.push_back(a.indicative_qty);
        while self.qty_history.len() > self.window_size {
            self.qty_history.pop_front();
        }
        let avg = self.rolling_avg();
        self.last_imbalance = if avg != 0.0 {
            a.indicative_qty / avg
        } else {
            1.0
        };
        IndicatorValue::Single(self.last_imbalance)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_imbalance)
    }

    fn reset(&mut self) {
        self.qty_history.clear();
        self.last_imbalance = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.qty_history.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_auction(qty: f64) -> AuctionEvent {
        AuctionEvent {
            auction_id: "1".to_string(),
            indicative_price: 100.0,
            indicative_qty: qty,
            state: "indicative".to_string(),
            timestamp: 0,
        }
    }

    #[test]
    fn single_event_imbalance_is_one() {
        let mut ind = AuctionImbalance::new(5);
        let val = ind.update_auction(&make_auction(100.0));
        // avg of 1 element = 100, imbalance = 100/100 = 1.0
        if let IndicatorValue::Single(v) = val {
            assert!((v - 1.0).abs() < 1e-9, "imbalance = {v}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn higher_qty_gives_imbalance_above_one() {
        let mut ind = AuctionImbalance::new(5);
        for _ in 0..5 {
            ind.update_auction(&make_auction(100.0));
        }
        // avg = 100, now spike
        let val = ind.update_auction(&make_auction(200.0));
        if let IndicatorValue::Single(v) = val {
            assert!(v > 1.0, "imbalance should be > 1, got {v}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = AuctionImbalance::new(5);
        ind.update_auction(&make_auction(100.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::Single(0.0));
    }
}
