//! L3LargeOrderTracker — detects unusually large orders in the L3 book.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::OrderbookL3Consumer;
use crate::core::types::{OrderBookSide, OrderbookL3Event};

/// Detects unusually large orders relative to recent median size.
///
/// Maintains a rolling window of order sizes. For each event, computes the
/// median. If the current order size exceeds `threshold_multiplier × median`,
/// the order is marked as large.
///
/// Output: `Triple(side_as_f64, current_size, price)` when a large order is
/// detected; `Triple(0.0, current_size, price)` otherwise.
///
/// `side_as_f64`: Bid → −1.0, Ask → +1.0, no large order → 0.0.
#[derive(Clone)]
pub struct L3LargeOrderTracker {
    size_history: VecDeque<f64>,
    window_size: usize,
    threshold_multiplier: f64,
    last_side: f64,
    last_size: f64,
    last_price: f64,
}

impl L3LargeOrderTracker {
    /// Create a new indicator.
    ///
    /// - `window_size`: number of recent orders used to compute median (clamped to at least 2).
    /// - `threshold_multiplier`: multiplier applied to median (default 5.0).
    pub fn new(window_size: usize, threshold_multiplier: f64) -> Self {
        let window_size = window_size.max(2);
        Self {
            size_history: VecDeque::with_capacity(window_size),
            window_size,
            threshold_multiplier,
            last_side: 0.0,
            last_size: 0.0,
            last_price: 0.0,
        }
    }

    fn compute_median(history: &VecDeque<f64>) -> f64 {
        if history.is_empty() {
            return 0.0;
        }
        let mut sorted: Vec<f64> = history.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = sorted.len();
        if n % 2 == 1 {
            sorted[n / 2]
        } else {
            (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
        }
    }
}

impl Default for L3LargeOrderTracker {
    fn default() -> Self {
        Self::new(50, 5.0)
    }
}

impl OrderbookL3Consumer for L3LargeOrderTracker {
    fn update_orderbook_l3(&mut self, l3: &OrderbookL3Event) -> IndicatorValue {
        self.last_price = l3.price;
        self.last_size = l3.quantity;

        self.size_history.push_back(l3.quantity);
        while self.size_history.len() > self.window_size {
            self.size_history.pop_front();
        }

        let median = Self::compute_median(&self.size_history);
        let is_large = median > 0.0 && l3.quantity > self.threshold_multiplier * median;

        self.last_side = if is_large {
            match l3.side {
                OrderBookSide::Bid => -1.0,
                OrderBookSide::Ask => 1.0,
            }
        } else {
            0.0
        };

        IndicatorValue::Triple(self.last_side, self.last_size, self.last_price)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_side, self.last_size, self.last_price)
    }

    fn reset(&mut self) {
        self.size_history.clear();
        self.last_side = 0.0;
        self.last_size = 0.0;
        self.last_price = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.size_history.len() >= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::L3Action;

    fn make_l3(side: OrderBookSide, quantity: f64, price: f64) -> OrderbookL3Event {
        OrderbookL3Event {
            side,
            order_id: "test".to_string(),
            price,
            quantity,
            action: L3Action::Add,
            timestamp: 0,
        }
    }

    #[test]
    fn large_ask_detected() {
        let mut ind = L3LargeOrderTracker::new(10, 5.0);
        // fill history with small orders
        for _ in 0..9 {
            ind.update_orderbook_l3(&make_l3(OrderBookSide::Ask, 1.0, 100.0));
        }
        // send a very large ask order (6× median=1.0)
        let v = ind.update_orderbook_l3(&make_l3(OrderBookSide::Ask, 6.0, 100.0));
        if let IndicatorValue::Triple(side, size, _price) = v {
            assert_eq!(side, 1.0, "Ask large order should be +1.0");
            assert_eq!(size, 6.0);
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn large_bid_detected() {
        let mut ind = L3LargeOrderTracker::new(10, 5.0);
        for _ in 0..9 {
            ind.update_orderbook_l3(&make_l3(OrderBookSide::Bid, 1.0, 100.0));
        }
        let v = ind.update_orderbook_l3(&make_l3(OrderBookSide::Bid, 7.0, 100.0));
        if let IndicatorValue::Triple(side, _size, _price) = v {
            assert_eq!(side, -1.0, "Bid large order should be -1.0");
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn normal_order_not_flagged() {
        let mut ind = L3LargeOrderTracker::new(10, 5.0);
        for _ in 0..9 {
            ind.update_orderbook_l3(&make_l3(OrderBookSide::Ask, 1.0, 100.0));
        }
        let v = ind.update_orderbook_l3(&make_l3(OrderBookSide::Ask, 1.5, 100.0)); // 1.5×median < 5×
        if let IndicatorValue::Triple(side, _size, _price) = v {
            assert_eq!(side, 0.0, "normal order should be 0.0");
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = L3LargeOrderTracker::new(5, 5.0);
        ind.update_orderbook_l3(&make_l3(OrderBookSide::Ask, 1.0, 100.0));
        ind.update_orderbook_l3(&make_l3(OrderBookSide::Ask, 1.0, 100.0));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Triple(s, sz, p) = ind.value() {
            assert_eq!(s, 0.0);
            assert_eq!(sz, 0.0);
            assert_eq!(p, 0.0);
        }
    }
}
