//! BlockTradeImpact — rolling block trade event rate (events per minute).

use std::collections::VecDeque;

use crate::engine::streams::block_trade_consumer::BlockTradeConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::time_window::TimeWindow;
use crate::contract::Param;
use crate::contract::Render;
use crate::contract::{Color, Family, Indicator, Output, RenderSpec, SourceAxis};
use crate::core::types::BlockTrade;
use crate::engine::stream_kind::StreamKind;

/// Rolling rate of block trade events in events per minute.
///
/// rate = (count of events in window) / window_minutes
///
/// Output: `Single(events_per_min)`. Returns 0.0 until at least one event.
#[derive(Debug, Clone)]
pub struct BlockTradeImpact {
    events: VecDeque<i64>,
    window_ms: i64,
    last_rate: f64,
}

impl BlockTradeImpact {
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
        let window_minutes = window_ms as f64 / 60_000.0;
        count as f64 / window_minutes
    }
}

impl Default for BlockTradeImpact {
    fn default() -> Self {
        Self::new(60_000) // 1 minute
    }
}

impl BlockTradeConsumer for BlockTradeImpact {
    fn update_block_trade(&mut self, bt: &BlockTrade) {
        let cutoff = bt.timestamp - self.window_ms;
        while self.events.front().map_or(false, |&ts| ts < cutoff) {
            self.events.pop_front();
        }
        self.events.push_back(bt.timestamp);
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

/// Typed dual-mode config for [`BlockTradeImpact`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct BlockTradeImpactConfig {
    pub window: crate::contract::Param<TimeWindow>,
}

impl Indicator for BlockTradeImpact {
    const ID: IndicatorId = IndicatorId::BlockTradeImpact;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::BlockTrade];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::BlockTradeImpact)];
    type Config = BlockTradeImpactConfig;
    type Runtime = BlockTradeImpact;

    fn create(cfg: BlockTradeImpactConfig) -> BlockTradeImpact {
        BlockTradeImpact::new(cfg.window.resolved().as_millis())
    }
}

impl crate::contract::Config for BlockTradeImpactConfig {
    fn defaults() -> Self {
        BlockTradeImpactConfig {
            window: crate::contract::Param::Solo(TimeWindow::Minutes(1)),
        }
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


impl Render for BlockTradeImpact {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::BlockTradeImpact, "Block Trade Impact", Color::hex(0xFF7043))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    fn make_bt(timestamp: i64) -> BlockTrade {
        BlockTrade {
            block_id: "test".to_string(),
            price: 100.0,
            quantity: 1.0,
            is_buy: true,
            timestamp,
            is_iv: false,
        }
    }

    #[test]
    fn rate_equals_events_per_minute() {
        // window = 60_000ms = 1 min, 6 events → 6 per min
        let mut ind = BlockTradeImpact::new(60_000);
        for i in 0..6 {
            ind.update_block_trade(&make_bt(i * 10_000));
        }
        let r = ind.value();
        assert!((r - 6.0).abs() < 1e-9, "expected 6.0 events/min, got {r}");
    }

    #[test]
    fn expired_events_excluded() {
        let mut ind = BlockTradeImpact::new(60_000);
        ind.update_block_trade(&make_bt(0));
        // advance by more than window
        ind.update_block_trade(&make_bt(70_000));
        // only t=70_000 is in window
        let r = ind.value();
        let expected = 1.0 / 1.0; // 1 event / 1 min
        assert!((r - expected).abs() < 1e-9, "got {r}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BlockTradeImpact::new(60_000);
        ind.update_block_trade(&make_bt(1000));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_block_trade_impact() {
        let mut f = IndicatorOrder::BlockTradeImpact(<<BlockTradeImpact as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let bt = BlockTrade {
            block_id: "x".to_string(),
            price: 50_000.0,
            quantity: 5.0,
            is_buy: true,
            timestamp: 0,
            is_iv: false,
        };
        f.feed(0, MarketSample::BlockTrade(&bt));
        assert!(f.primary() > 0.0);
    }
}

impl BlockTradeImpact {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_rate
    }
}
