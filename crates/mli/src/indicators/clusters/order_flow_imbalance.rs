//! Order Flow Imbalance — buy/sell pressure from L2 orderbook deltas.
//!
//! Primary path: `update_orderbook(&OrderBook)` — computes real OFI by comparing
//! current bid/ask depth against the previous snapshot.
//!
//! Fallback path: `update_volume_bar(&Bar)` — SYNTHETIC ESTIMATE only. Uses
//! close > open heuristic to split volume into buy/sell. Accuracy is limited;
//! prefer the L2 path when orderbook snapshots are available.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::order_book_consumer::OrderBookConsumer;
use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity, ValueDomain};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::core::types::OrderBook;
use crate::engine::stream_kind::StreamKind;
use crate::types::Bar;
use std::collections::HashMap;

/// Fixed price-window (rows) of the OFI per-level imbalance vector, centered on the
/// strongest imbalance level.
const OFI_WINDOW: u16 = 64;

/// One price level with accumulated buy/sell volume split.
#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub total_volume: f64,
    /// (buy - sell) / total, range [-1, 1].
    pub imbalance: f64,
    /// buy / sell ratio (inf when sell == 0 and buy > 0).
    pub imbalance_ratio: f64,
}

impl PriceLevel {
    pub fn new(price: f64) -> Self {
        Self {
            price,
            buy_volume: 0.0,
            sell_volume: 0.0,
            total_volume: 0.0,
            imbalance: 0.0,
            imbalance_ratio: 1.0,
        }
    }

    pub fn add_volume(&mut self, volume: f64, is_buy: bool) {
        if is_buy {
            self.buy_volume += volume;
        } else {
            self.sell_volume += volume;
        }
        self.total_volume += volume;
        self.update_metrics();
    }

    fn update_metrics(&mut self) {
        if self.total_volume > 0.0 {
            self.imbalance = (self.buy_volume - self.sell_volume) / self.total_volume;
        }
        self.imbalance_ratio = if self.sell_volume > 0.0 {
            self.buy_volume / self.sell_volume
        } else if self.buy_volume > 0.0 {
            f64::INFINITY
        } else {
            1.0
        };
    }
}

/// Order Flow Imbalance indicator.
#[derive(Debug, Clone)]
pub struct OrderFlowImbalance {
    period: usize,
    tick_size: f64,

    // OHLCV-path state
    volume_bars: Vec<Bar>,
    price_levels: HashMap<i64, PriceLevel>,

    // L2-path state: depth snapshots for OFI delta
    prev_bid_depth: f64,
    prev_ask_depth: f64,
    l2_depth_levels: usize,

    // Metrics (shared output)
    total_imbalance: f64,
    avg_imbalance: f64,
    max_imbalance: f64,
    min_imbalance: f64,
    dominant_side: i8,
    imbalance_strength: f64,
    flow_acceleration: f64,
    prev_imbalance: f64,

    max_buy_level: Option<PriceLevel>,
    max_sell_level: Option<PriceLevel>,
    strongest_imbalance_level: Option<PriceLevel>,

    /// Number of `update_orderbook` calls received on the L2 path.
    l2_updates: usize,

    /// Signed bid−ask imbalance per price level: `OFI_WINDOW × 1` centered on the strongest
    /// imbalance level (OHLCV path only; zeroed on the pure L2 path where no per-level map exists).
    grid: MatrixGrid,
}

/// Typed dual-mode config for [`OrderFlowImbalance`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct OrderFlowImbalanceConfig {
    pub period: Param<usize>,
    pub tick_size: Param<f64>,
}

impl OrderFlowImbalance {
    pub fn new(period: usize, tick_size: f64) -> Self {
        Self {
            period,
            tick_size,
            volume_bars: Vec::with_capacity(period),
            price_levels: HashMap::new(),
            prev_bid_depth: 0.0,
            prev_ask_depth: 0.0,
            l2_depth_levels: 10,
            total_imbalance: 0.0,
            avg_imbalance: 0.0,
            max_imbalance: 0.0,
            min_imbalance: 0.0,
            dominant_side: 0,
            imbalance_strength: 0.0,
            flow_acceleration: 0.0,
            prev_imbalance: 0.0,
            max_buy_level: None,
            max_sell_level: None,
            strongest_imbalance_level: None,
            l2_updates: 0,
            grid: MatrixGrid::new(OFI_WINDOW, 1, false)
                .with_labels(AxisLabels::Values(Vec::new()), AxisLabels::Bars),
        }
    }

