//! BlockTradeFlow — rolling cumulative net flow from block trades.

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

/// Rolling net flow from block trades within a time window.
///
/// net_flow = Σ(buy_qty) − Σ(sell_qty) for events within `window_ms` milliseconds.
///
/// Output: `Single(net_flow)`. Returns 0.0 until at least one event.
#[derive(Debug, Clone)]
pub struct BlockTradeFlow {
    /// Circular buffer: (timestamp_ms, quantity, is_buy)
    events: VecDeque<(i64, f64, bool)>,
    window_ms: i64,
    last_net_flow: f64,
}

impl BlockTradeFlow {
    /// Create a new indicator.
    ///
    /// - `window_ms`: rolling time window in milliseconds (clamped to at least 1).
    pub fn new(window_ms: i64) -> Self {
        Self {
            events: VecDeque::new(),
            window_ms: window_ms.max(1),
            last_net_flow: 0.0,
        }
    }

    fn compute_net_flow(events: &VecDeque<(i64, f64, bool)>) -> f64 {
        events.iter().fold(0.0, |acc, &(_, qty, is_buy)| {
            if is_buy { acc + qty } else { acc - qty }
        })
    }
}

impl Default for BlockTradeFlow {
    fn default() -> Self {
        Self::new(60_000) // 1 minute
    }
}

impl BlockTradeConsumer for BlockTradeFlow {
    fn update_block_trade(&mut self, bt: &BlockTrade) {
        let cutoff = bt.timestamp - self.window_ms;
        while self.events.front().map_or(false, |&(ts, _, _)| ts < cutoff) {
            self.events.pop_front();
        }
        self.events.push_back((bt.timestamp, bt.quantity, bt.is_buy));
        self.last_net_flow = Self::compute_net_flow(&self.events);
    }


    fn reset(&mut self) {
        self.events.clear();
        self.last_net_flow = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

/// Typed dual-mode config for [`BlockTradeFlow`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct BlockTradeFlowConfig {
    pub window: crate::contract::Param<TimeWindow>,
}

impl Indicator for BlockTradeFlow {
    const ID: IndicatorId = IndicatorId::BlockTradeFlow;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::BlockTrade];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::BlockTradeFlow)];
    type Config = BlockTradeFlowConfig;
    type Runtime = BlockTradeFlow;

    fn create(cfg: BlockTradeFlowConfig) -> BlockTradeFlow {
        BlockTradeFlow::new(cfg.window.resolved().as_millis())
    }
}

impl crate::contract::Config for BlockTradeFlowConfig {
    fn defaults() -> Self {
        BlockTradeFlowConfig {
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


impl Render for BlockTradeFlow {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::BlockTradeFlow, "Block Trade Flow", Color::hex(0x42A5F5))
            .precision(2)
            .zero_baseline()
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    fn make_bt(timestamp: i64, quantity: f64, is_buy: bool) -> BlockTrade {
        BlockTrade {
            block_id: "test".to_string(),
            price: 100.0,
            quantity,
            is_buy,
            timestamp,
            is_iv: false,
        }
    }

    #[test]
    fn net_flow_buy_dominant() {
        let mut ind = BlockTradeFlow::new(60_000);
        ind.update_block_trade(&make_bt(1000, 10.0, true));
        ind.update_block_trade(&make_bt(2000, 3.0, false));
        let nf = ind.value();
        assert!((nf - 7.0).abs() < 1e-9, "net_flow should be 7.0, got {nf}");
    }

    #[test]
    fn expired_events_excluded() {
        let mut ind = BlockTradeFlow::new(60_000);
        // event at t=0 is before cutoff when t=70_000
        ind.update_block_trade(&make_bt(0, 100.0, true));
        ind.update_block_trade(&make_bt(70_000, 5.0, false));
        // now only the sell at t=70_000 is in window (cutoff = 70_000 - 60_000 = 10_000, so t=0 dropped)
        let nf = ind.value();
        assert!((nf - (-5.0)).abs() < 1e-9, "old event should be dropped, got {nf}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BlockTradeFlow::new(60_000);
        ind.update_block_trade(&make_bt(1000, 10.0, true));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_block_trade_flow() {
        let mut f = IndicatorOrder::BlockTradeFlow(<<BlockTradeFlow as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let bt = BlockTrade {
            block_id: "x".to_string(),
            price: 50_000.0,
            quantity: 10.0,
            is_buy: true,
            timestamp: 1_000,
            is_iv: false,
        };
        f.feed(0, MarketSample::BlockTrade(&bt));
        let v = f.primary();
        assert!((v - 10.0).abs() < 1e-9);
    }
}

impl BlockTradeFlow {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_net_flow
    }
}
