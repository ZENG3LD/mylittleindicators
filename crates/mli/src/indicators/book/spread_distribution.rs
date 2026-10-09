//! SpreadDistribution — rolling spread percentile rank.
//!
//! Tracks the last N spread values and computes where the current spread
//! falls within that distribution.
//!
//! Outputs: `spread`, `percentile` where
//! - `spread` = current best_ask - best_bid
//! - `percentile` = 0-100, where 100 = tightest (current spread ≤ all historical)
//!   and 0 = widest (current spread > all historical)
//!
//! Note: percentile is inverted vs raw rank — higher means tighter spread (better liquidity).

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::order_book_consumer::OrderBookConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OrderBook;

/// Rolling percentile rank of bid-ask spread.
#[derive(Clone, Debug)]
pub struct SpreadDistribution {
    window: usize,
    history: VecDeque<f64>,
    last_spread: f64,
    last_percentile: f64,
}

impl SpreadDistribution {
    /// Create with given rolling window size.
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            history: VecDeque::new(),
            last_spread: 0.0,
            last_percentile: 0.0,
        }
    }
}

impl SpreadDistribution {
    /// Current best_ask - best_bid spread.
    pub fn spread(&self) -> f64 {
        self.last_spread
    }

    /// Percentile rank of current spread (0-100; 100 = tightest).
    pub fn percentile(&self) -> f64 {
        self.last_percentile
    }
}

impl Default for SpreadDistribution {
    fn default() -> Self {
        Self::new(50)
    }
}

impl OrderBookConsumer for SpreadDistribution {
    fn update_orderbook(&mut self, book: &OrderBook) {
        let spread = match book.spread() {
            Some(s) if s.is_finite() && s > 0.0 => s,
            _ => return,
        };

        self.last_spread = spread;
        self.history.push_back(spread);
        if self.history.len() > self.window {
            self.history.pop_front();
        }

        // Percentile rank: % of historical spreads that are >= current spread
        // (wider spreads = lower percentile; tighter spread = higher percentile)
        let count_ge = self.history.iter().filter(|&&s| s >= spread).count();
        self.last_percentile = (count_ge as f64 / self.history.len() as f64) * 100.0;

    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_spread = 0.0;
        self.last_percentile = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.history.is_empty()
    }
}

/// Typed configuration for [`SpreadDistribution`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SpreadDistributionConfig {
    /// Rolling window size (number of snapshots).
    pub window: crate::contract::Param<usize>,
}

impl Indicator for SpreadDistribution {
    const ID: IndicatorId = IndicatorId::SpreadDistribution;
    /// L2 order-book spread-distribution indicator.
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::magnitude(IndicatorOutputId::SpreadDistributionSpread),
        Output::percent(IndicatorOutputId::SpreadDistributionPercentile),
    ];
    /// O(window) per update — linear scan for percentile.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = SpreadDistributionConfig;
    type Runtime = SpreadDistribution;

    fn create(cfg: SpreadDistributionConfig) -> SpreadDistribution {
        SpreadDistribution::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for SpreadDistributionConfig {
    fn defaults() -> Self {
        SpreadDistributionConfig { window: crate::contract::Param::Solo(50) }
    }
    fn machine_defaults() -> Self {
        // window: Class A (snapshot window) — auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for SpreadDistribution {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::SpreadDistributionSpread, "Spread", Color::hex(0xFF9800), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::SpreadDistributionPercentile, "Percentile", Color::hex(0x2196F3), 1.5))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{OrderBook, OrderBookLevel};
    use crate::contract::market_sample::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;

    fn make_book_spread(bid: f64, ask: f64) -> OrderBook {
        OrderBook {
            bids: vec![OrderBookLevel::new(bid, 10.0)],
            asks: vec![OrderBookLevel::new(ask, 10.0)],
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn new_not_ready() {
        let sd = SpreadDistribution::new(10);
        assert!(!sd.is_ready());
        assert_eq!(sd.spread(), 0.0);
        assert_eq!(sd.percentile(), 0.0);
    }

    #[test]
    fn first_update_is_ready() {
        let mut sd = SpreadDistribution::new(10);
        sd.update_orderbook(&make_book_spread(100.0, 101.0));
        assert!(sd.is_ready());
    }

    #[test]
    fn tightest_spread_gets_100_percentile() {
        let mut sd = SpreadDistribution::new(10);
        // Fill with spread=1.0
        for _ in 0..5 {
            sd.update_orderbook(&make_book_spread(100.0, 101.0));
        }
        // 6th snapshot with tighter spread=0.5
        sd.update_orderbook(&make_book_spread(100.0, 100.5));
        assert!((sd.spread() - 0.5).abs() < 1e-10);
        // 0.5 is tightest, all 6 entries >= 0.5, so percentile = 100
        assert!((sd.percentile() - 100.0).abs() < 1e-6);
    }

    #[test]
    fn widest_spread_gets_low_percentile() {
        let mut sd = SpreadDistribution::new(10);
        for _ in 0..5 {
            sd.update_orderbook(&make_book_spread(100.0, 101.0));
        }
        sd.update_orderbook(&make_book_spread(100.0, 102.0));
        assert!((sd.spread() - 2.0).abs() < 1e-10);
        assert!(sd.percentile() < 30.0);
    }

    #[test]
    fn zero_spread_skipped() {
        let mut sd = SpreadDistribution::new(5);
        let bad_book = OrderBook { bids: vec![], asks: vec![], timestamp: 0, ..Default::default() };
        sd.update_orderbook(&bad_book);
        assert!(!sd.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut sd = SpreadDistribution::new(5);
        sd.update_orderbook(&make_book_spread(100.0, 101.0));
        assert!(sd.is_ready());
        sd.reset();
        assert!(!sd.is_ready());
        assert_eq!(sd.spread(), 0.0);
        assert_eq!(sd.percentile(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_spread_distribution() {
        let mut f = IndicatorOrder::SpreadDistribution(
            <<SpreadDistribution as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let book = make_book_spread(100.0, 101.0);
        f.feed(0, MarketSample::OrderBook(&book));
        assert!(f.is_ready());
        // Single snapshot → spread=1.0, percentile=100; factory primary = spread
        assert!((f.primary() - 1.0).abs() < 1e-10);
    }
}
