//! Queue Imbalance — bid vs ask size ratio at the top of book.
//!
//! Primary path: `update_orderbook(&OrderBook)` — real queue imbalance:
//!   QI = bid_size / (bid_size + ask_size)
//!   Range [0, 1]: >0.5 = bid-heavy (buy pressure), <0.5 = ask-heavy (sell pressure).
//!   Centred to [-1, 1] as output: 2*QI - 1.
//!
//! `update_bar(o,h,l,c,v)` — no-op (returns current value).
//! OHLCV bars carry no queue data; synthetic price-position approximations are
//! meaningless as queue imbalance.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::order_book_consumer::OrderBookConsumer;
use crate::contract::{Family, Indicator, Output, Param, SourceAxis};
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, RenderOutput, RenderSpec};
use crate::core::types::OrderBook;
use crate::engine::stream_kind::StreamKind;

/// Number of top-of-book levels used for the imbalance calculation.
const DEFAULT_LEVELS: usize = 5;

/// Queue Imbalance indicator.
#[derive(Debug, Clone)]
pub struct QueueImbalance {
    value: f64,
    levels: usize,
}

/// Typed dual-mode config for [`QueueImbalance`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct QueueImbalanceConfig {
    /// Number of top-of-book levels to aggregate per side.
    pub levels: Param<usize>,
}

impl Default for QueueImbalance {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueImbalance {
    pub fn new() -> Self {
        Self { value: 0.0, levels: DEFAULT_LEVELS }
    }

    pub fn with_levels(levels: usize) -> Self {
        Self { value: 0.0, levels: levels.max(1) }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
}

impl Indicator for QueueImbalance {
    const ID: IndicatorId = IndicatorId::ClQueueImb;
    /// L2 orderbook consumer — pluggable in the OrderBook family.
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::ClQueueImb)];
    type Config = QueueImbalanceConfig;
    type Runtime = QueueImbalance;

    fn create(cfg: QueueImbalanceConfig) -> QueueImbalance {
        QueueImbalance::with_levels(cfg.levels.resolved())
    }
}

impl OrderBookConsumer for QueueImbalance {
    /// Real queue imbalance: bid_depth / (bid_depth + ask_depth) centred to [-1, 1].
    /// Uses top `levels` levels of the orderbook.
    fn update_orderbook(&mut self, book: &OrderBook) {
        let bid = book.bid_depth(self.levels);
        let ask = book.ask_depth(self.levels);
        let total = bid + ask;
        self.value = if total > 0.0 {
            2.0 * (bid / total) - 1.0
        } else {
            0.0
        };
    }

    fn reset(&mut self) { self.reset() }
    fn is_ready(&self) -> bool { self.is_ready() }
}

impl crate::contract::Config for QueueImbalanceConfig {
    fn defaults() -> Self {
        QueueImbalanceConfig { levels: Param::Solo(DEFAULT_LEVELS) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // levels: Class B (order-book level depth) — override auto's 2..=4048 to 1..=50 step 1
        s.levels = Param::range(1, 50, 1);
        s
    }
}


impl Render for QueueImbalance {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(IndicatorOutputId::ClQueueImb, "Queue Imbalance", Color::hex(0xFF9800)))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;
    use crate::core::types::OrderBook;

    #[test]
    fn test_queue_imbalance_creation() {
        let ind = QueueImbalance::new();
        assert!(ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_balanced_book() {
        let mut ind = QueueImbalance::new();
        let book = OrderBook::from_tuples(
            &[(100.0, 10.0), (99.0, 10.0)],
            &[(101.0, 10.0), (102.0, 10.0)],
            1000,
        );
        ind.update_orderbook(&book);
        assert!((ind.value()).abs() < 1e-9, "balanced book should give 0");
    }

    #[test]
    fn test_bid_heavy_book() {
        let mut ind = QueueImbalance::new();
        let book = OrderBook::from_tuples(
            &[(100.0, 100.0)],
            &[(101.0, 1.0)],
            1000,
        );
        ind.update_orderbook(&book);
        assert!(ind.value() > 0.0, "bid-heavy book should give positive value");
    }

    #[test]
    fn test_ask_heavy_book() {
        let mut ind = QueueImbalance::new();
        let book = OrderBook::from_tuples(
            &[(100.0, 1.0)],
            &[(101.0, 100.0)],
            1000,
        );
        ind.update_orderbook(&book);
        assert!(ind.value() < 0.0, "ask-heavy book should give negative value");
    }

    #[test]
    fn test_queue_imbalance_reset() {
        let mut ind = QueueImbalance::new();
        let book = OrderBook::from_tuples(
            &[(100.0, 100.0)],
            &[(101.0, 1.0)],
            1000,
        );
        ind.update_orderbook(&book);
        ind.reset();
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_cl_queue_imb() {
        
        let mut f = IndicatorOrder::ClQueueImb(
            <<QueueImbalance as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let book = OrderBook::from_tuples(
            &[(100.0, 10.0)],
            &[(101.0, 5.0)],
            1000,
        );
        f.feed(0, MarketSample::OrderBook(&book));
        assert!(f.primary().is_finite());
    }
}
