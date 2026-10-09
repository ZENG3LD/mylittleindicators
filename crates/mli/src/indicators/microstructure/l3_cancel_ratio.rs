//! L3CancelRatio — rolling ratio of order cancellations to new orders.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::orderbook_l3_consumer::OrderbookL3Consumer;
use crate::contract::Render;
use crate::contract::{Color, Family, Indicator, Output, RenderSpec, SourceAxis};
use crate::core::types::{L3Action, OrderbookL3Event};
use crate::engine::stream_kind::StreamKind;

/// Rolling cancel-to-add ratio from L3 orderbook events.
///
/// ratio = delete_count / add_count within the last `window_size` events.
/// Returns 0.0 when add_count == 0.
///
/// Output: `Single(ratio)`.
#[derive(Debug, Clone)]
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

/// Typed dual-mode config for [`L3CancelRatio`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct L3CancelRatioConfig {
    pub window: crate::contract::Param<usize>,
}

impl Indicator for L3CancelRatio {
    const ID: IndicatorId = IndicatorId::L3CancelRatio;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderbookL3];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::ratio(IndicatorOutputId::L3CancelRatio)];
    type Config = L3CancelRatioConfig;
    type Runtime = L3CancelRatio;

    fn create(cfg: L3CancelRatioConfig) -> L3CancelRatio {
        L3CancelRatio::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for L3CancelRatioConfig {
    fn defaults() -> Self {
        L3CancelRatioConfig { window: crate::contract::Param::Solo(100) }
    }
    fn machine_defaults() -> Self {
        // window: Param<usize> — Class A (event-count lookback window); auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for L3CancelRatio {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::L3CancelRatio, "L3 Cancel Ratio", Color::hex(0xEF5350))
            .precision(3)
            .build()
    }
}

impl OrderbookL3Consumer for L3CancelRatio {
    fn update_orderbook_l3(&mut self, l3: &OrderbookL3Event) {
        self.events.push_back(l3.action);
        while self.events.len() > self.window_size {
            self.events.pop_front();
        }
        self.last_ratio = Self::compute_ratio(&self.events);
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
        let r = ind.value();
        assert!((r - 1.0).abs() < 1e-9, "expected ratio 1.0, got {r}");
    }

    #[test]
    fn no_cancels_ratio_zero() {
        let mut ind = L3CancelRatio::new(4);
        for _ in 0..4 {
            ind.update_orderbook_l3(&make_l3(L3Action::Add));
        }
        assert_eq!(ind.value(), 0.0, "no deletes → ratio should be 0.0");
    }

    #[test]
    fn no_adds_ratio_zero() {
        let mut ind = L3CancelRatio::new(4);
        for _ in 0..4 {
            ind.update_orderbook_l3(&make_l3(L3Action::Delete));
        }
        assert_eq!(ind.value(), 0.0, "no adds → ratio should be 0.0");
    }

    #[test]
    fn window_evicts_old_events() {
        let mut ind = L3CancelRatio::new(2);
        // add two deletes, then two adds — window only holds the two adds
        ind.update_orderbook_l3(&make_l3(L3Action::Delete));
        ind.update_orderbook_l3(&make_l3(L3Action::Delete));
        ind.update_orderbook_l3(&make_l3(L3Action::Add));
        ind.update_orderbook_l3(&make_l3(L3Action::Add));
        assert_eq!(ind.value(), 0.0, "window should only see the two adds, ratio = 0");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = L3CancelRatio::new(4);
        ind.update_orderbook_l3(&make_l3(L3Action::Add));
        ind.update_orderbook_l3(&make_l3(L3Action::Delete));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_l3_cancel_ratio() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::L3CancelRatio(<<L3CancelRatio as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let ev = make_l3(L3Action::Add);
        f.feed(0, MarketSample::OrderbookL3(&ev));
        let ev2 = make_l3(L3Action::Delete);
        f.feed(0, MarketSample::OrderbookL3(&ev2));
        let r = f.primary();
        assert!((r - 1.0).abs() < 1e-9, "r={r}");
    }
}

impl L3CancelRatio {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_ratio
    }
}
