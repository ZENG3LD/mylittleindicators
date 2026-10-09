//! Volume Weighted Price Levels — VWAP and volume-based support/resistance.
//!
//! This indicator is OHLCV-based and does not require L2 data.
//! VWAP is computed from (H+L+C)/3 * volume. Volume nodes and swing
//! support/resistance are derived from OHLCV bars.
//!
//! Output: current VWAP price. Additional data accessible via getters.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity, ValueDomain};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::types::Bar;

/// Fixed row count of the price-levels VECTOR. Rows are filled from index 0 ascending by price;
/// remaining rows are zero-padded. Sized to comfortably exceed the practical level count
/// (≤ 10 HVN + ~several S/R + 1 VWAP).
const VWPL_ROWS: u16 = 32;

/// A volume-weighted price level.
#[derive(Debug, Clone)]
pub struct VwapLevel {
    pub price: f64,
    pub volume_weight: f64,
    /// Significance 0.0–1.0 (fraction of total cumulative volume).
    pub significance: f64,
    pub level_type: LevelType,
    pub touch_count: usize,
    pub last_touch_time: i64,
}

/// Classification of a price level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LevelType {
    Support,
    Resistance,
    Pivot,
    VWAP,
    HighVolumeNode,
    LowVolumeNode,
}

/// Volume Weighted Price Levels analyser.
#[derive(Debug, Clone)]
pub struct VolumeWeightedPriceLevels {
    period: usize,
    price_precision: f64,

    volume_bars: Vec<Bar>,
    levels: Vec<VwapLevel>,

    cumulative_volume: f64,
    cumulative_price_volume: f64,
    current_vwap: f64,

    strongest_support: Option<VwapLevel>,
    strongest_resistance: Option<VwapLevel>,
    active_levels_count: usize,

    /// Price-levels vector: `VWPL_ROWS × 1` of price values sorted ascending.
    /// Row 0 = lowest price level, rows filled up to `min(levels.len(), VWPL_ROWS)`.
    /// The matrix output behind `IndicatorOutputId::VwapLevelsLevelsGrid`.
    grid: MatrixGrid,
}

/// Typed dual-mode config for [`VolumeWeightedPriceLevels`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VolumeWeightedPriceLevelsConfig {
    pub period: Param<usize>,
    pub price_precision: Param<f64>,
}

impl VolumeWeightedPriceLevels {
    pub fn new(period: usize, price_precision: f64) -> Self {
        Self {
            period,
            price_precision,
            volume_bars: Vec::with_capacity(period),
            levels: Vec::with_capacity(64),
            cumulative_volume: 0.0,
            cumulative_price_volume: 0.0,
            current_vwap: 0.0,
            strongest_support: None,
            strongest_resistance: None,
            active_levels_count: 0,
            grid: MatrixGrid::new(VWPL_ROWS, 1, false)
                .with_labels(AxisLabels::Values(Vec::new()), AxisLabels::Bars),
        }
    }

    /// Update from an OHLCV bar. Returns current VWAP.
    pub fn update_volume_bar(&mut self, volume_bar: &Bar) -> f64 {
        if self.volume_bars.len() >= self.period {
            self.volume_bars.remove(0);
        }
        self.volume_bars.push(*volume_bar);
        self.update_vwap(volume_bar);
        self.analyze_levels();
        self.current_vwap
    }

    /// Contract entry-point: receive all 5 OHLCV fields as a flat slice.
    /// Lane order matches `const SOURCE`: [Open, High, Low, Close, Volume].
    pub fn feed(&mut self, lanes: &[f64]) {
        if lanes.len() < 5 {
            return;
        }
        let bar = Bar {
            time: 0,
            open: lanes[0],
            high: lanes[1],
            low: lanes[2],
            close: lanes[3],
            volume: lanes[4],
        };
        self.update_volume_bar(&bar);
    }

    fn update_vwap(&mut self, volume_bar: &Bar) {
        let typical_price = (volume_bar.high + volume_bar.low + volume_bar.close) / 3.0;
        self.cumulative_volume += volume_bar.volume;
        self.cumulative_price_volume += typical_price * volume_bar.volume;
        if self.cumulative_volume > 0.0 {
            self.current_vwap = self.cumulative_price_volume / self.cumulative_volume;
        }
    }

    fn analyze_levels(&mut self) {
        self.levels.clear();
        if self.volume_bars.len() < 5 {
            return;
        }
        if self.current_vwap > 0.0 {
            self.add_level(self.current_vwap, self.cumulative_volume, LevelType::VWAP);
        }
        self.find_high_volume_nodes();
        self.identify_support_resistance();
        self.update_level_statistics();
    }

