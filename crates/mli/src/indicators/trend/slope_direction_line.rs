// Slope Direction Line (SDL): sign of slope of a smoothing MA

use crate::engine::contract_engine::{SmootherChoice, SmootherSlot, SmootherId};

/// Slope Direction Line — the sign of a smoothed-price one-step slope. PURE core — owns no
/// source; the factory feeds it the resolved scalar via [`SlopeDirectionLine::feed`].
#[derive(Debug, Clone)]
pub struct SlopeDirectionLine {
    period: usize,
    ma: SmootherSlot,
    prev: f64,
    curr: f64,
    dir: i8,
}

impl SlopeDirectionLine {
    /// Default ctor — price smoothed with SMA.
    pub fn new(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Sma)
    }

    /// Build from a narrow `SmootherId` for the smoother + period.
    /// Legacy bridge; the contract path goes through `SdlConfig`.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        Self {
            period,
            ma: SmootherSlot::new(smoother, period),
            prev: 0.0,
            curr: 0.0,
            dir: 0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ma.reset();
        self.prev = 0.0;
        self.curr = 0.0;
        self.dir = 0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ma.is_ready()
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, value: f64) -> i8 {
        let v = self.ma.feed(value);
        self.prev = self.curr;
        self.curr = v;
        self.dir = if self.curr > self.prev {
            1
        } else if self.curr < self.prev {
            -1
        } else {
            0
        };
        self.dir
    }

    #[inline]
    pub fn value(&self) -> f64 {
        (self.dir) as f64
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`SlopeDirectionLine`] — sign of a smoothed-price slope.
/// Self-contained: owns its `period` + `source`; the smoother `ma` (`Param<SmootherChoice>`)
/// owns its own period + shape.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct SdlConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl Indicator for SlopeDirectionLine {
    const ID: IndicatorId = IndicatorId::Sdl;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1): a one-step slope sign over the smoother; the smoother buffer cost lands via the slot.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = SdlConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::Sdl)];
    type Config = SdlConfig;
    type Runtime = SlopeDirectionLine;

    fn create(cfg: SdlConfig) -> SlopeDirectionLine {
        let period = cfg.period.resolved();
        let choice = cfg.ma.resolved();
        SlopeDirectionLine {
            period,
            ma: SmootherSlot::new(choice.id(), choice.period.resolve(period)),
            prev: 0.0,
            curr: 0.0,
            dir: 0,
        }
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the resolved scalar.
    fn source_fields(cfg: &SdlConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &SdlConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for SdlConfig {
    fn defaults() -> Self {
        SdlConfig {
            period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
            ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A — auto range(2,4048,1); source: Class O — auto all-8.
        // ma: #[slot] SmootherChoice — left Solo (deferred wave). [FLAG: slot]
        Self::machine_defaults_auto()
    }
}


impl Render for SlopeDirectionLine {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Sdl, "SDL", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slope_direction_line_creation() {
        let sdl = SlopeDirectionLine::new(10);
        assert!(!sdl.is_ready());
        assert_eq!(sdl.period(), 10);
    }

    #[test]
    fn test_slope_direction_line_warmup() {
        let mut sdl = SlopeDirectionLine::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sdl.feed(price);
        }
        assert!(sdl.is_ready());
    }

    #[test]
    fn test_slope_direction_line_signal_range() {
        let mut sdl = SlopeDirectionLine::new(10);
        for i in 0..20 {
            let dir = sdl.feed(100.0 + i as f64);
            assert!(dir >= -1 && dir <= 1, "Direction should be -1, 0, or 1");
        }
    }

    #[test]
    fn test_slope_direction_line_reset() {
        let mut sdl = SlopeDirectionLine::new(10);
        for _ in 0..15 {
            sdl.feed(101.0);
        }
        sdl.reset();
        assert!(!sdl.is_ready());
    }

    #[test]
    fn test_slope_direction_line_contract_create() {
        let cfg = <<SlopeDirectionLine as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut sdl = <SlopeDirectionLine as Indicator>::create(cfg);
        for i in 0..20 {
            sdl.feed(100.0 + i as f64);
        }
        assert!(sdl.is_ready());
    }
}
