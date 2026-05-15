//! L3CancelRatio — rolling ratio of order cancellations to new orders.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::OrderbookL3Consumer;
use crate::core::types::{L3Action, OrderbookL3Event};

/// Rolling cancel-to-add ratio from L3 orderbook events.
///
/// ratio = delete_count / add_count within the last `window_size` events.
/// Returns 0.0 when add_count == 0.
///
/// Output: `Single(ratio)`.
#[derive(Clone)]
pub struct L3CancelRatio {
    events: VecDeque<L3Action>,
    window_size: usize,
    last_ratio: f64,
}

impl L3CancelRatio {
    /// Create a new indicator.
    ///
    /// - `window_size`: number of recent L3 events to track (clamped to at least 2).
    pub fn new(window_size: usize) -> Self {
        let window_size = window_size.max(2);
        Self {
            events: VecDeque::with_capacity(window_size),
            window_size,
            last_ratio: 0.0,
        }
    }

    fn compute_ratio(events: &VecDeque<L3Action>) -> f64 {
        let add_count = events.iter().filter(|&&a| a == L3Action::Add).count();
        let delete_count = events.iter().filter(|&&a| a == L3Action::Delete).count();
        if add_count == 0 {
            0.0
        } else {
            delete_count as f64 / add_count as f64
        }
    }
}

impl Default for L3CancelRatio {
    fn default() -> Self {
        Self::new(100)
    }
}

impl OrderbookL3Consumer for L3CancelRatio {
    fn update_orderbook_l3(&mut self, l3: &OrderbookL3Event) -> IndicatorValue {
        self.events.push_back(l3.action);
        while self.events.len() > self.window_size {
            self.events.pop_front();
        }
        self.last_ratio = Self::compute_ratio(&self.events);
        IndicatorValue::Single(self.last_ratio)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_ratio)
    }

    fn reset(&mut self) {
        self.events.clear();
        self.last_ratio = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.events.len() >= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBookSide;

    fn make_l3(action: L3Action) -> OrderbookL3Event {
        OrderbookL3Event {
            side: OrderBookSide::Ask,
            order_id: "test".to_string(),
            price: 100.0,
            quantity: 1.0,
            action,
            timestamp: 0,
        }
    }

    #[test]
    fn ratio_one_to_one() {
        let mut ind = L3CancelRatio::new(4);
        ind.update_orderbook_l3(&make_l3(L3Action::Add));
        ind.update_orderbook_l3(&make_l3(L3Action::Delete));
        ind.update_orderbook_l3(&make_l3(L3Action::Add));
        ind.update_orderbook_l3(&make_l3(L3Action::Delete));
        if let IndicatorValue::Single(r) = ind.value() {
            assert!((r - 1.0).abs() < 1e-9, "expected ratio 1.0, got {r}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn no_cancels_ratio_zero() {
        let mut ind = L3CancelRatio::new(4);
        for _ in 0..4 {
            ind.update_orderbook_l3(&make_l3(L3Action::Add));
        }
        if let IndicatorValue::Single(r) = ind.value() {
            assert_eq!(r, 0.0, "no deletes → ratio should be 0.0");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn no_adds_ratio_zero() {
        let mut ind = L3CancelRatio::new(4);
        for _ in 0..4 {
            ind.update_orderbook_l3(&make_l3(L3Action::Delete));
        }
        if let IndicatorValue::Single(r) = ind.value() {
            assert_eq!(r, 0.0, "no adds → ratio should be 0.0");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn window_evicts_old_events() {
        let mut ind = L3CancelRatio::new(2);
        // add two deletes, then two adds — window only holds the two adds
        ind.update_orderbook_l3(&make_l3(L3Action::Delete));
        ind.update_orderbook_l3(&make_l3(L3Action::Delete));
        ind.update_orderbook_l3(&make_l3(L3Action::Add));
        ind.update_orderbook_l3(&make_l3(L3Action::Add));
        if let IndicatorValue::Single(r) = ind.value() {
            assert_eq!(r, 0.0, "window should only see the two adds, ratio = 0");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = L3CancelRatio::new(4);
        ind.update_orderbook_l3(&make_l3(L3Action::Add));
        ind.update_orderbook_l3(&make_l3(L3Action::Delete));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
