// Didi Index (Odir Aguiar): three MAs expressed as ratios to the mid line.

use crate::engine::contract_engine::{SmootherSlot, SmootherId};

/// Didi Index: short/mid/long MAs of price, reported as `short = Short/Mid` and
/// `long = Long/Mid` (the mid line is the 1.0 reference). Three independent smoother
/// slots, each owning its period + shape. PURE core — owns no source; the factory feeds
/// it the resolved scalar via [`DidiIndex::feed`].
#[derive(Debug, Clone)]
pub struct DidiIndex {
    ema_short: SmootherSlot,
    ema_mid: SmootherSlot,
    ema_long: SmootherSlot,
    short_ratio: f64,
    long_ratio: f64,
}

impl DidiIndex {
    /// Default ctor — all three MAs are EMA.
    pub fn new(short_p: usize, mid_p: usize, long_p: usize) -> Self {
        Self::from_smoothers(SmootherId::Ema, short_p, mid_p, long_p)
    }

    /// Alias kept for the legacy call site.
    pub fn new_default(short_p: usize, mid_p: usize, long_p: usize) -> Self {
        Self::new(short_p, mid_p, long_p)
    }

    /// Build the three MAs from one narrow `SmootherId`.
    /// Legacy bridge; the contract path goes through `DidiConfig`.
    pub fn from_smoothers(ma: SmootherId, short_p: usize, mid_p: usize, long_p: usize) -> Self {
        Self {
            ema_short: SmootherSlot::new(ma, short_p.max(1)),
            ema_mid: SmootherSlot::new(ma, mid_p.max(2)),
            ema_long: SmootherSlot::new(ma, long_p.max(3)),
            short_ratio: 0.0,
            long_ratio: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ema_short.reset();
        self.ema_mid.reset();
        self.ema_long.reset();
        self.short_ratio = 0.0;
        self.long_ratio = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ema_long.is_ready()
    }

    /// Returns the short ratio (Short/Mid).
    pub fn short_ratio(&self) -> f64 {
        self.short_ratio
    }

    /// Returns the long ratio (Long/Mid).
    pub fn long_ratio(&self) -> f64 {
        self.long_ratio
    }

    /// Brace-named getter for the `short` output (delegates to `short_ratio()`).
    #[inline]
    pub fn short(&self) -> f64 {
        self.short_ratio
    }

    /// Brace-named getter for the `long` output (delegates to `long_ratio()`).
    #[inline]
    pub fn long(&self) -> f64 {
        self.long_ratio
    }

    /// Feed ONE pre-extracted scalar — the pure core computation. Returns the short-long
    /// spread (kept for the scalar return; the full output is the `Double` from `value()`).
    pub fn feed(&mut self, value: f64) -> f64 {
        let s = self.ema_short.feed(value);
        let m = self.ema_mid.feed(value);
        let l = self.ema_long.feed(value);
        if m.abs() > 1e-12 {
            self.short_ratio = s / m;
            self.long_ratio = l / m;
        }
        self.short_ratio - self.long_ratio
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderOutput, RenderSpec};

/// Typed contract config for [`DidiIndex`] — three MAs (short/mid/long) over `source`,
/// each following its lane's period field. The mid MA is the 1.0 reference the
/// short/long ratios are taken against.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct DidiConfig {
    pub source: Param<OhlcvField>,
    pub short_period: Param<usize>,
    pub mid_period: Param<usize>,
    pub long_period: Param<usize>,
    #[slot]
    pub short: Param<SmootherChoice>,
    #[slot]
    pub mid: Param<SmootherChoice>,
    #[slot]
    pub long: Param<SmootherChoice>,
}

impl Indicator for DidiIndex {
    const ID: IndicatorId = IndicatorId::Didi;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1): three smoothers expressed as ratios; all three buffers land recursively
    /// through `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = DidiConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::ratio(IndicatorOutputId::DidiShort),
        Output::ratio(IndicatorOutputId::DidiLong),
    ];
    type Config = DidiConfig;
    type Runtime = DidiIndex;

    fn create(cfg: DidiConfig) -> DidiIndex {
        let short_p = cfg.short_period.resolved().max(1);
        let mid_p = cfg.mid_period.resolved().max(2);
        let long_p = cfg.long_period.resolved().max(3);
        DidiIndex {
            ema_short: cfg.short.resolved().build(short_p),
            ema_mid: cfg.mid.resolved().build(mid_p),
            ema_long: cfg.long.resolved().build(long_p),
            short_ratio: 0.0,
            long_ratio: 0.0,
        }
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the resolved scalar.
    fn source_fields(cfg: &DidiConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &DidiConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for DidiConfig {
    fn valid_params(&self) -> Result<(), String> {
        let short = self.short_period.resolved();
        let mid = self.mid_period.resolved();
        let long = self.long_period.resolved();
        if !(short < mid && mid < long) {
            return Err(format!("short_period({short}) < mid_period({mid}) < long_period({long}) required"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        DidiConfig {
            source: Param::Solo(OhlcvField::Close),
            short_period: Param::Solo(3),
            mid_period: Param::Solo(8),
            long_period: Param::Solo(20),
            short: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            mid: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            long: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // source: Class O — auto all-8; short_period/mid_period/long_period: Class A — auto
        // range(2,4048,1) EACH, but all three then resolve to the same min (2), failing this
        // config's OWN valid_params (short < mid < long) at the min corner (2026-07-03 fix).
        // Split into three disjoint ranges so resolved() stays strictly ordered.
        // short/mid/long: #[slot] SmootherChoice — left Solo (deferred wave). [FLAG: slots]
        let mut s = Self::machine_defaults_auto();
        s.short_period = Param::range(1, 50, 1);
        s.mid_period = Param::range(51, 200, 1);
        s.long_period = Param::range(201, 10000, 1);
        s
    }
}


impl Render for DidiIndex {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::DidiShort, "Short", Color::hex(0x4CAF50), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::DidiLong, "Long", Color::hex(0xF44336), 1.0))
            .reference_line(ReferenceLine::new(1.0, Color::hex(0x9E9E9E)))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_didi_index_creation() {
        let didi = DidiIndex::new(3, 8, 20);
        assert!(!didi.is_ready());
        assert_eq!(didi.short_ratio(), 0.0);
        assert_eq!(didi.long_ratio(), 0.0);
    }

    #[test]
    fn test_didi_index_warmup() {
        let mut didi = DidiIndex::new(3, 8, 20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            didi.feed(price);
        }
        assert!(didi.is_ready());
    }

    #[test]
    fn test_didi_index_values_finite() {
        let mut didi = DidiIndex::new(3, 8, 20);
        for i in 0..30 {
            let value = didi.feed(100.0 + i as f64);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_didi_index_reset() {
        let mut didi = DidiIndex::new(3, 8, 20);
        for _ in 0..30 {
            didi.feed(101.0);
        }
        didi.reset();
        assert!(!didi.is_ready());
        assert_eq!(didi.short_ratio(), 0.0);
        assert_eq!(didi.long_ratio(), 0.0);
    }

    #[test]
    fn test_didi_index_contract_create() {
        let cfg = <<DidiIndex as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut didi = <DidiIndex as Indicator>::create(cfg);
        for i in 0..40 {
            didi.feed(100.0 + i as f64);
        }
        assert!(didi.is_ready());
    }
}
