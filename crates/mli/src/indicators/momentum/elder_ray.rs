use crate::engine::contract_engine::SmootherSlot;

/// Elder-Ray: Bull Power = High - EMA(close), Bear Power = Low - EMA(close)
#[derive(Debug, Clone)]
pub struct ElderRay {
    period: usize,
    ema: SmootherSlot,
    bull: f64,
    bear: f64,
    ready: bool,
}

impl ElderRay {
    /// Default ctor — close smoothed with EMA (the classic Elder-Ray kernel).
    pub fn new(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Ema)
    }

    /// Build from a narrow `SmootherId` for the smoother + period (over close).
    /// Legacy bridge; the contract path goes through `ElderRayConfig`.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            ema: SmootherSlot::new(smoother, p),
            bull: 0.0,
            bear: 0.0,
            ready: false,
        }
    }

    /// Feed the resolved input lanes — `[high, low, close]` (the factory resolves the fixed
    /// HLC slice). Bull = high - EMA(close), Bear = low - EMA(close). Knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let h = lanes[0];
        let l = lanes[1];
        let c = lanes[2];
        self.ema.feed(c);
        let ema = self.ema.value();
        self.bull = h - ema;
        self.bear = l - ema;
        self.ready = self.ema.is_ready();
        (self.bull, self.bear)
    }


    /// Brace-named getter: `bull` output (Bull Power = High - EMA).
    #[inline]
    pub fn bull(&self) -> f64 {
        self.bull
    }

    /// Brace-named getter: `bear` output (Bear Power = Low - EMA).
    #[inline]
    pub fn bear(&self) -> f64 {
        self.bear
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn reset(&mut self) {
        self.ema.reset();
        self.bull = 0.0;
        self.bear = 0.0;
        self.ready = false;
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, RenderOutput, RenderSpec};

impl ElderRay {
    /// Build an ElderRay from a smoother CHOICE (kind + follow/own period) at the host `period`.
    pub fn from_choice(choice: SmootherChoice, period: usize) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            ema: choice.build(p),
            bull: 0.0,
            bear: 0.0,
            ready: false,
        }
    }
}

/// Typed contract config for [`ElderRay`] — Bull/Bear Power around a smoothed close.
///
/// Dual-mode: every field is a `Param`. The `#[slot]` smoother is a `Param<SmootherChoice>`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct ElderRayConfig {
    pub period: Param<usize>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl Indicator for ElderRay {
    const ID: IndicatorId = IndicatorId::ElderRay;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Smooths close; bull/bear measured against high/low.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// O(1): high/low minus the smoothed close; the smoother buffer cost lands via the slot.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = ElderRayConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::ElderRayBull),
        Output::centered(IndicatorOutputId::ElderRayBear),
    ];
    type Config = ElderRayConfig;
    type Runtime = ElderRay;

    fn create(cfg: ElderRayConfig) -> ElderRay {
        let period = cfg.period.resolved();
        ElderRay {
            period: period.max(1),
            ema: cfg.ma.resolved().build(period),
            bull: 0.0,
            bear: 0.0,
            ready: false,
        }
    }

    fn slot_members(cfg: &ElderRayConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for ElderRayConfig {
    fn defaults() -> Self {
        ElderRayConfig {
            period: Param::Solo(14),
            ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        // ma: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for ElderRay {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(IndicatorOutputId::ElderRayBull, "Bull Power", Color::hex(0x4CAF50)))
            .output(RenderOutput::histogram(IndicatorOutputId::ElderRayBear, "Bear Power", Color::hex(0xF44336)))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elder_ray_creation() {
        let er = ElderRay::new(13);
        assert!(!er.is_ready());
        assert_eq!(er.bull(), 0.0);
        assert_eq!(er.bear(), 0.0);
        assert_eq!(er.period(), 13);
    }

    #[test]
    fn test_elder_ray_uptrend() {
        let mut er = ElderRay::new(10);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            er.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(er.is_ready());
        let (bull, bear) = (er.bull(), er.bear());
        // In uptrend: high > EMA, so bull > 0
        assert!(bull > 0.0, "Bull power should be positive in uptrend, got {}", bull);
        // Bear power = low - EMA, could be negative or positive
        assert!(bear.is_finite());
    }

    #[test]
    fn test_elder_ray_downtrend() {
        let mut er = ElderRay::new(10);
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            er.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(er.is_ready());
        let (bull, bear) = (er.bull(), er.bear());
        // In downtrend: low < EMA, so bear < 0
        assert!(bear < 0.0, "Bear power should be negative in downtrend, got {}", bear);
        assert!(bull.is_finite());
    }

    #[test]
    fn test_elder_ray_reset() {
        let mut er = ElderRay::new(10);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            er.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(er.is_ready());
        er.reset();
        assert!(!er.is_ready());
        assert_eq!(er.bull(), 0.0);
        assert_eq!(er.bear(), 0.0);
    }

    #[test]
    fn test_elder_ray_finite_values() {
        let mut er = ElderRay::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let (bull, bear) = er.feed(&[price + 2.0, price - 2.0, price]);
            assert!(bull.is_finite(), "Bull power should always be finite");
            assert!(bear.is_finite(), "Bear power should always be finite");
        }
    }

    #[test]
    fn test_elder_ray_contract_create() {
        let cfg = <<ElderRay as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.period.resolved(), 14);
        let mut er = <ElderRay as Indicator>::create(cfg);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            er.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(er.is_ready());
    }
}
