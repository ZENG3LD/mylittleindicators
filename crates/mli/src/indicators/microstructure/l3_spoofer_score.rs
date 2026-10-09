//! L3SpooferScore — composite spoofing score from cancel ratio + large order frequency.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::orderbook_l3_consumer::OrderbookL3Consumer;
use crate::contract::Render;
use crate::contract::{Color, Family, Indicator, Output, Param, RenderSpec, SourceAxis, sweep_f64};
use crate::core::types::{L3Action, OrderbookL3Event};
use crate::engine::stream_kind::StreamKind;

#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

/// Typed dual-mode config for [`L3SpooferScore`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct L3SpooferScoreConfig {
    pub window: crate::contract::Param<usize>,
    pub large_size_multiplier: crate::contract::Param<f64>,
}

impl Indicator for L3SpooferScore {
    const ID: IndicatorId = IndicatorId::L3SpooferScore;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderbookL3];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::L3SpooferScore)];
    type Config = L3SpooferScoreConfig;
    type Runtime = L3SpooferScore;

    fn create(cfg: L3SpooferScoreConfig) -> L3SpooferScore {
        L3SpooferScore::new(cfg.window.resolved(), cfg.large_size_multiplier.resolved())
    }
}

impl crate::contract::Config for L3SpooferScoreConfig {
    fn defaults() -> Self {
        L3SpooferScoreConfig {
            window: crate::contract::Param::Solo(100),
            large_size_multiplier: crate::contract::Param::Solo(3.0),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // window: Param<usize> — Class A (event-count lookback window); auto range(2,4048,1) is correct.
        // large_size_multiplier: Param<f64> — Class C multiplier (scales median for large-order detection)
        s.large_size_multiplier = Param::many(sweep_f64(0.1, 10.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for L3SpooferScore {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::L3SpooferScore, "L3 Spoofer Score", Color::hex(0xFF5722))
            .bounds(0.0, 1.0)
            .precision(3)
            .build()
    }
}

impl OrderbookL3Consumer for L3SpooferScore {
    fn update_orderbook_l3(&mut self, l3: &OrderbookL3Event) {
        self.events.push_back(L3Entry {
            action: l3.action,
            size: l3.quantity,
        });
        while self.events.len() > self.window {
            self.events.pop_front();
        }
        self.last_score = self.compute_score();
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
        let s = ind.value();
        // cancel_ratio = 0, large_order_freq = 0 → score = 0
        assert!((s).abs() < 1e-9, "score should be near 0, got {s}");
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
        let s = ind.value();
        // cancel_ratio = 1.0 (capped), large = 0 → score = 0.5
        assert!(s > 0.3, "high cancel ratio → score > 0.3, got {s}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = L3SpooferScore::new(10, 3.0);
        for _ in 0..5 {
            ind.update_orderbook_l3(&make_event(L3Action::Add, 1.0));
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_l3_spoofer_score() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::L3SpooferScore(<<L3SpooferScore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for _ in 0..4 {
            let ev = make_event(L3Action::Add, 1.0);
            f.feed(0, MarketSample::OrderbookL3(&ev));
        }
        let s = f.primary();
        assert!(s >= 0.0 && s <= 1.0, "s={s}");
    }
}

impl L3SpooferScore {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_score
    }
}
