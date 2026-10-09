//! BookDepthChange — delta of bid/ask depth between consecutive snapshots.
//!
//! Tracks how the total size at the top N bid/ask levels changed since
//! the previous snapshot. Positive bid_change = book deepening on bid side.
//!
//! Outputs: `bid_depth_change`, `ask_depth_change`

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::order_book_consumer::OrderBookConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OrderBook;

/// Measures change in aggregated bid/ask depth between consecutive snapshots.
#[derive(Debug, Clone)]
pub struct BookDepthChange {
    /// Number of levels to aggregate on each side.
    levels_to_aggregate: usize,
    prev_bid_depth: f64,
    prev_ask_depth: f64,
    has_prev: bool,
    last_bid_change: f64,
    last_ask_change: f64,
}

impl BookDepthChange {
    /// Create with `levels` levels aggregated per side.
    pub fn new(levels: usize) -> Self {
        Self {
            levels_to_aggregate: levels.max(1),
            prev_bid_depth: 0.0,
            prev_ask_depth: 0.0,
            has_prev: false,
            last_bid_change: 0.0,
            last_ask_change: 0.0,
        }
    }
}

impl BookDepthChange {
    /// Change in aggregated bid depth since previous snapshot.
    pub fn bid(&self) -> f64 {
        self.last_bid_change
    }

    /// Change in aggregated ask depth since previous snapshot.
    pub fn ask(&self) -> f64 {
        self.last_ask_change
    }
}

impl Default for BookDepthChange {
    fn default() -> Self {
        Self::new(10)
    }
}

impl OrderBookConsumer for BookDepthChange {
    fn update_orderbook(&mut self, book: &OrderBook) {
        let bid = book.bid_depth(self.levels_to_aggregate);
        let ask = book.ask_depth(self.levels_to_aggregate);

        if self.has_prev {
            self.last_bid_change = bid - self.prev_bid_depth;
            self.last_ask_change = ask - self.prev_ask_depth;
        }

        self.prev_bid_depth = bid;
        self.prev_ask_depth = ask;
        self.has_prev = true;

    }


    fn reset(&mut self) {
        self.prev_bid_depth = 0.0;
        self.prev_ask_depth = 0.0;
        self.has_prev = false;
        self.last_bid_change = 0.0;
        self.last_ask_change = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.has_prev
    }
}

/// Typed configuration for [`BookDepthChange`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct BookDepthChangeConfig {
    /// Number of L2 levels to aggregate per side.
    pub levels: crate::contract::Param<usize>,
}

impl Indicator for BookDepthChange {
    const ID: IndicatorId = IndicatorId::BookDepthChange;
    /// L2 order-book depth-change indicator.
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::BookDepthChangeBid),
        Output::centered(IndicatorOutputId::BookDepthChangeAsk),
    ];
    /// O(1) per update — simple delta of running sums.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::fixed(StoreKind::Scalar, 4)],
    );
    type Config = BookDepthChangeConfig;
    type Runtime = BookDepthChange;

    fn create(cfg: BookDepthChangeConfig) -> BookDepthChange {
        BookDepthChange::new(cfg.levels.resolved())
    }
}

impl crate::contract::Config for BookDepthChangeConfig {
    fn defaults() -> Self {
        BookDepthChangeConfig { levels: crate::contract::Param::Solo(10) }
    }
    fn machine_defaults() -> Self {
        use crate::contract::Param;
        let mut s = Self::machine_defaults_auto();
        // levels: Class B (order-book depth count, 1–50) — correct off auto range(2,4048,1).
        s.levels = Param::range(1, 50, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for BookDepthChange {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::BookDepthChangeBid, "Bid Depth Change", Color::hex(0x2196F3), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::BookDepthChangeAsk, "Ask Depth Change", Color::hex(0xF44336), 1.5))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBook;
    use crate::contract::market_sample::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;

    fn book(bid_size: f64, ask_size: f64) -> OrderBook {
        OrderBook::from_tuples(
            &[(100.0, bid_size)],
            &[(101.0, ask_size)],
            0,
        )
    }

    #[test]
    fn first_snapshot_gives_zero_change() {
        let mut ind = BookDepthChange::new(5);
        ind.update_orderbook(&book(10.0, 10.0));
        assert_eq!(ind.bid(), 0.0);
        assert_eq!(ind.ask(), 0.0);
    }

    #[test]
    fn second_snapshot_gives_correct_delta() {
        let mut ind = BookDepthChange::new(5);
        ind.update_orderbook(&book(10.0, 10.0));
        ind.update_orderbook(&book(15.0, 8.0));
        assert!((ind.bid() - 5.0).abs() < 1e-9, "bid change: {}", ind.bid());
        assert!((ind.ask() - (-2.0)).abs() < 1e-9, "ask change: {}", ind.ask());
    }

    #[test]
    fn is_ready_after_first_snapshot() {
        let mut ind = BookDepthChange::new(5);
        assert!(!ind.is_ready());
        ind.update_orderbook(&book(10.0, 10.0));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BookDepthChange::new(5);
        ind.update_orderbook(&book(10.0, 10.0));
        ind.update_orderbook(&book(20.0, 5.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.bid(), 0.0);
        assert_eq!(ind.ask(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_book_depth_change() {
        let mut f = IndicatorOrder::BookDepthChange(
            <<BookDepthChange as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let b = book(10.0, 20.0);
        f.feed(0, MarketSample::OrderBook(&b));
        f.feed(0, MarketSample::OrderBook(&b));
        // Same book twice → deltas should be zero on second snapshot
        assert_eq!(f.primary(), 0.0);
    }
}
