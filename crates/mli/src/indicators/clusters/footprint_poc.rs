//! Footprint POC — Point of Control for the current bar.
//!
//! Tracks total volume per price bucket and outputs the price level with the
//! maximum accumulated volume after `close_bar()`.
//!
//! Output: `poc_price` where `poc_price` is the bucket
//! mid-point (bucket_index * price_bucket) of the highest-volume level.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity, ValueDomain};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::types::Tick;
use std::collections::HashMap;

/// Fixed price-window (rows) of the POC volume-profile vector, centered on the POC bucket.
const POC_WINDOW: u16 = 64;

/// Footprint POC — reports the price level with maximum volume per bar.
#[derive(Debug, Clone)]
pub struct FootprintPoc {
    price_bucket: f64,
    /// In-progress bar: bucket_index → total volume.
    levels: HashMap<i64, f64>,
    /// POC price from the last closed bar.
    last_poc: f64,
    /// Volume-by-price profile: `POC_WINDOW × 1` vector centered on the POC bucket.
    /// Row → price: `last_poc + (row - WINDOW/2) * price_bucket`.
    grid: MatrixGrid,
}

impl FootprintPoc {
    /// `price_bucket`: price-level quantization step (e.g. 0.01, 1.0).
    pub fn new(price_bucket: f64) -> Self {
        Self {
            price_bucket: price_bucket.max(1e-9),
            levels: HashMap::new(),
            last_poc: 0.0,
            grid: MatrixGrid::new(POC_WINDOW, 1, false)
                .with_labels(AxisLabels::Values(Vec::new()), AxisLabels::Bars),
        }
    }

    /// Accumulate one tick into the in-progress bar.
    ///
    /// Eagerly updates `last_poc` from the live in-bar levels so that
    /// `is_ready()` returns true after the first tick without waiting for
    /// `close_bar()`. `close_bar()` still finalises and resets the bar.
    pub fn update_tick(&mut self, tick: &Tick) {
        let bucket = (tick.price / self.price_bucket).floor() as i64;
        *self.levels.entry(bucket).or_insert(0.0) += tick.size;
        // Update live POC so is_ready() flips true mid-bar
        if let Some((&poc_bucket, _)) = self
            .levels
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            self.last_poc = poc_bucket as f64 * self.price_bucket;
        }
    }

    /// Finalize bar: compute POC, snapshot the volume-profile grid, and reset accumulation.
    pub fn close_bar(&mut self) {
        if let Some((&poc_bucket, _)) = self
            .levels
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            self.last_poc = poc_bucket as f64 * self.price_bucket;

            // Snapshot volume-by-price into the profile vector centered on the POC bucket.
            self.grid.reset();
            let half = (POC_WINDOW / 2) as i64;
            let mut row_prices = Vec::with_capacity(POC_WINDOW as usize);
            for row in 0..POC_WINDOW {
                let key = poc_bucket + (row as i64 - half);
                row_prices.push(key as f64 * self.price_bucket);
                if let Some(&vol) = self.levels.get(&key) {
                    self.grid.set_direction(MatrixCell::new(row, 0), vol);
                }
            }
            self.grid.set_row_values(&row_prices);
        }
        self.levels.clear();
    }

    pub fn value(&self) -> f64 {
        self.last_poc
    }

    pub fn reset(&mut self) {
        self.levels.clear();
        self.last_poc = 0.0;
        self.grid.reset();
    }

    pub fn is_ready(&self) -> bool {
        self.last_poc != 0.0
    }

    /// POC price from the last closed bar.
    pub fn poc_price(&self) -> f64 { self.last_poc }

    /// Volume-by-price profile: `POC_WINDOW × 1` vector centered on the POC bucket.
    /// The matrix output behind `IndicatorOutputId::FootprintPocProfileGrid`.
    pub fn profile_grid(&self) -> &MatrixGrid { &self.grid }
}

