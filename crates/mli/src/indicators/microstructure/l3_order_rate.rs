//! L3OrderRate — rolling rate of L3 orderbook events per second.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::orderbook_l3_consumer::OrderbookL3Consumer;
use crate::engine::time_window::TimeWindow;
use crate::contract::Param;
use crate::contract::Render;
use crate::contract::{Color, Family, Indicator, Output, RenderSpec, SourceAxis};
use crate::core::types::OrderbookL3Event;
use crate::engine::stream_kind::StreamKind;

/// Rolling rate of L3 orderbook events in events per second.
///
/// rate = (count of events in window) / window_seconds
///
/// Output: `Single(events_per_sec)`. Returns 0.0 until at least one event.
#[derive(Debug, Clone)]
pub struct L3OrderRate {
    events: VecDeque<i64>,
    window_ms: i64,
    last_rate: f64,
}

impl L3OrderRate {
    /// Create a new indicator.
    ///
    /// - `window_ms`: rolling time window in milliseconds (clamped to at least 1).
    pub fn new(window_ms: i64) -> Self {
        Self {
            events: VecDeque::new(),
            window_ms: window_ms.max(1),
            last_rate: 0.0,
        }
    }

    fn compute_rate(count: usize, window_ms: i64) -> f64 {
        let window_seconds = window_ms as f64 / 1_000.0;
        count as f64 / window_seconds
    }
}

impl Default for L3OrderRate {
    fn default() -> Self {
        Self::new(10_000) // 10 seconds
    }
}

/// Typed dual-mode config for [`L3OrderRate`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct L3OrderRateConfig {
    pub window: crate::contract::Param<TimeWindow>,
}

impl Indicator for L3OrderRate {
    const ID: IndicatorId = IndicatorId::L3OrderRate;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderbookL3];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::count(IndicatorOutputId::L3OrderRate)];
    type Config = L3OrderRateConfig;
    type Runtime = L3OrderRate;

    fn create(cfg: L3OrderRateConfig) -> L3OrderRate {
        L3OrderRate::new(cfg.window.resolved().as_millis())
    }
}

impl crate::contract::Config for L3OrderRateConfig {
    fn defaults() -> Self {
        L3OrderRateConfig { window: crate::contract::Param::Solo(TimeWindow::Seconds(10)) }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // Class N — TimeWindow discrete set spanning 1s..24h
        s.window = Param::many(vec![
            TimeWindow::Seconds(1),
            TimeWindow::Seconds(5),
            TimeWindow::Seconds(15),
            TimeWindow::Seconds(30),
            TimeWindow::Minutes(1),
            TimeWindow::Minutes(5),
            TimeWindow::Minutes(15),
            TimeWindow::Minutes(30),
            TimeWindow::Hours(1),
            TimeWindow::Hours(4),
            TimeWindow::Hours(24),
        ]);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for L3OrderRate {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::L3OrderRate, "L3 Order Rate", Color::hex(0x26C6DA))
            .precision(1)
            .build()
    }
}

impl OrderbookL3Consumer for L3OrderRate {
    fn update_orderbook_l3(&mut self, l3: &OrderbookL3Event) {
        let cutoff = l3.timestamp - self.window_ms;
        while self.events.front().map_or(false, |&ts| ts < cutoff) {
            self.events.pop_front();
        }
        self.events.push_back(l3.timestamp);
        self.last_rate = Self::compute_rate(self.events.len(), self.window_ms);
    }


    fn reset(&mut self) {
        self.events.clear();
        self.last_rate = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{L3Action, OrderBookSide};

    fn make_l3(timestamp: i64) -> OrderbookL3Event {
        OrderbookL3Event {
            side: OrderBookSide::Bid,
            order_id: "test".to_string(),
            price: 100.0,
            quantity: 1.0,
            action: L3Action::Add,
            timestamp,
        }
    }

    #[test]
    fn rate_per_second() {
        // window = 10_000ms = 10s, 10 events → 1.0 per sec
        let mut ind = L3OrderRate::new(10_000);
        for i in 0..10 {
            ind.update_orderbook_l3(&make_l3(i * 1_000));
        }
        let r = ind.value();
        assert!((r - 1.0).abs() < 1e-9, "expected 1.0 events/sec, got {r}");
    }

    #[test]
    fn expired_events_excluded() {
        let mut ind = L3OrderRate::new(10_000);
        ind.update_orderbook_l3(&make_l3(0));
        ind.update_orderbook_l3(&make_l3(15_000)); // t=0 drops out
        let r = ind.value();
        let expected = 1.0 / 10.0; // 1 event / 10 seconds
        assert!((r - expected).abs() < 1e-9, "got {r}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = L3OrderRate::new(10_000);
        ind.update_orderbook_l3(&make_l3(1000));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_l3_order_rate() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::L3OrderRate(<<L3OrderRate as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let ev = make_l3(0);
        f.feed(0, MarketSample::OrderbookL3(&ev));
        assert!(f.primary() > 0.0);
    }
}

impl L3OrderRate {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_rate
    }
}
