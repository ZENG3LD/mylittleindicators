//! L3LargeOrderTracker — detects unusually large orders in the L3 book.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::orderbook_l3_consumer::OrderbookL3Consumer;
use crate::contract::Render;
use crate::contract::{Color, Family, Indicator, Output, Param, RenderSpec, SourceAxis, sweep_f64};
use crate::core::types::{OrderBookSide, OrderbookL3Event};
use crate::engine::stream_kind::StreamKind;

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
#[derive(Debug, Clone)]
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

/// Typed dual-mode config for [`L3LargeOrderTracker`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct L3LargeOrderTrackerConfig {
    pub window: crate::contract::Param<usize>,
    pub threshold_multiplier: crate::contract::Param<f64>,
}

impl Indicator for L3LargeOrderTracker {
    const ID: IndicatorId = IndicatorId::L3LargeOrderTracker;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderbookL3];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::L3LargeOrderTrackerSide),
        Output::count(IndicatorOutputId::L3LargeOrderTrackerSize),
        Output::price(IndicatorOutputId::L3LargeOrderTrackerPrice),
    ];
    type Config = L3LargeOrderTrackerConfig;
    type Runtime = L3LargeOrderTracker;

    fn create(cfg: L3LargeOrderTrackerConfig) -> L3LargeOrderTracker {
        L3LargeOrderTracker::new(cfg.window.resolved(), cfg.threshold_multiplier.resolved())
    }
}

impl crate::contract::Config for L3LargeOrderTrackerConfig {
    fn defaults() -> Self {
        L3LargeOrderTrackerConfig {
            window: crate::contract::Param::Solo(50),
            threshold_multiplier: crate::contract::Param::Solo(5.0),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // window: Param<usize> — Class A (event-count lookback window); auto range(2,4048,1) is correct.
        // threshold_multiplier: Param<f64> — Class C multiplier (scales median to detect large orders)
        s.threshold_multiplier = Param::many(sweep_f64(0.1, 10.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for L3LargeOrderTracker {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::L3LargeOrderTrackerSide, "Side", Color::hex(0x66BB6A))
            .line_output(IndicatorOutputId::L3LargeOrderTrackerSize, "Size", Color::hex(0x42A5F5))
            .line_output(IndicatorOutputId::L3LargeOrderTrackerPrice, "Price", Color::hex(0xFFA726))
            .precision(2)
            .build()
    }
}

impl L3LargeOrderTracker {
    /// Named output: brace `side`.
    pub fn side(&self) -> f64 { self.last_side }
    /// Named output: brace `size`.
    pub fn size(&self) -> f64 { self.last_size }
    /// Named output: brace `price`.
    pub fn price(&self) -> f64 { self.last_price }
}

impl OrderbookL3Consumer for L3LargeOrderTracker {
    fn update_orderbook_l3(&mut self, l3: &OrderbookL3Event) {
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
        ind.update_orderbook_l3(&make_l3(OrderBookSide::Ask, 6.0, 100.0));
        assert_eq!(ind.side(), 1.0, "Ask large order should be +1.0");
        assert_eq!(ind.size(), 6.0);
    }

    #[test]
    fn large_bid_detected() {
        let mut ind = L3LargeOrderTracker::new(10, 5.0);
        for _ in 0..9 {
            ind.update_orderbook_l3(&make_l3(OrderBookSide::Bid, 1.0, 100.0));
        }
        ind.update_orderbook_l3(&make_l3(OrderBookSide::Bid, 7.0, 100.0));
        assert_eq!(ind.side(), -1.0, "Bid large order should be -1.0");
    }

    #[test]
    fn normal_order_not_flagged() {
        let mut ind = L3LargeOrderTracker::new(10, 5.0);
        for _ in 0..9 {
            ind.update_orderbook_l3(&make_l3(OrderBookSide::Ask, 1.0, 100.0));
        }
        ind.update_orderbook_l3(&make_l3(OrderBookSide::Ask, 1.5, 100.0)); // 1.5×median < 5×
        assert_eq!(ind.side(), 0.0, "normal order should be 0.0");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = L3LargeOrderTracker::new(5, 5.0);
        ind.update_orderbook_l3(&make_l3(OrderBookSide::Ask, 1.0, 100.0));
        ind.update_orderbook_l3(&make_l3(OrderBookSide::Ask, 1.0, 100.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.side(), 0.0);
        assert_eq!(ind.size(), 0.0);
        assert_eq!(ind.price(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_l3_large_order_tracker() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        use crate::core::types::L3Action;
        let mut f = IndicatorOrder::L3LargeOrderTracker(<<L3LargeOrderTracker as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let ev = OrderbookL3Event {
            side: OrderBookSide::Ask,
            order_id: "a".to_string(),
            price: 100.0,
            quantity: 1.0,
            action: L3Action::Add,
            timestamp: 0,
        };
        f.feed(0, MarketSample::OrderbookL3(&ev));
        // f.value() returns the first output (side) as f64
        assert_eq!(f.primary(), 0.0);
    }
}