    // -------------------------------------------------------------------------
    // L2 path helpers
    // -------------------------------------------------------------------------

    /// Standard OFI formula:
    ///   Δbid = current_bid_depth - prev_bid_depth  (positive → buy pressure)
    ///   Δask = current_ask_depth - prev_ask_depth  (positive → sell pressure)
    ///   OFI  = Δbid - Δask, normalised to [-1, 1] by total depth change.
    fn apply_l2_ofi(&mut self, bid_depth: f64, ask_depth: f64) {
        let delta_bid = bid_depth - self.prev_bid_depth;
        let delta_ask = ask_depth - self.prev_ask_depth;
        let raw_ofi = delta_bid - delta_ask;
        let normaliser = (delta_bid.abs() + delta_ask.abs()).max(1e-12);
        let ofi = raw_ofi / normaliser;

        // Blend into rolling avg_imbalance
        let old = self.avg_imbalance;
        self.avg_imbalance = ofi;
        self.total_imbalance = ofi;
        self.flow_acceleration = self.avg_imbalance - old;
        self.prev_imbalance = old;

        self.dominant_side = if ofi > 0.1 {
            1
        } else if ofi < -0.1 {
            -1
        } else {
            0
        };
        self.imbalance_strength = ofi.abs();

        self.prev_bid_depth = bid_depth;
        self.prev_ask_depth = ask_depth;
    }

    // -------------------------------------------------------------------------
    // OHLCV path helpers (synthetic — kept for pipelines without L2)
    // -------------------------------------------------------------------------

    pub fn update_volume_bar(&mut self, volume_bar: &Bar) -> f64 {
        if self.volume_bars.len() >= self.period {
            self.volume_bars.remove(0);
        }
        self.volume_bars.push(*volume_bar);
        self.analyze_price_levels(volume_bar);
        self.recalculate_metrics();
        self.total_imbalance
    }

    fn analyze_price_levels(&mut self, volume_bar: &Bar) {
        if self.volume_bars.len() >= self.period {
            self.price_levels.clear();
            let bars_to_process = self.volume_bars.clone();
            for bar in &bars_to_process {
                self.process_bar_levels(bar);
            }
        } else {
            self.process_bar_levels(volume_bar);
        }
    }

    /// SYNTHETIC: price above mid → buy volume, below mid → sell volume.
    fn process_bar_levels(&mut self, volume_bar: &Bar) {
        let prices = [volume_bar.open, volume_bar.high, volume_bar.low, volume_bar.close];
        let volume_per_price = volume_bar.volume / 4.0;
        let mid = (volume_bar.open + volume_bar.close) / 2.0;

        for price in &prices {
            let price_key = self.price_to_key(*price);
            // SYNTHETIC ESTIMATE: price >= mid → buy side
            let is_buy = *price >= mid;
            let level = self.price_levels.entry(price_key).or_insert_with(|| PriceLevel::new(*price));
            level.add_volume(volume_per_price, is_buy);
        }
    }

    fn price_to_key(&self, price: f64) -> i64 {
        (price / self.tick_size).round() as i64
    }

