// GMMA Compression/Expansion score based on MA cluster spreads

use crate::engine::contract_engine::{SmootherChoice, SmootherSlot, SmootherId};
use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};

/// Number of EMA lines in the GMMA ribbon (6 fast + 6 slow).
const GMMA_RIBBON: u16 = 12;

/// Period labels for the 12-EMA ribbon, row order: fast 3/5/8/10/12/15 then slow 30/35/40/45/50/60.
const GMMA_PERIODS: &[&'static str] = &["3","5","8","10","12","15","30","35","40","45","50","60"];

/// GMMA Compression/Expansion score.
///
/// 12 independent smoother slots: 6 fast (periods 3/5/8/10/12/15) + 6 slow
/// (periods 30/35/40/45/50/60). Compression score = inter-cluster separation /
/// (intra-fast spread + intra-slow spread). Large score = trending, small = compressed.
#[derive(Debug, Clone)]
pub struct GmmaCompression {
    fast3:  SmootherSlot,
    fast5:  SmootherSlot,
    fast8:  SmootherSlot,
    fast10: SmootherSlot,
    fast12: SmootherSlot,
    fast15: SmootherSlot,
    slow30: SmootherSlot,
    slow35: SmootherSlot,
    slow40: SmootherSlot,
    slow45: SmootherSlot,
    slow50: SmootherSlot,
    slow60: SmootherSlot,
    score: f64,
    /// 12×1 price-vector snapshot of all EMA values after each bar (row 0 = fast3 … row 11 = slow60).
    grid: MatrixGrid,
}

impl GmmaCompression {
    /// Default ctor — all slots are EMA.
    pub fn new() -> Self {
        Self::from_smoothers(SmootherId::Ema)
    }

    /// Build with a custom smoother shape across all 12 slots.
    pub fn from_smoothers(id: SmootherId) -> Self {
        Self {
            fast3:  SmootherSlot::new(id, 3),
            fast5:  SmootherSlot::new(id, 5),
            fast8:  SmootherSlot::new(id, 8),
            fast10: SmootherSlot::new(id, 10),
            fast12: SmootherSlot::new(id, 12),
            fast15: SmootherSlot::new(id, 15),
            slow30: SmootherSlot::new(id, 30),
            slow35: SmootherSlot::new(id, 35),
            slow40: SmootherSlot::new(id, 40),
            slow45: SmootherSlot::new(id, 45),
            slow50: SmootherSlot::new(id, 50),
            slow60: SmootherSlot::new(id, 60),
            score: 0.0,
            grid: MatrixGrid::new(GMMA_RIBBON, 1, false)
                .with_labels(AxisLabels::Named(GMMA_PERIODS), AxisLabels::Bars),
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, close: f64) -> f64 {
        self.fast3.feed(close);
        self.fast5.feed(close);
        self.fast8.feed(close);
        self.fast10.feed(close);
        self.fast12.feed(close);
        self.fast15.feed(close);
        self.slow30.feed(close);
        self.slow35.feed(close);
        self.slow40.feed(close);
        self.slow45.feed(close);
        self.slow50.feed(close);
        self.slow60.feed(close);

        if self.is_ready() {
            let fast_vals = [
                self.fast3.value(), self.fast5.value(), self.fast8.value(),
                self.fast10.value(), self.fast12.value(), self.fast15.value(),
            ];
            let slow_vals = [
                self.slow30.value(), self.slow35.value(), self.slow40.value(),
                self.slow45.value(), self.slow50.value(), self.slow60.value(),
            ];
            let (fast_min, fast_max) = min_max(&fast_vals);
            let (slow_min, slow_max) = min_max(&slow_vals);
            let intra_fast = fast_max - fast_min;
            let intra_slow = slow_max - slow_min;
            let fast_avg: f64 = fast_vals.iter().sum::<f64>() / fast_vals.len() as f64;
            let slow_avg: f64 = slow_vals.iter().sum::<f64>() / slow_vals.len() as f64;
            let inter = (fast_avg - slow_avg).abs();
            self.score = inter / (1e-9 + intra_fast + intra_slow);

            // Snapshot the 12 EMA values into the ribbon grid.
            // Rows 0-5: fast EMAs (period 3/5/8/10/12/15).
            // Rows 6-11: slow EMAs (period 30/35/40/45/50/60).
            self.grid.reset();
            for (i, &v) in fast_vals.iter().enumerate() {
                self.grid.set_direction(MatrixCell::new(i as u16, 0), v);
            }
            for (i, &v) in slow_vals.iter().enumerate() {
                self.grid.set_direction(MatrixCell::new((6 + i) as u16, 0), v);
            }
        }
        self.score
    }

    /// The 12-row ribbon grid: rows 0-5 = fast EMAs (3/5/8/10/12/15), rows 6-11 = slow EMAs
    /// (30/35/40/45/50/60). Column is always 0. Only populated once `is_ready()` is true.
    pub fn ribbon_grid(&self) -> &MatrixGrid {
        &self.grid
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.score
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        // The slowest slot (slow60) dominates; once it is ready all faster ones are too.
        self.slow60.is_ready()
    }

    pub fn reset(&mut self) {
        self.fast3.reset();  self.fast5.reset();  self.fast8.reset();
        self.fast10.reset(); self.fast12.reset(); self.fast15.reset();
        self.slow30.reset(); self.slow35.reset(); self.slow40.reset();
        self.slow45.reset(); self.slow50.reset(); self.slow60.reset();
        self.score = 0.0;
        self.grid.reset();
    }
}

fn min_max(vals: &[f64]) -> (f64, f64) {
    let mut mn = f64::INFINITY;
    let mut mx = f64::NEG_INFINITY;
    for &v in vals {
        if v < mn { mn = v; }
        if v > mx { mx = v; }
    }
    (mn, mx)
}

impl Default for GmmaCompression {
    fn default() -> Self {
        Self::new()
    }
}

// ─── contract ────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param,
    Slot, SourceAxis, UpdateComplexity, ValueDomain,
};
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`GmmaCompression`].
///
/// 12 independent smoother slots — one per GMMA EMA. Each slot follows its lane's period
/// from `fast_periods` (indices 0-5 for 3/5/8/10/12/15) or `slow_periods` (indices 0-5
/// for 30/35/40/45/50/60). `source` is the price field.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct GmmaCompressionConfig {
    pub source: Param<OhlcvField>,
    pub fast_periods: Param<[usize; 6]>,
    pub slow_periods: Param<[usize; 6]>,
    #[slot] pub fast3:  Param<SmootherChoice>,
    #[slot] pub fast5:  Param<SmootherChoice>,
    #[slot] pub fast8:  Param<SmootherChoice>,
    #[slot] pub fast10: Param<SmootherChoice>,
    #[slot] pub fast12: Param<SmootherChoice>,
    #[slot] pub fast15: Param<SmootherChoice>,
    #[slot] pub slow30: Param<SmootherChoice>,
    #[slot] pub slow35: Param<SmootherChoice>,
    #[slot] pub slow40: Param<SmootherChoice>,
    #[slot] pub slow45: Param<SmootherChoice>,
    #[slot] pub slow50: Param<SmootherChoice>,
    #[slot] pub slow60: Param<SmootherChoice>,
}

