//! Footprint Chart — volume-by-price breakdown with buy/sell split.
//!
//! Aggregates trade volume at each price level over a bar, splitting into
//! buy-initiated (taker lifted the ask) and sell-initiated (taker hit the bid).
//!
//! Primary path: `update_tick(&Tick)` — uses real `tick.is_buy` flag. Accurate
//! footprint requires a live tick feed with aggressor-side information.
//!
//! Fallback path: `feed(...)` — SYNTHETIC ESTIMATE only.
//! Volume is split proportionally: upper half of bar range → buy, lower half → sell.
//! Delta and totals are approximate; no per-level breakdown is available. Prefer
//! `update_tick` when available.
//!
//! Outputs: `net_delta`, `poc_price`, `total_volume` from last
//! closed bar. Call `close_bar()` to finalize a bar and populate cached metrics.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity, ValueDomain};
use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::types::Tick;
use std::collections::HashMap;

/// Footprint Chart indicator.
///
/// Accumulates ticks for the current bar by price bucket. Call `close_bar()` to
/// snapshot the completed bar (POC, max imbalance, totals) and reset for the next.
#[derive(Debug, Clone)]
pub struct FootprintChart {
    /// Price quantization step. E.g. 0.01 for 1-cent buckets.
    price_bucket: f64,

    /// Current bar levels: bucket_index → (buy_vol, sell_vol).
    levels: HashMap<i64, (f64, f64)>,

    /// Cumulative totals for the in-progress bar.
    total_buy: f64,
    total_sell: f64,

    // ── Cached metrics from the last closed bar ──────────────────────────────
    /// Price level with maximum total volume (Point of Control).
    last_poc_price: f64,
    /// Maximum |buy - sell| / total across all levels, in percent.
    last_max_imbalance_pct: f64,
    /// Price level that had the maximum imbalance.
    last_max_imbalance_price: f64,
    /// total_buy − total_sell for the last closed bar.
    last_net_delta: f64,
    /// total_buy + total_sell for the last closed bar.
    last_total_volume: f64,

    /// Last closed bar's footprint as a `FOOTPRINT_WINDOW × 2` grid: row = price bucket
    /// (centered on the POC bucket), col 0 = buy volume, col 1 = sell volume. The full table
    /// the 3 scalars (poc/net_delta/total) summarize; read via `footprint_grid` /
    /// `ContractFactory::grid(IndicatorOutputId::FootprintChartFootprintGrid)`. Row→price maps as
    /// `poc_price + (row - WINDOW/2) * price_bucket`.
    grid: MatrixGrid,
}

/// Fixed price-window (rows) of the footprint grid, centered on the POC bucket. Buckets beyond
/// the window are clipped — a footprint display is always a bounded price band.
const FOOTPRINT_WINDOW: u16 = 64;

/// Column names for the footprint grid: index 0 = buy volume, index 1 = sell volume.
const FOOTPRINT_SIDES: &[&str] = &["buy", "sell"];

impl FootprintChart {
    /// `price_bucket`: price-level quantization step (e.g. 0.01 for cents, 1.0 for integer ticks).
    pub fn new(price_bucket: f64) -> Self {
        Self {
            price_bucket: price_bucket.max(1e-9),
            levels: HashMap::new(),
            total_buy: 0.0,
            total_sell: 0.0,
            last_poc_price: 0.0,
            last_max_imbalance_pct: 0.0,
            last_max_imbalance_price: 0.0,
            last_net_delta: 0.0,
            last_total_volume: 0.0,
            grid: MatrixGrid::new(FOOTPRINT_WINDOW, 2, false)
                .with_labels(AxisLabels::Values(Vec::new()), AxisLabels::Named(FOOTPRINT_SIDES)),
        }
    }

    /// Add a real trade tick to the current bar accumulation.
    pub fn update_tick(&mut self, tick: &Tick) {
        let bucket = (tick.price / self.price_bucket).floor() as i64;
        let entry = self.levels.entry(bucket).or_insert((0.0, 0.0));
        if tick.is_buy {
            entry.0 += tick.size;
            self.total_buy += tick.size;
        } else {
            entry.1 += tick.size;
            self.total_sell += tick.size;
        }
        // Eagerly update cached totals so is_ready() returns true mid-bar
        self.last_total_volume = self.total_buy + self.total_sell;
        self.last_net_delta = self.total_buy - self.total_sell;

    }