    fn recalculate_metrics(&mut self) {
        if self.price_levels.is_empty() {
            return;
        }

        let mut total_imbalance = 0.0;
        let mut max_imbalance = f64::NEG_INFINITY;
        let mut min_imbalance = f64::INFINITY;
        let mut max_buy_volume = 0.0f64;
        let mut max_sell_volume = 0.0f64;
        let mut strongest_imbalance = 0.0f64;

        self.max_buy_level = None;
        self.max_sell_level = None;
        self.strongest_imbalance_level = None;

        for level in self.price_levels.values() {
            total_imbalance += level.imbalance;

            if level.imbalance > max_imbalance {
                max_imbalance = level.imbalance;
            }
            if level.imbalance < min_imbalance {
                min_imbalance = level.imbalance;
            }

            if level.buy_volume > max_buy_volume {
                max_buy_volume = level.buy_volume;
                self.max_buy_level = Some(level.clone());
            }
            if level.sell_volume > max_sell_volume {
                max_sell_volume = level.sell_volume;
                self.max_sell_level = Some(level.clone());
            }
            if level.imbalance.abs() > strongest_imbalance {
                strongest_imbalance = level.imbalance.abs();
                self.strongest_imbalance_level = Some(level.clone());
            }
        }

        self.total_imbalance = total_imbalance;
        self.avg_imbalance = total_imbalance / self.price_levels.len() as f64;
        self.max_imbalance = max_imbalance;
        self.min_imbalance = min_imbalance;

        self.dominant_side = if self.avg_imbalance > 0.1 {
            1
        } else if self.avg_imbalance < -0.1 {
            -1
        } else {
            0
        };
        self.imbalance_strength = self.avg_imbalance.abs();
        self.flow_acceleration = self.avg_imbalance - self.prev_imbalance;
        self.prev_imbalance = self.avg_imbalance;

        // Snapshot signed bid−ask imbalance (buy − sell) per level into the profile grid,
        // centered on the strongest imbalance level's price key.
        if let Some(ref anchor) = self.strongest_imbalance_level {
            let anchor_key = self.price_to_key(anchor.price);
            self.grid.reset();
            let half = (OFI_WINDOW / 2) as i64;
            // Build a temporary price-key → (buy, sell) lookup from the live levels.
            let key_map: HashMap<i64, (f64, f64)> = self
                .price_levels
                .iter()
                .map(|(&k, lvl)| (k, (lvl.buy_volume, lvl.sell_volume)))
                .collect();
            let mut row_prices = Vec::with_capacity(OFI_WINDOW as usize);
            for row in 0..OFI_WINDOW {
                let key = anchor_key + (row as i64 - half);
                row_prices.push(key as f64 * self.tick_size);
                if let Some(&(buy, sell)) = key_map.get(&key) {
                    self.grid.set_direction(MatrixCell::new(row, 0), buy - sell);
                }
            }
            self.grid.set_row_values(&row_prices);
        }
    }

    // -------------------------------------------------------------------------
    // Public accessors
    // -------------------------------------------------------------------------

    pub fn total_imbalance(&self) -> f64 { self.total_imbalance }
    pub fn avg_imbalance(&self) -> f64 { self.avg_imbalance }
    pub fn dominant_side(&self) -> i8 { self.dominant_side }
    pub fn imbalance_strength(&self) -> f64 { self.imbalance_strength }
    pub fn flow_acceleration(&self) -> f64 { self.flow_acceleration }
    pub fn max_buy_level(&self) -> Option<&PriceLevel> { self.max_buy_level.as_ref() }
    pub fn max_sell_level(&self) -> Option<&PriceLevel> { self.max_sell_level.as_ref() }
    pub fn strongest_imbalance_level(&self) -> Option<&PriceLevel> { self.strongest_imbalance_level.as_ref() }
    pub fn price_levels_count(&self) -> usize { self.price_levels.len() }

    pub fn flow_state(&self) -> &'static str {
        match (self.dominant_side, self.imbalance_strength) {
            (1, s) if s > 0.5 => "Strong Buy Flow",
            (1, s) if s > 0.2 => "Moderate Buy Flow",
            (-1, s) if s > 0.5 => "Strong Sell Flow",
            (-1, s) if s > 0.2 => "Moderate Sell Flow",
            (0, s) if s < 0.1 => "Balanced Flow",
            _ => "Weak Flow",
        }
    }

    pub fn analysis_quality(&self) -> &'static str {
        match self.price_levels.len() {
            n if n >= 20 => "Excellent",
            n if n >= 10 => "Good",
            n if n >= 5 => "Fair",
            _ => "Poor",
        }
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.total_imbalance
    }

    pub fn is_ready(&self) -> bool {
        (self.volume_bars.len() >= (self.period / 2).max(1) && !self.price_levels.is_empty())
            || self.l2_updates >= 5
    }

    pub fn reset(&mut self) {
        self.volume_bars.clear();
        self.price_levels.clear();
        self.prev_bid_depth = 0.0;
        self.prev_ask_depth = 0.0;
        self.total_imbalance = 0.0;
        self.avg_imbalance = 0.0;
        self.max_imbalance = 0.0;
        self.min_imbalance = 0.0;
        self.dominant_side = 0;
        self.imbalance_strength = 0.0;
        self.flow_acceleration = 0.0;
        self.prev_imbalance = 0.0;
        self.max_buy_level = None;
        self.max_sell_level = None;
        self.strongest_imbalance_level = None;
        self.l2_updates = 0;
        self.grid.reset();
    }

    /// Signed bid−ask imbalance per price level: `OFI_WINDOW × 1` centered on the strongest
    /// imbalance level (populated on the OHLCV path; zeroed on the pure L2 path).
    /// The matrix output behind `IndicatorOutputId::OrderFlowImbProfileGrid`.
    pub fn profile_grid(&self) -> &MatrixGrid { &self.grid }
}

