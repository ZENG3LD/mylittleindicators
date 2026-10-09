//! PriceLevelDensity — count of price levels per unit of price range.
//!
//! Measures how tightly packed the order book levels are.
//! High density → narrow price range, thin book structure.
//! Low density → levels spread wide, coarse book.
//!
//! density_bid = count_bid / (max_bid_price - min_bid_price)
//! density_ask = count_ask / (max_ask_price - min_ask_price)
//!
//! Output: `Triple(density_bid, density_ask, avg(density_bid, density_ask))`

use crate::engine::streams::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Price-level density (levels per price unit) for top-N bid and ask levels.
#[derive(Clone, Debug)]
pub struct PriceLevelDensity {
    top_n: usize,
    last_density_bid: f64,
    last_density_ask: f64,
    last_avg: f64,
}

impl PriceLevelDensity {
    /// Create with the number of price levels to consider on each side.
    pub fn new(top_n: usize) -> Self {
        Self {
            top_n: top_n.max(2),
            last_density_bid: 0.0,
            last_density_ask: 0.0,
            last_avg: 0.0,
        }
    }

    fn compute_density(prices: &[f64]) -> f64 {
        let n = prices.len();
        if n < 2 {
            return 0.0;
        }
        let min = prices.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = prices.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;
        if range <= 0.0 {
            return 0.0;
        }
        n as f64 / range
    }
}

impl PriceLevelDensity {
    /// Price-level density (levels per price unit) on the bid side.
    pub fn density_bid(&self) -> f64 {
        self.last_density_bid
    }

    /// Price-level density (levels per price unit) on the ask side.
    pub fn density_ask(&self) -> f64 {
        self.last_density_ask
    }

    /// Average of bid and ask density.
    pub fn avg(&self) -> f64 {
        self.last_avg
    }
}

impl Default for PriceLevelDensity {
    fn default() -> Self {
        Self::new(10)
    }
}

impl OrderBookConsumer for PriceLevelDensity {
    fn update_orderbook(&mut self, book: &OrderBook) {
        let bid_prices: Vec<f64> = book.bids.iter().take(self.top_n).map(|l| l.price).collect();
        let ask_prices: Vec<f64> = book.asks.iter().take(self.top_n).map(|l| l.price).collect();

        self.last_density_bid = Self::compute_density(&bid_prices);
        self.last_density_ask = Self::compute_density(&ask_prices);
        self.last_avg = (self.last_density_bid + self.last_density_ask) / 2.0;

    }


    fn reset(&mut self) {
        self.last_density_bid = 0.0;
        self.last_density_ask = 0.0;
        self.last_avg = 0.0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

/// Typed configuration for [`PriceLevelDensity`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PriceLevelDensityConfig {
    /// Number of price levels to consider on each side.
    pub top_n: crate::contract::Param<usize>,
}

impl Indicator for PriceLevelDensity {
    const ID: IndicatorId = IndicatorId::PriceLevelDensity;
    /// L2 order-book indicator — price-level packing density per side.
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::count(IndicatorOutputId::PriceLevelDensityDensityBid),
        Output::count(IndicatorOutputId::PriceLevelDensityDensityAsk),
        Output::count(IndicatorOutputId::PriceLevelDensityAvg),
    ];
    /// O(top_n) per update (two linear passes over price levels). No persistent stores.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[]);
    type Config = PriceLevelDensityConfig;
    type Runtime = PriceLevelDensity;

    fn create(cfg: PriceLevelDensityConfig) -> PriceLevelDensity {
        PriceLevelDensity::new(cfg.top_n.resolved())
    }
}

impl crate::contract::Config for PriceLevelDensityConfig {
    fn defaults() -> Self {
        PriceLevelDensityConfig { top_n: crate::contract::Param::Solo(10) }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // top_n: Class B (structural count — number of price levels/buckets per side),
        // auto widens to 2..=4048. CORRECT: range(1,20,1) per taxonomy.
        // No price_bucket field present — no PIN needed.
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


impl Render for PriceLevelDensity {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::PriceLevelDensityDensityBid, "Density Bid", Color::hex(0x4CAF50), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::PriceLevelDensityDensityAsk, "Density Ask", Color::hex(0xF44336), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::PriceLevelDensityAvg, "Avg Density", Color::hex(0xFF9800), 2.0))
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
    fn narrow_spacing_gives_high_density() {
        let mut ind = PriceLevelDensity::new(3);
        // bids at 100.0, 99.9, 99.8 → range = 0.2, count = 3 → density = 15.0
        let bids = [(100.0, 10.0), (99.9, 10.0), (99.8, 10.0)];
        let asks = [(100.1, 10.0), (100.2, 10.0), (100.3, 10.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        assert!((ind.density_bid() - 15.0).abs() < 1e-8, "density_bid should be 15.0, got {}", ind.density_bid());
    }

    #[test]
    fn wide_spacing_gives_low_density() {
        let mut ind = PriceLevelDensity::new(3);
        // bids at 100.0, 90.0, 80.0 → range = 20.0, count = 3 → density = 0.15
        let bids = [(100.0, 10.0), (90.0, 10.0), (80.0, 10.0)];
        let asks = [(101.0, 10.0), (111.0, 10.0), (121.0, 10.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        assert!((ind.density_bid() - 0.15).abs() < 1e-10, "density_bid should be 0.15, got {}", ind.density_bid());
    }

    #[test]
    fn single_level_gives_zero_density() {
        let mut ind = PriceLevelDensity::new(3);
        // Only 1 bid level available → can't compute range
        let bids = [(100.0, 10.0)];
        let asks = [(101.0, 10.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        // density_bid = 0 (only 1 level), density_ask = 0 (only 1 level)
        assert_eq!(ind.density_bid(), 0.0);
    }

    #[test]
    fn avg_is_mean_of_bid_ask_density() {
        let mut ind = PriceLevelDensity::new(2);
        // bids: 100.0, 99.0 → range=1.0, density=2.0
        // asks: 101.0, 103.0 → range=2.0, density=1.0
        let bids = [(100.0, 1.0), (99.0, 1.0)];
        let asks = [(101.0, 1.0), (103.0, 1.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        assert!((ind.density_bid() - 2.0).abs() < 1e-10);
        assert!((ind.density_ask() - 1.0).abs() < 1e-10);
        assert!((ind.avg() - 1.5).abs() < 1e-10);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = PriceLevelDensity::new(3);
        let bids = [(100.0, 10.0), (99.0, 10.0), (98.0, 10.0)];
        let asks = [(101.0, 10.0), (102.0, 10.0), (103.0, 10.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        ind.reset();
        assert_eq!(ind.density_bid(), 0.0);
        assert_eq!(ind.density_ask(), 0.0);
        assert_eq!(ind.avg(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_price_level_density() {
        let mut f = IndicatorOrder::PriceLevelDensity(
            <<PriceLevelDensity as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // Wide spacing → low density (use WILD price in size fields, not price fields)
        let bids: Vec<(f64, f64)> = (0..10).map(|i| (100.0 - i as f64 * 10.0, 9999.0)).collect();
        let asks: Vec<(f64, f64)> = (0..10).map(|i| (101.0 + i as f64 * 10.0, 9999.0)).collect();
        let book = OrderBook::from_tuples(&bids, &asks, 0);
        f.feed(0, MarketSample::OrderBook(&book));
        // density > 0 since we have 10 levels
        assert!(f.primary() > 0.0);
    }
}