    /// SYNTHETIC ESTIMATE: split bar volume across OHLCV price points.
    ///
    /// Volume above mid-price → buy, below → sell. No per-level breakdown is
    /// computed; only aggregate totals are updated. Prefer `update_tick`.
    pub fn feed(&mut self, lanes: &[f64]) {
        let h = lanes[0];
        let l = lanes[1];
        let c = lanes[2];
        let v = lanes[3];
        let mid = (h + l) / 2.0;
        let buy_frac = if h > l { (c - l) / (h - l) } else { 0.5 };
        let buy_vol = v * buy_frac;
        let sell_vol = v * (1.0 - buy_frac);
        let _ = mid; // unused but kept for readability

        // Synthetic single bucket at close price
        let bucket = (c / self.price_bucket).floor() as i64;
        let entry = self.levels.entry(bucket).or_insert((0.0, 0.0));
        entry.0 += buy_vol;
        entry.1 += sell_vol;
        self.total_buy += buy_vol;
        self.total_sell += sell_vol;


    }

    /// Finalize current bar: compute POC and max imbalance, then reset accumulators.
    pub fn close_bar(&mut self) {
        if self.levels.is_empty() {
            return;
        }

        let mut poc_bucket = 0i64;
        let mut poc_vol = 0.0f64;
        let mut max_imb_pct = 0.0f64;
        let mut max_imb_bucket = 0i64;

        for (&bucket, &(buy, sell)) in &self.levels {
            let total = buy + sell;
            if total > poc_vol {
                poc_vol = total;
                poc_bucket = bucket;
            }
            if total > 0.0 {
                let imb_pct = ((buy - sell).abs() / total) * 100.0;
                if imb_pct > max_imb_pct {
                    max_imb_pct = imb_pct;
                    max_imb_bucket = bucket;
                }
            }
        }

        self.last_poc_price = poc_bucket as f64 * self.price_bucket;
        self.last_max_imbalance_pct = max_imb_pct;
        self.last_max_imbalance_price = max_imb_bucket as f64 * self.price_bucket;
        self.last_net_delta = self.total_buy - self.total_sell;
        self.last_total_volume = self.total_buy + self.total_sell;

        // Snapshot the bar's footprint into the fixed price-window grid (rows centered on the
        // POC bucket) before the level map is cleared.
        self.grid.reset();
        let half = (FOOTPRINT_WINDOW / 2) as i64;
        let mut row_prices = Vec::with_capacity(FOOTPRINT_WINDOW as usize);
        for row in 0..FOOTPRINT_WINDOW {
            let bucket = poc_bucket + (row as i64 - half);
            row_prices.push(bucket as f64 * self.price_bucket);
            if let Some(&(buy, sell)) = self.levels.get(&bucket) {
                self.grid.set_direction(MatrixCell::new(row, 0), buy);
                self.grid.set_direction(MatrixCell::new(row, 1), sell);
            }
        }
        self.grid.set_row_values(&row_prices);

        self.levels.clear();
        self.total_buy = 0.0;
        self.total_sell = 0.0;
    }

    // ── Accessors ────────────────────────────────────────────────────────────

    /// Price level with the most volume in the last closed bar.
    pub fn poc_price(&self) -> f64 { self.last_poc_price }

    /// Maximum imbalance percent across all levels in the last closed bar.
    pub fn max_imbalance_pct(&self) -> f64 { self.last_max_imbalance_pct }

    /// Price level that had the maximum imbalance in the last closed bar.
    pub fn max_imbalance_price(&self) -> f64 { self.last_max_imbalance_price }

    /// Net delta (total_buy − total_sell) from the last closed bar.
    pub fn net_delta(&self) -> f64 { self.last_net_delta }

    /// Total volume (total_buy + total_sell) from the last closed bar.
    pub fn total_volume(&self) -> f64 { self.last_total_volume }

    /// In-progress levels map: bucket_index → (buy_vol, sell_vol).
    pub fn current_levels(&self) -> &HashMap<i64, (f64, f64)> { &self.levels }

    /// In-progress buy accumulator.
    pub fn total_buy(&self) -> f64 { self.total_buy }

    /// In-progress sell accumulator.
    pub fn total_sell(&self) -> f64 { self.total_sell }

    /// The last closed bar's footprint table: `FOOTPRINT_WINDOW × 2` (price bucket × {buy,
    /// sell} volume), centered on the POC. The matrix output behind
    /// `IndicatorOutputId::FootprintChartFootprintGrid`.
    pub fn footprint_grid(&self) -> &MatrixGrid { &self.grid }