impl Default for OrderFlowImbalance {
    /// Factory default: period=14, tick_size=0.01.
    fn default() -> Self {
        Self::new(14, 0.01)
    }
}

impl Indicator for OrderFlowImbalance {
    const ID: IndicatorId = IndicatorId::OrderFlowImb;
    /// L2 orderbook consumer — pluggable in the OrderBook family.
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::OrderFlowImb),
        Output::matrix(IndicatorOutputId::OrderFlowImbProfileGrid, ValueDomain::Centered),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[
        Store::window(StoreKind::Vec), // volume_bars
    ]);
    type Config = OrderFlowImbalanceConfig;
    type Runtime = OrderFlowImbalance;

    fn create(cfg: OrderFlowImbalanceConfig) -> OrderFlowImbalance {
        OrderFlowImbalance::new(cfg.period.resolved(), cfg.tick_size.resolved())
    }
}

impl OrderBookConsumer for OrderFlowImbalance {
    /// Real OFI from L2 deltas. Compares current bid/ask depth against
    /// previous snapshot. Δbid - Δask normalised to [-1, 1].
    fn update_orderbook(&mut self, book: &OrderBook) {
        let bid_depth = book.bid_depth(self.l2_depth_levels);
        let ask_depth = book.ask_depth(self.l2_depth_levels);
        self.apply_l2_ofi(bid_depth, ask_depth);
        self.l2_updates += 1;
    }

    fn reset(&mut self) { self.reset() }
    fn is_ready(&self) -> bool { self.is_ready() }
}

impl crate::contract::Config for OrderFlowImbalanceConfig {
    fn defaults() -> Self {
        OrderFlowImbalanceConfig { period: Param::Solo(14), tick_size: Param::Solo(0.01) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // period: Class A period usize → auto range(2,4048,1) — correct, no override.
        // tick_size: Class I (instrument-relative price granularity) — PIN.
        // f64 auto already leaves it Solo; restore to make intent explicit.
        s.tick_size = Self::defaults().tick_size;
        s
    }
}


impl Render for OrderFlowImbalance {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(IndicatorOutputId::OrderFlowImb, "Order Flow Imb", Color::hex(0x00BCD4)))
            .zero_baseline()
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
    fn test_order_flow_imbalance_creation() {
        let ind = OrderFlowImbalance::new(20, 0.01);
        assert!(!ind.is_ready());
        assert_eq!(ind.total_imbalance(), 0.0);
    }