impl TickConsumer for FootprintPoc {
    fn update_tick(&mut self, tick: &Tick) {
        FootprintPoc::update_tick(self, tick);
    }
    fn reset(&mut self) { FootprintPoc::reset(self) }
    fn is_ready(&self) -> bool { FootprintPoc::is_ready(self) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(price: f64, qty: f64) -> Tick {
        Tick::new(0, price, qty, true)
    }

    #[test]
    fn test_poc_selects_highest_volume_bucket() {
        let mut poc = FootprintPoc::new(1.0);
        // 5 ticks @ 100 (total 5) vs 10 ticks @ 101 (total 10)
        for _ in 0..5 {
            poc.update_tick(&tick(100.0, 1.0));
        }
        for _ in 0..10 {
            poc.update_tick(&tick(101.0, 1.0));
        }
        poc.close_bar();
        assert_eq!(poc.poc_price(), 101.0, "bucket 101 has more volume");
    }

    #[test]
    fn test_poc_single_bucket() {
        let mut poc = FootprintPoc::new(1.0);
        poc.update_tick(&tick(50.0, 100.0));
        poc.close_bar();
        assert_eq!(poc.poc_price(), 50.0);
    }

    #[test]
    fn test_poc_reset() {
        let mut poc = FootprintPoc::new(1.0);
        poc.update_tick(&tick(100.0, 10.0));
        poc.close_bar();
        poc.reset();
        assert_eq!(poc.poc_price(), 0.0);
        assert!(!poc.is_ready());
    }

    #[test]
    fn poc_profile_row_labels_are_prices() {
        use crate::engine::matrix_grid::Label;
        let mut poc = FootprintPoc::new(1.0);
        poc.update_tick(&tick(100.0, 10.0));
        poc.close_bar();

        let g = poc.profile_grid();
        let center = POC_WINDOW / 2;
        // Center row = POC bucket 100; price label = 100.0
        assert_eq!(g.row_label(center), Label::Value(100.0));
    }

    #[test]
    fn test_poc_not_ready_before_close() {
        let poc = FootprintPoc::new(1.0);
        assert!(!poc.is_ready());
    }
}

impl Default for FootprintPoc {
    /// Factory default: price_bucket=0.01.
    fn default() -> Self {
        Self::new(0.01)
    }
}

/// Typed dual-mode config for [`FootprintPoc`]: price-level quantization step.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FootprintPocConfig {
    /// Price quantization step (e.g. 0.01 for 1-cent buckets).
    pub price_bucket: Param<f64>,
}

impl Indicator for FootprintPoc {
    const ID: IndicatorId = IndicatorId::FootprintPoc;
    /// Not a pluggable family — a tick-stream POC producer.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::FootprintPoc),
        Output::matrix(IndicatorOutputId::FootprintPocProfileGrid, ValueDomain::Count),
    ];
    /// O(n levels) per tick (rescans all live bucket entries to find max); uses a HashMap.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    type Config = FootprintPocConfig;
    type Runtime = Self;

    fn create(cfg: FootprintPocConfig) -> Self {
        Self::new(cfg.price_bucket.resolved())
    }
}

impl crate::contract::Config for FootprintPocConfig {
    fn defaults() -> Self {
        FootprintPocConfig { price_bucket: Param::Solo(0.01) }
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


impl Render for FootprintPoc {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::FootprintPoc, "POC", Color::hex(0xFF9800))
            .precision(4)
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
        let mut f = IndicatorOrder::FootprintPoc(
            <<FootprintPoc as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();

        // 5 ticks at 100, 10 ticks at 101 → POC at 101
        for _ in 0..5 {
            let t = CoreTick::new(0, 100.0, 1.0, true);
            f.feed(0, MarketSample::Tick(&t));
        }
        for _ in 0..10 {
            let t = CoreTick::new(0, 101.0, 1.0, true);
            f.feed(0, MarketSample::Tick(&t));
        }
        // value() = poc_price ≈ 101.0 (bucket 10100 * 0.01 = 101.0)
        let v = f.primary();
        assert!((v - 101.0).abs() < 1e-9, "POC should be 101.0, got {}", v);
    }

    #[test]
    fn profile_grid_shape_and_content() {
        let mut poc = FootprintPoc::new(1.0);
        // 10 vol at bucket 100 (the POC), 2 vol at bucket 101
        for _ in 0..10 {
            poc.update_tick(&Tick::new(0, 100.0, 1.0, true));
        }
        for _ in 0..2 {
            poc.update_tick(&Tick::new(0, 101.0, 1.0, true));
        }
        poc.close_bar();

        let g = poc.profile_grid();
        assert_eq!(g.rows(), POC_WINDOW, "64 price rows");
        assert_eq!(g.cols(), 1, "single volume column");
        // POC at bucket 100; center row = POC_WINDOW/2
        let center = POC_WINDOW / 2;
        assert_eq!(
            g.read_direction(MatrixCell::new(center, 0)),
            10.0,
            "POC bucket carries 10 volume"
        );
        // Bucket 101 = one above POC → center+1
        assert_eq!(
            g.read_direction(MatrixCell::new(center + 1, 0)),
            2.0,
            "adjacent bucket carries 2 volume"
        );
    }

    #[test]
    fn factory_exposes_poc_profile_vector() {
        let f = IndicatorOrder::FootprintPoc(
            <<FootprintPoc as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let g = f
            .grid(IndicatorOutputId::FootprintPocProfileGrid)
            .expect("FootprintPoc emits the POC profile vector");
        assert_eq!(g.rows(), 64);
        assert_eq!(g.cols(), 1);

        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma).unwrap().build_solo().unwrap();
        assert!(sma.grid(IndicatorOutputId::FootprintPocProfileGrid).is_none());
    }
}
