//! BidAskAsymmetry — normalized depth imbalance between top-N bid and ask levels.
//!
//! asymmetry = (bid_depth - ask_depth) / (bid_depth + ask_depth) ∈ [-1, 1]
//!
//! Positive → more volume on bid side (demand pressure).
//! Negative → more volume on ask side (supply pressure).

use crate::engine::streams::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Normalized bid/ask depth asymmetry over top-N levels.
#[derive(Clone, Debug)]
pub struct BidAskAsymmetry {
    top_n: usize,
    last_asymmetry: f64,
}

impl BidAskAsymmetry {
    /// Create with the number of price levels to aggregate on each side.
    pub fn new(top_n: usize) -> Self {
        Self {
            top_n: top_n.max(1),
            last_asymmetry: 0.0,
        }
    }
}

impl Default for BidAskAsymmetry {
    fn default() -> Self {
        Self::new(5)
    }
}

impl OrderBookConsumer for BidAskAsymmetry {
    fn update_orderbook(&mut self, book: &OrderBook) {
        let bid_depth: f64 = book.bids.iter().take(self.top_n).map(|l| l.size).sum();
        let ask_depth: f64 = book.asks.iter().take(self.top_n).map(|l| l.size).sum();
        let total = bid_depth + ask_depth;
        self.last_asymmetry = if total > 0.0 {
            (bid_depth - ask_depth) / total
        } else {
            0.0
        };
    }


    fn reset(&mut self) {
        self.last_asymmetry = 0.0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

/// Typed configuration for [`BidAskAsymmetry`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct BidAskAsymmetryConfig {
    /// Number of price levels to aggregate on each side.
    pub top_n: crate::contract::Param<usize>,
}

impl Indicator for BidAskAsymmetry {
    const ID: IndicatorId = IndicatorId::BidAskAsymmetry;
    /// L2 order-book indicator — normalized depth asymmetry [-1, +1].
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::BidAskAsymmetry)];
    /// O(top_n) per update (two linear passes over the top levels). No heap stores.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[]);
    type Config = BidAskAsymmetryConfig;
    type Runtime = BidAskAsymmetry;

    fn create(cfg: BidAskAsymmetryConfig) -> BidAskAsymmetry {
        BidAskAsymmetry::new(cfg.top_n.resolved())
    }
}

impl crate::contract::Config for BidAskAsymmetryConfig {
    fn defaults() -> Self {
        BidAskAsymmetryConfig { top_n: crate::contract::Param::Solo(5) }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // top_n: Class B (structural count — top N book levels), auto widens to 2..=4048.
        // CORRECT: range(1,20,1) per taxonomy.
        s.top_n = crate::contract::Param::range(1, 20, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for BidAskAsymmetry {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(IndicatorOutputId::BidAskAsymmetry, "Bid/Ask Asymmetry", Color::hex(0x2196F3)))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBook;
    use crate::contract::market_sample::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;

    fn make_book(bids: &[(f64, f64)], asks: &[(f64, f64)]) -> OrderBook {
        OrderBook::from_tuples(bids, asks, 0)
    }

    #[test]
    fn bid_heavy_gives_positive() {
        let mut ind = BidAskAsymmetry::new(5);
        // bid_depth=200, ask_depth=100 → asymmetry = (200-100)/(200+100) = 100/300 ≈ 0.333
        let bids = [(100.0, 200.0)];
        let asks = [(101.0, 100.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        let expected = (200.0 - 100.0) / (200.0 + 100.0);
        assert!((ind.value() - expected).abs() < 1e-10);
        assert!(ind.value() > 0.0);
    }

    #[test]
    fn ask_heavy_gives_negative() {
        let mut ind = BidAskAsymmetry::new(5);
        let bids = [(100.0, 50.0)];
        let asks = [(101.0, 150.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        assert!(ind.value() < 0.0);
    }

    #[test]
    fn balanced_gives_zero() {
        let mut ind = BidAskAsymmetry::new(5);
        let bids = [(100.0, 100.0)];
        let asks = [(101.0, 100.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        assert!((ind.value()).abs() < 1e-10);
    }

    #[test]
    fn empty_book_gives_zero() {
        let mut ind = BidAskAsymmetry::new(5);
        ind.update_orderbook(&make_book(&[], &[]));
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn top_n_limits_levels() {
        let mut ind = BidAskAsymmetry::new(2);
        // 3 bid levels: 100+50+200 = 350 total, but top_n=2 → 100+50=150
        let bids = [(103.0, 100.0), (102.0, 50.0), (101.0, 200.0)];
        let asks = [(104.0, 150.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        let expected = (150.0 - 150.0) / (150.0 + 150.0);
        assert!((ind.value() - expected).abs() < 1e-10);
    }

    #[test]
    fn is_ready_from_start() {
        let ind = BidAskAsymmetry::new(5);
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_value() {
        let mut ind = BidAskAsymmetry::new(5);
        let bids = [(100.0, 200.0)];
        let asks = [(101.0, 100.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        assert!(ind.value().abs() > 0.0);
        ind.reset();
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_bid_ask_asymmetry() {
        let mut f = IndicatorOrder::BidAskAsymmetry(
            <<BidAskAsymmetry as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // bid heavy — asymmetry should be positive
        let book = OrderBook::from_tuples(&[(100.0, 200.0)], &[(101.0, 100.0)], 0);
        f.feed(0, MarketSample::OrderBook(&book));
        assert!(f.primary() > 0.0);
    }
}

impl BidAskAsymmetry {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_asymmetry
    }
}
