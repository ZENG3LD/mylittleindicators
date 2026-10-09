//! LayerConcentration — Gini coefficient of depth distribution across price levels.
//!
//! High Gini → liquidity concentrated on a few levels (wall-like).
//! Low Gini → liquidity distributed evenly across levels.
//!
//! Output: `Triple(gini_bid, gini_ask, max(gini_bid, gini_ask))`

use crate::engine::streams::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Gini-coefficient concentration of order book depth per side.
#[derive(Clone, Debug)]
pub struct LayerConcentration {
    top_n: usize,
    last_gini_bid: f64,
    last_gini_ask: f64,
    last_max: f64,
}

impl LayerConcentration {
    /// Create with the number of price levels to sample on each side.
    pub fn new(top_n: usize) -> Self {
        Self {
            top_n: top_n.max(2),
            last_gini_bid: 0.0,
            last_gini_ask: 0.0,
            last_max: 0.0,
        }
    }

    /// Compute Gini coefficient for a slice of values (sorted ascending internally).
    ///
    /// Formula: G = Σ(2i - n - 1) * x_i / (n * Σx_i), where i is 1-based.
    fn gini(sizes: &[f64]) -> f64 {
        let n = sizes.len();
        if n < 2 {
            return 0.0;
        }
        let sum: f64 = sizes.iter().sum();
        if sum <= 0.0 {
            return 0.0;
        }
        // Sort ascending (copy into stack-allocated small vec)
        let mut sorted: Vec<f64> = sizes.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let weighted_sum: f64 = sorted
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                // i is 0-based; formula uses 1-based i
                let rank = (i + 1) as f64;
                (2.0 * rank - n as f64 - 1.0) * x
            })
            .sum();

        weighted_sum / (n as f64 * sum)
    }
}

impl LayerConcentration {
    /// Gini coefficient of bid-side depth distribution.
    pub fn gini_bid(&self) -> f64 {
        self.last_gini_bid
    }

    /// Gini coefficient of ask-side depth distribution.
    pub fn gini_ask(&self) -> f64 {
        self.last_gini_ask
    }

    /// max(gini_bid, gini_ask).
    pub fn max(&self) -> f64 {
        self.last_max
    }
}

impl Default for LayerConcentration {
    fn default() -> Self {
        Self::new(10)
    }
}

impl OrderBookConsumer for LayerConcentration {
    fn update_orderbook(&mut self, book: &OrderBook) {
        let bid_sizes: Vec<f64> = book.bids.iter().take(self.top_n).map(|l| l.size).collect();
        let ask_sizes: Vec<f64> = book.asks.iter().take(self.top_n).map(|l| l.size).collect();

        self.last_gini_bid = Self::gini(&bid_sizes);
        self.last_gini_ask = Self::gini(&ask_sizes);
        self.last_max = self.last_gini_bid.max(self.last_gini_ask);

    }


    fn reset(&mut self) {
        self.last_gini_bid = 0.0;
        self.last_gini_ask = 0.0;
        self.last_max = 0.0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

/// Typed configuration for [`LayerConcentration`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LayerConcentrationConfig {
    /// Number of price levels to sample on each side.
    pub top_n: crate::contract::Param<usize>,
}

impl Indicator for LayerConcentration {
    const ID: IndicatorId = IndicatorId::LayerConcentration;
    /// L2 order-book indicator — Gini concentration of depth distribution.
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::LayerConcentrationGiniBid),
        Output::percent(IndicatorOutputId::LayerConcentrationGiniAsk),
        Output::percent(IndicatorOutputId::LayerConcentrationMax),
    ];
    /// O(top_n log top_n) per update (sort for Gini). No persistent heap stores.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[]);
    type Config = LayerConcentrationConfig;
    type Runtime = LayerConcentration;

    fn create(cfg: LayerConcentrationConfig) -> LayerConcentration {
        LayerConcentration::new(cfg.top_n.resolved())
    }
}

impl crate::contract::Config for LayerConcentrationConfig {
    fn defaults() -> Self {
        LayerConcentrationConfig { top_n: crate::contract::Param::Solo(10) }
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


impl Render for LayerConcentration {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::LayerConcentrationGiniBid, "Gini Bid", Color::hex(0x4CAF50), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::LayerConcentrationGiniAsk, "Gini Ask", Color::hex(0xF44336), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::LayerConcentrationMax, "Max Gini", Color::hex(0xFF9800), 2.0))
            .bounds(0.0, 1.0)
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
    fn uniform_distribution_gives_low_gini() {
        let mut ind = LayerConcentration::new(4);
        // All levels equal size → Gini = 0
        let bids = [(104.0, 100.0), (103.0, 100.0), (102.0, 100.0), (101.0, 100.0)];
        let asks = [(105.0, 100.0), (106.0, 100.0), (107.0, 100.0), (108.0, 100.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        assert!(ind.gini_bid().abs() < 1e-10, "uniform bid gini should be ~0, got {}", ind.gini_bid());
        assert!(ind.gini_ask().abs() < 1e-10, "uniform ask gini should be ~0, got {}", ind.gini_ask());
    }

    #[test]
    fn concentrated_distribution_gives_high_gini() {
        let mut ind = LayerConcentration::new(4);
        // One level dominates: [1, 1, 1, 1000] → high Gini
        let bids = [(104.0, 1.0), (103.0, 1.0), (102.0, 1.0), (101.0, 1000.0)];
        let asks = [(105.0, 1.0), (106.0, 1.0), (107.0, 1.0), (108.0, 1000.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        assert!(ind.gini_bid() > 0.5, "concentrated bid gini should be high, got {}", ind.gini_bid());
    }

    #[test]
    fn gini_bounded_zero_to_one() {
        let mut ind = LayerConcentration::new(5);
        let bids = [(105.0, 5.0), (104.0, 10.0), (103.0, 2.0), (102.0, 80.0), (101.0, 3.0)];
        let asks = [(106.0, 20.0), (107.0, 1.0), (108.0, 50.0), (109.0, 5.0), (110.0, 24.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        assert!(ind.gini_bid() >= 0.0 && ind.gini_bid() <= 1.0);
        assert!(ind.gini_ask() >= 0.0 && ind.gini_ask() <= 1.0);
        assert!((ind.max() - ind.gini_bid().max(ind.gini_ask())).abs() < 1e-10);
    }

    #[test]
    fn reset_clears_values() {
        let mut ind = LayerConcentration::new(4);
        let bids = [(104.0, 1000.0), (103.0, 1.0), (102.0, 1.0), (101.0, 1.0)];
        let asks = [(105.0, 1.0), (106.0, 1.0), (107.0, 1.0), (108.0, 1.0)];
        ind.update_orderbook(&make_book(&bids, &asks));
        ind.reset();
        assert_eq!(ind.gini_bid(), 0.0);
        assert_eq!(ind.gini_ask(), 0.0);
        assert_eq!(ind.max(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_layer_concentration() {
        let mut f = IndicatorOrder::LayerConcentration(
            <<LayerConcentration as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // Uniform book → Gini = 0
        let bids: Vec<(f64, f64)> = (0..10).map(|i| (100.0 - i as f64, 50.0)).collect();
        let asks: Vec<(f64, f64)> = (0..10).map(|i| (101.0 + i as f64, 50.0)).collect();
        let book = OrderBook::from_tuples(&bids, &asks, 0);
        f.feed(0, MarketSample::OrderBook(&book));
        // Gini of uniform distribution = 0 → factory primary = gini_bid
        assert!(f.primary().abs() < 1e-10);
    }
}