    #[test]
    fn test_order_flow_imbalance_warmup() {
        let mut ind = OrderFlowImbalance::new(10, 0.01);
        for i in 0..15 {
            let bar = Bar {
                time: i as i64,
                open: 100.0 + (i as f64 * 0.1).sin(),
                high: 101.0 + (i as f64 * 0.1).sin(),
                low: 99.0 + (i as f64 * 0.1).sin(),
                close: 100.5 + (i as f64 * 0.1).sin(),
                volume: 1000.0 + i as f64 * 10.0,
            };
            ind.update_volume_bar(&bar);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_order_flow_imbalance_values() {
        let mut ind = OrderFlowImbalance::new(10, 0.01);
        for i in 0..20 {
            let bar = Bar {
                time: i as i64,
                open: 100.0,
                high: 102.0,
                low: 98.0,
                close: 101.0,
                volume: 1000.0,
            };
            ind.update_volume_bar(&bar);
        }
        assert!(ind.total_imbalance().is_finite());
        let side = ind.dominant_side();
        assert!(side >= -1 && side <= 1);
    }

    #[test]
    fn test_order_flow_imbalance_reset() {
        let mut ind = OrderFlowImbalance::new(10, 0.01);
        for i in 0..15 {
            let bar = Bar {
                time: i as i64,
                open: 100.0,
                high: 102.0,
                low: 98.0,
                close: 101.0,
                volume: 1000.0,
            };
            ind.update_volume_bar(&bar);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.total_imbalance(), 0.0);
    }

    #[test]
    fn test_ofi_l2_buy_pressure() {
        let mut ind = OrderFlowImbalance::new(10, 0.01);
        // First snapshot: balanced
        let book1 = OrderBook::from_tuples(
            &[(100.0, 10.0), (99.0, 8.0)],
            &[(101.0, 10.0), (102.0, 8.0)],
            1000,
        );
        ind.update_orderbook(&book1);

        // Second snapshot: bids grew → buy pressure
        let book2 = OrderBook::from_tuples(
            &[(100.0, 20.0), (99.0, 16.0)], // bid_depth doubled
            &[(101.0, 10.0), (102.0, 8.0)],
            2000,
        );
        ind.update_orderbook(&book2);
        assert!(ind.value() > 0.0, "bid growth should produce positive OFI");
    }

    #[test]
    fn test_ofi_l2_sell_pressure() {
        let mut ind = OrderFlowImbalance::new(10, 0.01);
        let book1 = OrderBook::from_tuples(
            &[(100.0, 10.0), (99.0, 8.0)],
            &[(101.0, 10.0), (102.0, 8.0)],
            1000,
        );
        ind.update_orderbook(&book1);

        // asks grew → sell pressure
        let book2 = OrderBook::from_tuples(
            &[(100.0, 10.0), (99.0, 8.0)],
            &[(101.0, 20.0), (102.0, 16.0)],
            2000,
        );
        ind.update_orderbook(&book2);
        assert!(ind.value() < 0.0, "ask growth should produce negative OFI");
    }

    #[test]
    fn factory_feeds_resolved_order_flow_imb() {
        
        let mut f = IndicatorOrder::OrderFlowImb(
            <<OrderFlowImbalance as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let book = OrderBook::from_tuples(
            &[(100.0, 10.0), (99.0, 8.0)],
            &[(101.0, 10.0), (102.0, 8.0)],
            1000,
        );
        f.feed(0, MarketSample::OrderBook(&book));
        let v = f.primary();
        assert!(v.is_finite());
    }

    #[test]
    fn ofi_profile_row_labels_are_prices() {
        use crate::engine::matrix_grid::Label;
        let mut ind = OrderFlowImbalance::new(5, 1.0);
        for i in 0..5 {
            let bar = Bar {
                time: i as i64,
                open: 100.0,
                high: 102.0,
                low: 98.0,
                close: 101.0,
                volume: 1000.0,
            };
            ind.update_volume_bar(&bar);
        }
        let g = ind.profile_grid();
        let center = OFI_WINDOW / 2;
        // Center row must be a valued price label (not Index fallback)
        match g.row_label(center) {
            Label::Value(_) => {}
            other => panic!("expected Label::Value at center row, got {:?}", other),
        }
    }

    #[test]
    fn profile_grid_shape_and_content() {
        let mut ind = OrderFlowImbalance::new(5, 1.0);
        // Feed bars so the OHLCV path populates price_levels and recalculate_metrics runs.
        for i in 0..5 {
            let bar = Bar {
                time: i as i64,
                open: 100.0,
                high: 102.0,
                low: 98.0,
                close: 101.0,
                volume: 1000.0,
            };
            ind.update_volume_bar(&bar);
        }
        let g = ind.profile_grid();
        assert_eq!(g.rows(), OFI_WINDOW, "64 price rows");
        assert_eq!(g.cols(), 1, "single imbalance column");
        // Grid must be a valid finite slice — spot-check center row.
        let center = OFI_WINDOW / 2;
        assert!(g.read_direction(MatrixCell::new(center, 0)).is_finite());
    }

    #[test]
    fn factory_exposes_order_flow_profile_vector() {
        let f = IndicatorOrder::OrderFlowImb(
            <<OrderFlowImbalance as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let g = f
            .grid(IndicatorOutputId::OrderFlowImbProfileGrid)
            .expect("OrderFlowImbalance emits the OrderFlowProfile vector");
        assert_eq!(g.rows(), 64);
        assert_eq!(g.cols(), 1);

        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma).unwrap().build_solo().unwrap();
        assert!(sma.grid(IndicatorOutputId::OrderFlowImbProfileGrid).is_none());
    }
}