    pub fn is_ready(&self) -> bool {
        self.last_total_volume > 0.0
    }

    pub fn reset(&mut self) {
        self.levels.clear();
        self.total_buy = 0.0;
        self.total_sell = 0.0;
        self.last_poc_price = 0.0;
        self.last_max_imbalance_pct = 0.0;
        self.last_max_imbalance_price = 0.0;
        self.last_net_delta = 0.0;
        self.last_total_volume = 0.0;
        self.grid.reset();
    }
}

impl TickConsumer for FootprintChart {
    fn update_tick(&mut self, tick: &Tick) {
        FootprintChart::update_tick(self, tick)
    }
    fn reset(&mut self) { FootprintChart::reset(self) }
    fn is_ready(&self) -> bool { FootprintChart::is_ready(self) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buy_tick(price: f64, qty: f64) -> Tick {
        Tick::new(0, price, qty, true)
    }

    fn sell_tick(price: f64, qty: f64) -> Tick {
        Tick::new(0, price, qty, false)
    }

    #[test]
    fn test_footprint_creation() {
        let fp = FootprintChart::new(0.01);
        assert!(!fp.is_ready());
        assert_eq!(fp.net_delta(), 0.0);
    }

    #[test]
    fn test_accumulate_and_close_bar() {
        let mut fp = FootprintChart::new(1.0);

        // 5 buy ticks @ price 100, qty 10 each → 50 buy at bucket 100
        for _ in 0..5 {
            fp.update_tick(&buy_tick(100.0, 10.0));
        }
        // 3 sell ticks @ price 101, qty 5 each → 15 sell at bucket 101
        for _ in 0..3 {
            fp.update_tick(&sell_tick(101.0, 5.0));
        }

        fp.close_bar();

        assert!((fp.net_delta() - 35.0).abs() < 1e-9, "net delta should be 50-15=35");
        assert_eq!(fp.poc_price(), 100.0, "POC should be at price 100 (50 vol > 15 vol)");
        assert!((fp.total_volume() - 65.0).abs() < 1e-9);
    }

    #[test]
    fn test_max_imbalance_is_100_pct_for_pure_buy() {
        let mut fp = FootprintChart::new(1.0);
        fp.update_tick(&buy_tick(100.0, 20.0));
        fp.close_bar();
        assert!((fp.max_imbalance_pct() - 100.0).abs() < 1e-9);
        assert_eq!(fp.max_imbalance_price(), 100.0);
    }

    #[test]
    fn test_footprint_bar_synthetic() {
        let mut fp = FootprintChart::new(1.0);
        // bullish bar: close above mid → buy > sell
        fp.feed(&[110.0, 90.0, 108.0, 400.0]);
        assert!(fp.total_buy() > fp.total_sell());
    }

    #[test]
    fn footprint_grid_captures_buy_sell_around_poc() {
        let mut fp = FootprintChart::new(1.0);
        // 50 buy at bucket 100 (the POC), 15 sell one bucket above at 101.
        for _ in 0..5 { fp.update_tick(&buy_tick(100.0, 10.0)); }
        for _ in 0..3 { fp.update_tick(&sell_tick(101.0, 5.0)); }
        fp.close_bar();

        let g = fp.footprint_grid();
        assert_eq!(g.cols(), 2, "buy / sell columns");
        let center = FOOTPRINT_WINDOW / 2;
        // POC bucket sits at the center row; its buy volume is in col 0.
        assert_eq!(g.read_direction(MatrixCell::new(center, 0)), 50.0);
        // Bucket 101 = one above the POC -> next row up; its sell volume is in col 1.
        assert_eq!(g.read_direction(MatrixCell::new(center + 1, 1)), 15.0);
        // A bar with no activity at a far row stays zero.
        assert_eq!(g.read_direction(MatrixCell::new(0, 0)), 0.0);
    }

    #[test]
    fn footprint_grid_axis_labels() {
        use crate::engine::matrix_grid::Label;
        let mut fp = FootprintChart::new(1.0);
        for _ in 0..5 { fp.update_tick(&buy_tick(100.0, 10.0)); }
        for _ in 0..3 { fp.update_tick(&sell_tick(101.0, 5.0)); }
        fp.close_bar();

        let g = fp.footprint_grid();
        // Col 0 → "buy", col 1 → "sell"
        assert_eq!(g.col_label(0), Label::Name("buy"));
        assert_eq!(g.col_label(1), Label::Name("sell"));
        // POC bucket = 100; center row = FOOTPRINT_WINDOW/2; its price label = 100.0
        let center = FOOTPRINT_WINDOW / 2;
        assert_eq!(g.row_label(center), Label::Value(100.0));
    }

    #[test]
    fn test_footprint_reset() {
        let mut fp = FootprintChart::new(1.0);
        fp.update_tick(&buy_tick(100.0, 10.0));
        fp.close_bar();
        fp.reset();
        assert!(!fp.is_ready());
        assert_eq!(fp.net_delta(), 0.0);
    }

    #[test]
    fn test_close_bar_clears_in_progress() {
        let mut fp = FootprintChart::new(1.0);
        fp.update_tick(&buy_tick(100.0, 10.0));
        assert!(!fp.current_levels().is_empty());
        fp.close_bar();
        assert!(fp.current_levels().is_empty());
        assert!(fp.is_ready()); // last_total_volume is populated
    }
}

impl Default for FootprintChart {
    /// Factory default: price_bucket=0.01.
    fn default() -> Self {
        Self::new(0.01)
    }
}

/// Typed dual-mode config for [`FootprintChart`]: price-level quantization step.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FootprintChartConfig {
    /// Price quantization step (e.g. 0.01 for 1-cent buckets).
    pub price_bucket: Param<f64>,
}