    fn find_high_volume_nodes(&mut self) {
        let mut price_volume_map: std::collections::HashMap<i64, f64> =
            std::collections::HashMap::new();

        for bar in &self.volume_bars {
            let prices = [bar.open, bar.high, bar.low, bar.close];
            let volume_per_price = bar.volume / 4.0;
            for price in &prices {
                let price_key = (*price / self.price_precision).round() as i64;
                *price_volume_map.entry(price_key).or_insert(0.0) += volume_per_price;
            }
        }

        let mut volume_levels: Vec<(f64, f64)> = price_volume_map
            .into_iter()
            .map(|(price_key, volume)| (price_key as f64 * self.price_precision, volume))
            .collect();
        volume_levels.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        for (price, volume) in volume_levels.iter().take(10) {
            if *volume > self.cumulative_volume * 0.05 {
                self.add_level(*price, *volume, LevelType::HighVolumeNode);
            }
        }
    }

    fn identify_support_resistance(&mut self) {
        if self.volume_bars.len() < 10 {
            return;
        }
        let current_price = self.volume_bars.last().unwrap().close;
        let bars_clone = self.volume_bars.clone();

        for i in 2..bars_clone.len().saturating_sub(2) {
            let bar = &bars_clone[i];

            if bar.high > bars_clone[i - 1].high
                && bar.high > bars_clone[i - 2].high
                && bar.high > bars_clone[i + 1].high
                && bar.high > bars_clone[i + 2].high
            {
                let level_type = if bar.high > current_price {
                    LevelType::Resistance
                } else {
                    LevelType::Support
                };
                self.add_level(bar.high, bar.volume, level_type);
            }

            if bar.low < bars_clone[i - 1].low
                && bar.low < bars_clone[i - 2].low
                && bar.low < bars_clone[i + 1].low
                && bar.low < bars_clone[i + 2].low
            {
                let level_type = if bar.low < current_price {
                    LevelType::Support
                } else {
                    LevelType::Resistance
                };
                self.add_level(bar.low, bar.volume, level_type);
            }
        }
    }

    fn add_level(&mut self, price: f64, volume: f64, level_type: LevelType) {
        let significance = (volume / self.cumulative_volume.max(1.0)).min(1.0);
        self.levels.push(VwapLevel {
            price,
            volume_weight: volume,
            significance,
            level_type,
            touch_count: 0,
            last_touch_time: 0,
        });
    }

    fn update_level_statistics(&mut self) {
        self.strongest_support = None;
        self.strongest_resistance = None;
        self.active_levels_count = 0;

        let mut max_support_sig = 0.0f64;
        let mut max_resistance_sig = 0.0f64;

        for level in &self.levels {
            if level.significance > 0.1 {
                self.active_levels_count += 1;
            }
            match level.level_type {
                LevelType::Support if level.significance > max_support_sig => {
                    max_support_sig = level.significance;
                    self.strongest_support = Some(level.clone());
                }
                LevelType::Resistance if level.significance > max_resistance_sig => {
                    max_resistance_sig = level.significance;
                    self.strongest_resistance = Some(level.clone());
                }
                _ => {}
            }
        }

        // Snapshot all price levels into the VECTOR sorted ascending.
        self.grid.reset();
        let mut prices: Vec<f64> = self.levels.iter().map(|l| l.price).collect();
        prices.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let capped: Vec<f64> = prices.into_iter().take(VWPL_ROWS as usize).collect();
        for (row, &price) in capped.iter().enumerate() {
            self.grid.set_direction(MatrixCell::new(row as u16, 0), price);
        }
        self.grid.set_row_values(&capped);
    }

    pub fn current_vwap(&self) -> f64 { self.current_vwap }
    pub fn get_levels(&self) -> &[VwapLevel] { &self.levels }
    pub fn strongest_support(&self) -> Option<&VwapLevel> { self.strongest_support.as_ref() }
    pub fn strongest_resistance(&self) -> Option<&VwapLevel> { self.strongest_resistance.as_ref() }
    pub fn active_levels_count(&self) -> usize { self.active_levels_count }

    pub fn nearest_level(&self, price: f64) -> Option<&VwapLevel> {
        self.levels.iter().min_by(|a, b| {
            let da = (a.price - price).abs();
            let db = (b.price - price).abs();
            da.partial_cmp(&db).unwrap()
        })
    }

    pub fn levels_by_type(&self, level_type: LevelType) -> Vec<&VwapLevel> {
        self.levels.iter().filter(|l| l.level_type == level_type).collect()
    }

    pub fn is_ready(&self) -> bool {
        self.volume_bars.len() >= (self.period / 2).max(5)
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.current_vwap
    }

    pub fn reset(&mut self) {
        self.volume_bars.clear();
        self.levels.clear();
        self.cumulative_volume = 0.0;
        self.cumulative_price_volume = 0.0;
        self.current_vwap = 0.0;
        self.strongest_support = None;
        self.strongest_resistance = None;
        self.active_levels_count = 0;
        self.grid.reset();
    }

    /// Price levels sorted ascending: `VWPL_ROWS × 1` PRICE vector.
    /// Row 0 = lowest price level; unused rows are zero.
    /// The matrix output behind `IndicatorOutputId::VwapLevelsLevelsGrid`.
    pub fn levels_grid(&self) -> &MatrixGrid { &self.grid }
}

impl Default for VolumeWeightedPriceLevels {
    /// Factory default: period=14, price_precision=0.01.
    fn default() -> Self {
        Self::new(14, 0.01)
    }
}

