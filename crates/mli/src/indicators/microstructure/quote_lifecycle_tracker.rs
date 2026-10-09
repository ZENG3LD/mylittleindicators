//! QuoteLifecycleTracker — rolling average lifetime of L3 orders (Add→Delete).

use std::collections::{HashMap, VecDeque};

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::orderbook_l3_consumer::OrderbookL3Consumer;
use crate::contract::Render;
use crate::contract::{Color, Family, Indicator, Output, RenderSpec, SourceAxis};
use crate::core::types::{L3Action, OrderbookL3Event};
use crate::engine::stream_kind::StreamKind;

/// Rolling average lifetime of L3 orderbook quotes.
///
/// Tracks Add→Delete pairs for each order_id. When a Delete arrives,
/// computes `lifetime_ms = delete_ts - add_ts` and adds it to a rolling
/// window. Returns the mean lifetime across the window.
///
/// Modify events are ignored — only Add/Delete pairs are tracked.
///
/// Output: `Single(avg_lifetime_ms)`.
#[derive(Debug, Clone)]
pub struct QuoteLifecycleTracker {
    window_size: usize,
    /// Maps order_id → add timestamp
    pending: HashMap<String, i64>,
    /// Rolling window of completed lifetimes (ms)
    lifetimes: VecDeque<f64>,
    last_avg_lifetime: f64,
}

impl QuoteLifecycleTracker {
    /// Create a new tracker.
    ///
    /// - `window_size`: number of completed lifetimes to average over (clamped ≥ 2).
    pub fn new(window_size: usize) -> Self {
        let window_size = window_size.max(2);
        Self {
            window_size,
            pending: HashMap::new(),
            lifetimes: VecDeque::with_capacity(window_size),
            last_avg_lifetime: 0.0,
        }
    }

    fn compute_avg(&self) -> f64 {
        if self.lifetimes.is_empty() {
            return 0.0;
        }
        self.lifetimes.iter().sum::<f64>() / self.lifetimes.len() as f64
    }
}

impl Default for QuoteLifecycleTracker {
    fn default() -> Self {
        Self::new(50)
    }
}

/// Typed dual-mode config for [`QuoteLifecycleTracker`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct QuoteLifecycleTrackerConfig {
    pub window: crate::contract::Param<usize>,
}

impl Indicator for QuoteLifecycleTracker {
    const ID: IndicatorId = IndicatorId::QuoteLifecycleTracker;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderbookL3];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::count(IndicatorOutputId::QuoteLifecycleTracker)];
    type Config = QuoteLifecycleTrackerConfig;
    type Runtime = QuoteLifecycleTracker;

    fn create(cfg: QuoteLifecycleTrackerConfig) -> QuoteLifecycleTracker {
        QuoteLifecycleTracker::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for QuoteLifecycleTrackerConfig {
    fn defaults() -> Self {
        QuoteLifecycleTrackerConfig { window: crate::contract::Param::Solo(50) }
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


impl Render for QuoteLifecycleTracker {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::QuoteLifecycleTracker, "Avg Quote Lifetime (ms)", Color::hex(0x29B6F6))
            .precision(1)
            .build()
    }
}

impl OrderbookL3Consumer for QuoteLifecycleTracker {
    fn update_orderbook_l3(&mut self, l3: &OrderbookL3Event) {
        match l3.action {
            L3Action::Add => {
                self.pending.insert(l3.order_id.clone(), l3.timestamp);
            }
            L3Action::Delete => {
                if let Some(add_ts) = self.pending.remove(&l3.order_id) {
                    let lifetime = (l3.timestamp - add_ts).max(0) as f64;
                    self.lifetimes.push_back(lifetime);
                    while self.lifetimes.len() > self.window_size {
                        self.lifetimes.pop_front();
                    }
                    self.last_avg_lifetime = self.compute_avg();
                }
            }
            L3Action::Modify => {
                // Ignored — only Add/Delete pairs tracked
            }
        }
    }


    fn reset(&mut self) {
        self.pending.clear();
        self.lifetimes.clear();
        self.last_avg_lifetime = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.lifetimes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBookSide;

    fn add_event(order_id: &str, ts: i64) -> OrderbookL3Event {
        OrderbookL3Event {
            side: OrderBookSide::Bid,
            order_id: order_id.to_string(),
            price: 100.0,
            quantity: 1.0,
            action: L3Action::Add,
            timestamp: ts,
        }
    }

    fn delete_event(order_id: &str, ts: i64) -> OrderbookL3Event {
        OrderbookL3Event {
            side: OrderBookSide::Bid,
            order_id: order_id.to_string(),
            price: 100.0,
            quantity: 0.0,
            action: L3Action::Delete,
            timestamp: ts,
        }
    }

    #[test]
    fn lifetime_computed_correctly() {
        let mut ind = QuoteLifecycleTracker::new(10);
        ind.update_orderbook_l3(&add_event("order1", 1000));
        ind.update_orderbook_l3(&delete_event("order1", 1500));
        let v = ind.value();
        assert!((v - 500.0).abs() < 1e-9, "lifetime = {v}, expected 500.0");
    }

    #[test]
    fn rolling_average_over_multiple_orders() {
        let mut ind = QuoteLifecycleTracker::new(10);
        // order1: 200ms, order2: 400ms → avg = 300ms
        ind.update_orderbook_l3(&add_event("o1", 1000));
        ind.update_orderbook_l3(&add_event("o2", 1000));
        ind.update_orderbook_l3(&delete_event("o1", 1200));
        ind.update_orderbook_l3(&delete_event("o2", 1400));
        let v = ind.value();
        assert!((v - 300.0).abs() < 1e-9, "avg lifetime = {v}, expected 300.0");
    }

    #[test]
    fn orphan_delete_ignored() {
        let mut ind = QuoteLifecycleTracker::new(10);
        ind.update_orderbook_l3(&delete_event("unknown", 1000));
        assert!(!ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = QuoteLifecycleTracker::new(10);
        ind.update_orderbook_l3(&add_event("o1", 0));
        ind.update_orderbook_l3(&delete_event("o1", 100));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_quote_lifecycle_tracker() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::QuoteLifecycleTracker(<<QuoteLifecycleTracker as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        f.feed(0, MarketSample::OrderbookL3(&add_event("x", 0)));
        f.feed(0, MarketSample::OrderbookL3(&delete_event("x", 500)));
        let v = f.primary();
        assert!((v - 500.0).abs() < 1e-9, "v={v}");
    }
}

impl QuoteLifecycleTracker {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_avg_lifetime
    }
}