impl Indicator for FootprintChart {
    const ID: IndicatorId = IndicatorId::FootprintChart;
    /// Not a pluggable family — a tick-stream cluster/footprint producer.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::FootprintChartNetDelta),
        Output::price(IndicatorOutputId::FootprintChartPocPrice),
        Output::count(IndicatorOutputId::FootprintChartTotalVolume),
        Output::matrix(IndicatorOutputId::FootprintChartFootprintGrid, ValueDomain::Count),
    ];
    /// O(1) per tick; uses a HashMap (Vec-class) for the level buckets.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Vec)]);
    type Config = FootprintChartConfig;
    type Runtime = Self;

    fn create(cfg: FootprintChartConfig) -> Self {
        Self::new(cfg.price_bucket.resolved())
    }
}

impl crate::contract::Config for FootprintChartConfig {
    fn defaults() -> Self {
        FootprintChartConfig { price_bucket: Param::Solo(0.01) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // price_bucket: Class I (instrument-relative price granularity) — PIN.
        // f64 auto already leaves it Solo; no override needed.
        Self::machine_defaults_auto()
    }
}


impl Render for FootprintChart {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::FootprintChartNetDelta, "Net Delta", Color::hex(0x4CAF50))
            .output(RenderOutput::line(IndicatorOutputId::FootprintChartPocPrice, "POC Price", Color::hex(0xFF9800), 2.0))
            .output(RenderOutput::histogram(IndicatorOutputId::FootprintChartTotalVolume, "Total Volume", Color::hex(0x2196F3)))
            .histogram_style(HistogramStyle::FromBottom)
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;
    use crate::core::types::Tick as CoreTick;

    #[test]
    fn factory_feeds_resolved_tick() {
        let mut f = IndicatorOrder::FootprintChart(
            <<FootprintChart as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();

        let t1 = CoreTick::new(0, 100.0, 10.0, true);
        let t2 = CoreTick::new(0, 100.0, 5.0, false);
        f.feed(0, MarketSample::Tick(&t1));
        f.feed(0, MarketSample::Tick(&t2));

        // net_delta = buy - sell = 10 - 5 = 5
        let v = f.primary();
        assert!((v - 5.0).abs() < 1e-9, "net_delta should be 5.0, got {}", v);
    }

    #[test]
    fn factory_exposes_footprint_matrix() {
        use crate::engine::indicator_id::IndicatorId;
        // The matrix output is reachable through the universe via the F1 accessor.
        let f = IndicatorOrder::FootprintChart(
            <<FootprintChart as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let g = f
            .grid(IndicatorOutputId::FootprintChartFootprintGrid)
            .expect("FootprintChart emits the Footprint matrix");
        assert_eq!(g.rows(), FOOTPRINT_WINDOW);
        assert_eq!(g.cols(), 2);

        // A scalar-only producer returns None for a foreign matrix id.
        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma).unwrap().build_solo().unwrap();
        assert!(sma.grid(IndicatorOutputId::FootprintChartFootprintGrid).is_none());
    }
}
