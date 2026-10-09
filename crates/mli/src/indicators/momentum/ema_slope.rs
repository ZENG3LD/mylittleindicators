// EMA Slope: normalized slope of MA over lookback

use crate::engine::contract_engine::SmootherSlot;

/// Normalized slope of a smoothed price over a `lookback`. PURE core — owns no source;
/// the factory feeds it the resolved scalar via [`EmaSlope::feed`] (the chosen
/// `OhlcvField` lives on the `ContractFactory` variant, lifted via `source`).
#[derive(Debug, Clone)]
pub struct EmaSlope {
    ma_period: usize,
    ema: SmootherSlot,
    lookback: usize,
    buffer: Vec<f64>,
    idx: usize,
    filled: bool,
    slope: f64,
}

impl EmaSlope {
    /// Default ctor — price smoothed with EMA.
    pub fn new(ema_period: usize, lookback: usize) -> Self {
        Self::from_smoother(ema_period, lookback, SmootherId::Ema)
    }

    /// Build from a narrow `SmootherId` for the smoother + MA period + slope lookback.
    /// Legacy bridge; the contract path goes through `EmaSlopeConfig`.
    pub fn from_smoother(ma_period: usize, lookback: usize, smoother: SmootherId) -> Self {
        let lb = lookback.max(1);
        Self {
            ma_period,
            ema: SmootherSlot::new(smoother, ma_period),
            lookback: lb,
            buffer: vec![0.0; lb],
            idx: 0,
            filled: false,
            slope: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ema.reset();
        self.buffer.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.slope = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ema.is_ready() && (self.filled || self.idx >= self.lookback)
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, value: f64) -> f64 {
        let v = self.ema.feed(value);
        self.buffer[self.idx % self.lookback] = v;
        self.idx += 1;
        if self.idx >= self.lookback {
            self.filled = true;
        }

        if self.is_ready() {
            let last = v;
            let first = self.buffer[(self.idx - self.lookback) % self.lookback];
            let denom = (self.lookback as f64).max(1.0);
            self.slope = (last - first) / denom;
        } else {
            self.slope = 0.0;
        }
        self.slope
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.slope
    }

    pub fn ma_period(&self) -> usize {
        self.ma_period
    }

    pub fn lookback(&self) -> usize {
        self.lookback
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

impl EmaSlope {
    /// Build an EmaSlope from a smoother CHOICE (kind + follow/own period) at the host `ma_period`.
    pub fn from_choice(choice: SmootherChoice, ma_period: usize, lookback: usize) -> Self {
        let lb = lookback.max(1);
        Self {
            ma_period,
            ema: choice.build(ma_period),
            lookback: lb,
            buffer: vec![0.0; lb],
            idx: 0,
            filled: false,
            slope: 0.0,
        }
    }
}

/// Typed contract config for [`EmaSlope`] — normalized slope of a smoothed price over a
/// `lookback`.
///
/// Dual-mode: every field is a `Param`. The `#[slot]` smoother is a `Param<SmootherChoice>`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct EmaSlopeConfig {
    pub ma_period: Param<usize>,
    pub lookback: Param<usize>,
    pub source: Param<OhlcvField>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl Indicator for EmaSlope {
    const ID: IndicatorId = IndicatorId::EmaSlope;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1) slope read over a `lookback`-deep heap ring of smoothed values; the smoother
    /// buffer cost lands recursively through `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Vec)]);
    const SLOTS: &'static [Slot] = EmaSlopeConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::EmaSlope)];
    type Config = EmaSlopeConfig;
    type Runtime = EmaSlope;

    fn create(cfg: EmaSlopeConfig) -> EmaSlope {
        let ma_period = cfg.ma_period.resolved();
        let lb = cfg.lookback.resolved().max(1);
        EmaSlope {
            ma_period,
            ema: cfg.ma.resolved().build(ma_period),
            lookback: lb,
            buffer: vec![0.0; lb],
            idx: 0,
            filled: false,
            slope: 0.0,
        }
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the resolved scalar.
    fn source_fields(cfg: &EmaSlopeConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &EmaSlopeConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for EmaSlopeConfig {
    fn defaults() -> Self {
        EmaSlopeConfig {
            ma_period: Param::Solo(21),
            lookback: Param::Solo(20),
            source: Param::Solo(OhlcvField::Close),
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
        // ma_period: Class A → auto range(2,4048,1); lookback: Class A → auto range(2,4048,1).
        // source: Class O → auto all-8; ma: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for EmaSlope {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::EmaSlope, "EMA Slope", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_slope_creation() {
        let es = EmaSlope::new(13, 5);
        assert!(!es.is_ready());
        assert_eq!(es.value(), 0.0);
        assert_eq!(es.ma_period(), 13);
        assert_eq!(es.lookback(), 5);
    }

    #[test]
    fn test_ema_slope_with_smoother() {
        let es = EmaSlope::from_smoother(13, 5, SmootherId::Sma);
        assert_eq!(es.ma_period(), 13);
        assert_eq!(es.lookback(), 5);
    }

    #[test]
    fn test_ema_slope_uptrend() {
        let mut es = EmaSlope::new(10, 5);
        for i in 1..=30 {
            es.feed(100.0 + i as f64 * 2.0);
        }
        assert!(es.is_ready());
        // In uptrend, slope should be positive
        assert!(es.value() > 0.0, "EMA slope should be positive in uptrend, got {}", es.value());
    }

    #[test]
    fn test_ema_slope_downtrend() {
        let mut es = EmaSlope::new(10, 5);
        for i in 1..=30 {
            es.feed(200.0 - i as f64 * 2.0);
        }
        assert!(es.is_ready());
        // In downtrend, slope should be negative
        assert!(es.value() < 0.0, "EMA slope should be negative in downtrend, got {}", es.value());
    }

    #[test]
    fn test_ema_slope_reset() {
        let mut es = EmaSlope::new(10, 5);
        for i in 1..=30 {
            es.feed(100.0 + i as f64);
        }
        assert!(es.is_ready());
        es.reset();
        assert!(!es.is_ready());
        assert_eq!(es.value(), 0.0);
    }

    #[test]
    fn test_ema_slope_finite_values() {
        let mut es = EmaSlope::new(10, 5);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = es.feed(price);
            assert!(value.is_finite(), "EMA slope should always be finite");
        }
    }

    #[test]
    fn test_ema_slope_contract_create() {
        let cfg = <<EmaSlope as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.ma_period.resolved(), 21);
        let mut es = <EmaSlope as Indicator>::create(cfg);
        for i in 1..=60 {
            es.feed(100.0 + i as f64 * 2.0);
        }
        assert!(es.is_ready());
    }
}