impl Indicator for VolumeWeightedPriceLevels {
    const ID: IndicatorId = IndicatorId::VwapLevels;
    /// Not a pluggable family member — a specialized VWAP/S-R producer.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed OHLCV slice: [Open, High, Low, Close, Volume].
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::VwapLevels),
        Output::matrix(IndicatorOutputId::VwapLevelsLevelsGrid, ValueDomain::Price),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[
        Store::window(StoreKind::Vec), // volume_bars rolling buffer
        Store::fixed(StoreKind::Vec, 64), // levels output buffer
    ]);
    type Config = VolumeWeightedPriceLevelsConfig;
    type Runtime = VolumeWeightedPriceLevels;

    fn create(cfg: VolumeWeightedPriceLevelsConfig) -> VolumeWeightedPriceLevels {
        VolumeWeightedPriceLevels::new(cfg.period.resolved(), cfg.price_precision.resolved())
    }
}

impl crate::contract::Config for VolumeWeightedPriceLevelsConfig {
    fn defaults() -> Self {
        VolumeWeightedPriceLevelsConfig { period: Param::Solo(14), price_precision: Param::Solo(0.01) }
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
        // price_precision: Class I (instrument-relative price granularity — same semantics
        // as price_bucket). PIN — f64 auto already leaves it Solo; restore explicitly.
        s.price_precision = Self::defaults().price_precision;
        s
    }
}


impl Render for VolumeWeightedPriceLevels {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::VwapLevels, "VWAP Levels", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn test_vwpl_creation() {
        let ind = VolumeWeightedPriceLevels::new(20, 0.01);
        assert!(!ind.is_ready());
        assert_eq!(ind.current_vwap(), 0.0);
    }

    #[test]
    fn test_vwpl_warmup() {
        let mut ind = VolumeWeightedPriceLevels::new(10, 0.01);
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
    fn test_vwpl_vwap_calculation() {
        let mut ind = VolumeWeightedPriceLevels::new(10, 0.01);
        for i in 0..10 {
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
        assert!(ind.current_vwap() > 0.0);
        assert!(ind.current_vwap().is_finite());
    }

    #[test]
    fn test_vwpl_reset() {
        let mut ind = VolumeWeightedPriceLevels::new(10, 0.01);
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
        assert_eq!(ind.current_vwap(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_vwap_levels() {
        
        let mut f = IndicatorOrder::VwapLevels(
            <<VolumeWeightedPriceLevels as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        // SOURCE uses all 5 OHLCV fields; feed several bars to warm up.
        for i in 0..10 {
            f.feed(0, MarketSample::Bar {
                open: 100.0, high: 102.0, low: 98.0, close: 101.0,
                volume: 1000.0 + i as f64 * 10.0,
            });
        }
        let v = f.primary();
        assert!(v > 0.0, "VWAP must be positive after warmup");
        assert!(v.is_finite());
    }

    #[test]
    fn vwpl_row_labels_are_prices() {
        use crate::engine::matrix_grid::Label;
        let mut ind = VolumeWeightedPriceLevels::new(10, 0.01);
        for i in 0..10 {
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
        let g = ind.levels_grid();
        // Row 0 must be a valued price label (the lowest sorted level)
        match g.row_label(0) {
            Label::Value(p) => assert!(p > 0.0, "price label must be positive"),
            other => panic!("expected Label::Value at row 0, got {:?}", other),
        }
    }

    #[test]
    fn levels_grid_shape_and_content() {
        let mut ind = VolumeWeightedPriceLevels::new(10, 0.01);
        // Feed enough bars to trigger level analysis (≥5 required).
        for i in 0..10 {
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
        let g = ind.levels_grid();
        assert_eq!(g.rows(), VWPL_ROWS, "32 price rows");
        assert_eq!(g.cols(), 1, "single price column");
        // Row 0 should carry the lowest price level (> 0 after warmup).
        let p0 = g.read_direction(MatrixCell::new(0, 0));
        assert!(p0 > 0.0, "first price level must be positive, got {p0}");
        // Levels must be sorted ascending: each non-zero row ≥ previous non-zero row.
        let mut prev = 0.0f64;
        for row in 0..VWPL_ROWS {
            let p = g.read_direction(MatrixCell::new(row, 0));
            if p > 0.0 {
                assert!(
                    p >= prev,
                    "price at row {row} ({p}) must be >= prev ({prev})"
                );
                prev = p;
            }
        }
    }

    #[test]
    fn factory_exposes_vwap_price_levels_vector() {
        let f = IndicatorOrder::VwapLevels(
            <<VolumeWeightedPriceLevels as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let g = f
            .grid(IndicatorOutputId::VwapLevelsLevelsGrid)
            .expect("VolumeWeightedPriceLevels emits the VwapPriceLevels vector");
        assert_eq!(g.rows(), VWPL_ROWS);
        assert_eq!(g.cols(), 1);

        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma).unwrap().build_solo().unwrap();
        assert!(sma.grid(IndicatorOutputId::VwapLevelsLevelsGrid).is_none());
    }
}