impl Indicator for GmmaCompression {
    const ID: IndicatorId = IndicatorId::Gmma;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1): 12 recursive smoothers; the slowest warm-up is period=60, but per-bar is O(1).
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = GmmaCompressionConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::magnitude(IndicatorOutputId::Gmma),
        Output::matrix(IndicatorOutputId::GmmaRibbonGrid, ValueDomain::Price),
    ];
    type Config = GmmaCompressionConfig;
    type Runtime = GmmaCompression;

    fn create(cfg: GmmaCompressionConfig) -> GmmaCompression {
        let fp = cfg.fast_periods.resolved();
        let sp = cfg.slow_periods.resolved();
        let mk = |c: SmootherChoice, host: usize| SmootherSlot::new(c.id(), c.period.resolve(host).max(1));
        GmmaCompression {
            fast3:  mk(cfg.fast3.resolved(),  fp[0]),
            fast5:  mk(cfg.fast5.resolved(),  fp[1]),
            fast8:  mk(cfg.fast8.resolved(),  fp[2]),
            fast10: mk(cfg.fast10.resolved(), fp[3]),
            fast12: mk(cfg.fast12.resolved(), fp[4]),
            fast15: mk(cfg.fast15.resolved(), fp[5]),
            slow30: mk(cfg.slow30.resolved(), sp[0]),
            slow35: mk(cfg.slow35.resolved(), sp[1]),
            slow40: mk(cfg.slow40.resolved(), sp[2]),
            slow45: mk(cfg.slow45.resolved(), sp[3]),
            slow50: mk(cfg.slow50.resolved(), sp[4]),
            slow60: mk(cfg.slow60.resolved(), sp[5]),
            score: 0.0,
            grid: MatrixGrid::new(GMMA_RIBBON, 1, false)
                .with_labels(AxisLabels::Named(GMMA_PERIODS), AxisLabels::Bars),
        }
    }

    fn source_fields(cfg: &GmmaCompressionConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &GmmaCompressionConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for GmmaCompressionConfig {
    fn defaults() -> Self {
        GmmaCompressionConfig {
            source: Param::Solo(OhlcvField::Close),
            fast_periods: Param::Solo([3, 5, 8, 10, 12, 15]),
            slow_periods: Param::Solo([30, 35, 40, 45, 50, 60]),
            fast3:  Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            fast5:  Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            fast8:  Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            fast10: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            fast12: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            fast15: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            slow30: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            slow35: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            slow40: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            slow45: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            slow50: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            slow60: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // source: Class O — auto all-8.
        // fast3..slow60: #[slot] SmootherChoice × 12 — left Solo (deferred wave). [FLAG: slots]
        //
        // fast_periods / slow_periods: Class S ([usize;6] fixed arrays).
        // Cannot sweep element-by-element; enumerate 4 canonical Guppy presets per cluster.
        //   fast presets: Guppy standard | short-term bias | medium | crypto-adapted
        //   slow presets: Guppy standard | long-term bias  | medium | crypto-adapted
        let mut s = Self::machine_defaults_auto();
        s.fast_periods = Param::many(vec![
            [3,  5,  8,  10, 12, 15],  // Daryl Guppy original (standard)
            [4,  6,  9,  11, 13, 16],  // slight upward shift
            [2,  4,  6,   8, 10, 13],  // faster / scalping variant
            [5,  8, 10,  13, 15, 20],  // medium-term bias
        ]);
        s.slow_periods = Param::many(vec![
            [30, 35, 40, 45, 50, 60],  // Daryl Guppy original (standard)
            [25, 30, 35, 40, 45, 55],  // shorter long-term cluster
            [35, 40, 45, 50, 55, 65],  // longer long-term cluster
            [20, 25, 30, 35, 45, 50],  // crypto-adapted (faster slow group)
        ]);
        s
    }
}


impl Render for GmmaCompression {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Gmma, "GMMA", Color::hex(0x2196F3))
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
    fn test_gmma_compression_creation() {
        let gmma = GmmaCompression::new();
        assert!(!gmma.is_ready());
        assert_eq!(gmma.value(), 0.0);
    }

    #[test]
    fn test_gmma_compression_warmup() {
        let mut gmma = GmmaCompression::new();
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            gmma.feed(price);
        }
        assert!(gmma.is_ready());
    }

    #[test]
    fn test_gmma_compression_values_finite() {
        let mut gmma = GmmaCompression::new();
        for i in 0..80 {
            let price = 100.0 + i as f64;
            let value = gmma.feed(price);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_gmma_compression_reset() {
        let mut gmma = GmmaCompression::new();
        for i in 0..80 {
            gmma.feed(100.0 + i as f64);
        }
        gmma.reset();
        assert!(!gmma.is_ready());
        assert_eq!(gmma.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_gmma() {
        let mut f = IndicatorOrder::Gmma(<<GmmaCompression as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..80 {
            let price = 100.0 + i as f64;
            // WILD value in open/high/low — only close is consumed by SOURCE.
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 1000.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite());
    }

    #[test]
    fn ribbon_grid_labels() {
        use crate::engine::matrix_grid::Label;
        let gmma = GmmaCompression::new();
        let g = gmma.ribbon_grid();
        assert_eq!(g.row_label(0), Label::Name("3"));
        assert_eq!(g.row_label(11), Label::Name("60"));
    }

    #[test]
    fn ribbon_grid_direct_accessor() {
        use crate::engine::matrix_grid::MatrixCell;
        let mut gmma = GmmaCompression::new();
        // Feed a monotonically rising series long enough to warm slow60 (period=60).
        for i in 0..70 {
            gmma.feed(100.0 + i as f64);
        }
        assert!(gmma.is_ready(), "slow60 must be ready after 70 bars");

        let g = gmma.ribbon_grid();
        assert_eq!(g.rows(), 12, "12 EMA rows");
        assert_eq!(g.cols(), 1, "single column");

        // All 12 values must be non-zero and finite.
        for row in 0..12u16 {
            let v = g.read_direction(MatrixCell::new(row, 0));
            assert!(v.is_finite() && v != 0.0, "row {row} should be finite non-zero, got {v}");
        }

        // On a monotonically rising input, faster EMA (row 0, period=3) tracks price
        // more closely than the slowest (row 11, period=60), so fast >= slow.
        let fast = g.read_direction(MatrixCell::new(0, 0));
        let slow = g.read_direction(MatrixCell::new(11, 0));
        assert!(
            fast >= slow,
            "fast EMA (row 0) = {fast} should be >= slow EMA (row 11) = {slow} on rising input"
        );
    }

    #[test]
    fn factory_exposes_gmma_ribbon_vector() {
        use crate::engine::indicator_id::IndicatorId;
        use crate::engine::contract_engine::IndicatorOutputId;
        use crate::contract::Config;

        let mut f = IndicatorOrder::Gmma(GmmaCompressionConfig::defaults())
            .build_solo()
            .unwrap();
        for i in 0..70 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 1000.0,
            });
        }
        assert!(f.is_ready());

        let g = f
            .grid(IndicatorOutputId::GmmaRibbonGrid)
            .expect("GmmaCompression must expose the ribbon grid");
        assert_eq!(g.rows(), 12, "12 EMA rows");
        assert_eq!(g.cols(), 1, "single column");

        // A scalar-only producer must return None for this output id.
        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma).unwrap().build_solo().unwrap();
        assert!(sma.grid(IndicatorOutputId::GmmaRibbonGrid).is_none());
    }
}
