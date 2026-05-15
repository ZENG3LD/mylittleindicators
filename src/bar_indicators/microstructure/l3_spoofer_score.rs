//! L3SpooferScore — composite spoofing score from cancel ratio + large order frequency.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::orderbook_l3_consumer::OrderbookL3Consumer;
use crate::core::types::{L3Action, OrderbookL3Event};

#[derive(Clone)]
struct L3Entry {
    action: L3Action,
    size: f64,
}

/// Composite spoofing score combining cancel ratio and large order frequency.
///
/// Within a rolling `window` of L3 events:
/// - `cancel_ratio` = Delete count / Add count (0 if no adds)
/// - `large_order_freq` = count of orders with size > `large_size_multiplier × median` / total
/// - `spoofer_score` = (cancel_ratio.min(1.0) + large_order_freq) / 2.0 ∈ [0, 1]
///
/// Output: `Single(spoofer_score)`.
#[derive(Clone)]
pub struct L3SpooferScore {
    window: usize,
    large_size_multiplier: f64,
    events: VecDeque<L3Entry>,
    last_score: f64,
}

impl L3SpooferScore {
    /// Create a new indicator.
    ///
    /// - `window`: number of recent L3 events to track (clamped ≥ 4).
    /// - `large_size_multiplier`: orders with size > N × median are "large" (clamped ≥ 1.0).
    pub fn new(window: usize, large_size_multiplier: f64) -> Self {
        Self {
            window: window.max(4),
            large_size_multiplier: large_size_multiplier.max(1.0),
            events: VecDeque::new(),
            last_score: 0.0,
        }
    }

    fn compute_score(&self) -> f64 {
        if self.events.is_empty() {
            return 0.0;
        }
        // cancel_ratio
        let add_count = self.events.iter().filter(|e| e.action == L3Action::Add).count();
        let del_count = self.events.iter().filter(|e| e.action == L3Action::Delete).count();
        let cancel_ratio = if add_count == 0 {
            0.0
        } else {
            (del_count as f64 / add_count as f64).min(1.0)
        };

        // large_order_freq using median of all sizes
        let total = self.events.len();
        if total == 0 {
            return 0.0;
        }
        let mut sizes: Vec<f64> = self.events.iter().map(|e| e.size).collect();
        sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = if sizes.len() % 2 == 0 {
            (sizes[sizes.len() / 2 - 1] + sizes[sizes.len() / 2]) / 2.0
        } else {
            sizes[sizes.len() / 2]
        };
        let threshold = median * self.large_size_multiplier;
        let large_count = self.events.iter().filter(|e| e.size > threshold).count();
        let large_order_freq = large_count as f64 / total as f64;

        (cancel_ratio + large_order_freq) / 2.0
    }
}

impl Default for L3SpooferScore {
    fn default() -> Self {
        Self::new(100, 3.0)
    }
}

impl OrderbookL3Consumer for L3SpooferScore {
    fn update_orderbook_l3(&mut self, l3: &OrderbookL3Event) -> IndicatorValue {
        self.events.push_back(L3Entry {
            action: l3.action,
            size: l3.quantity,
        });
        while self.events.len() > self.window {
            self.events.pop_front();
        }
        self.last_score = self.compute_score();
        IndicatorValue::Single(self.last_score)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_score)
    }

    fn reset(&mut self) {
        self.events.clear();
        self.last_score = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.events.len() >= 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBookSide;

    fn make_event(action: L3Action, quantity: f64) -> OrderbookL3Event {
        OrderbookL3Event {
            side: OrderBookSide::Ask,
            order_id: "x".to_string(),
            price: 100.0,
            quantity,
            action,
            timestamp: 0,
        }
    }

    #[test]
    fn score_zero_with_only_adds_uniform_size() {
        let mut ind = L3SpooferScore::new(10, 3.0);
        for _ in 0..10 {
            ind.update_orderbook_l3(&make_event(L3Action::Add, 1.0));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            // cancel_ratio = 0, large_order_freq = 0 → score = 0
            assert!((s).abs() < 1e-9, "score should be near 0, got {s}");
        }
    }

    #[test]
    fn score_increases_with_more_cancels() {
        let mut ind = L3SpooferScore::new(10, 3.0);
        // 5 adds and 5 deletes — cancel_ratio = 1.0 (capped), no large orders
        for _ in 0..5 {
            ind.update_orderbook_l3(&make_event(L3Action::Add, 1.0));
        }
        for _ in 0..5 {
            ind.update_orderbook_l3(&make_event(L3Action::Delete, 1.0));
        }
        if let IndicatorValue::Single(s) = ind.value() {
            // cancel_ratio = 1.0 (capped), large = 0 → score = 0.5
            assert!(s > 0.3, "high cancel ratio → score > 0.3, got {s}");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = L3SpooferScore::new(10, 3.0);
        for _ in 0..5 {
            ind.update_orderbook_l3(&make_event(L3Action::Add, 1.0));
        }
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
