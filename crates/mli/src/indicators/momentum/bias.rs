// High-performance Bias indicator
// (c) 2024

use crate::engine::contract_engine::SmootherSlot;

/// Bias = `price / MA(price) - 1`. PURE core — owns no source; the factory feeds it the
/// resolved scalar via [`Bias::feed`] (the chosen `OhlcvField` lives on the
/// `ContractFactory` variant, lifted from the config via `source`).
#[derive(Debug, Clone)]
pub struct Bias {
    ma: SmootherSlot,
    period: usize,
    value: f64,
    filled: bool,
}

impl Bias {
    /// Default ctor — price smoothed with SMA.
    pub fn new(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Sma)
    }

    /// Build from a narrow `SmootherId` for the smoother + period.
    /// Legacy bridge; the contract path goes through `BiasConfig`.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        Self {
            ma: SmootherSlot::new(smoother, period),
            period,
            value: 0.0,
            filled: false,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, value: f64) -> f64 {
        self.ma.feed(value);
        if self.ma.is_ready() {
            let ma = self.ma.value();
            if ma.abs() < 1e-12 {
                self.value = 0.0;
            } else {
                self.value = value / ma - 1.0;
            }
            self.filled = true;
        } else {
            self.value = 0.0;
            self.filled = false;
        }
        self.value
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Returns the current Bias value as `f64` (the scalar-slot interface).
    pub fn value_f64(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn reset(&mut self) {
        self.ma.reset();
        self.value = 0.0;
        self.filled = false;
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

impl crate::contract::Oscillator for Bias {
    type Params = crate::indicators::average::moving_average::PeriodConfig;
    fn from_params(p: Self::Params) -> Self {
        Bias::new(p.period)
    }
    fn params_period(p: &Self::Params) -> usize {
        p.period
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

impl Bias {
    /// Build a Bias from a smoother CHOICE (kind + follow/own period) at the host `period`.
    pub fn from_choice(choice: SmootherChoice, period: usize) -> Self {
        Self {
            ma: choice.build(period),
            period,
            value: 0.0,
            filled: false,
        }
    }
}

/// Typed contract config for [`Bias`] — `price / MA(price) - 1`. Dual-mode: every field
/// is a `Param`. The `#[slot]` smoother is a `Param<SmootherChoice>`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct BiasConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl Indicator for Bias {
    const ID: IndicatorId = IndicatorId::Bias;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1): one ratio over the smoothed price; the smoother buffer cost lands via the slot.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = BiasConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Bias)];
    type Config = BiasConfig;
    type Runtime = Bias;

    fn create(cfg: BiasConfig) -> Bias {
        Bias::from_choice(cfg.ma.resolved(), cfg.period.resolved())
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the resolved scalar.
    fn source_fields(cfg: &BiasConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &BiasConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for BiasConfig {
    fn defaults() -> Self {
        BiasConfig {
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
        // period: Class A → auto; source: Class O → auto all-8.
        // ma: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for Bias {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Bias, "Bias", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bias_creation() {
        let bias = Bias::new(14);
        assert!(!bias.is_ready());
        assert_eq!(bias.value(), 0.0);
        assert_eq!(bias.period(), 14);
    }

    #[test]
    fn test_bias_with_ema() {
        let bias = Bias::from_smoother(10, SmootherId::Ema);
        assert!(!bias.is_ready());
        assert_eq!(bias.period(), 10);
    }

    #[test]
    fn test_bias_basic_calculation() {
        let mut bias = Bias::new(5);
        // Feed constant price - bias should be 0
        for _ in 0..10 {
            bias.feed(100.0);
        }
        assert!(bias.is_ready());
        // close/ma - 1 = 100/100 - 1 = 0
        assert!((bias.value()).abs() < 1e-10, "Bias should be 0 for constant prices");
    }

    #[test]
    fn test_bias_uptrend() {
        let mut bias = Bias::new(5);
        for i in 1..=20 {
            bias.feed(100.0 + i as f64 * 2.0);
        }
        assert!(bias.is_ready());
        // In uptrend, close > MA, so bias > 0
        assert!(bias.value() > 0.0, "Bias should be positive in uptrend");
    }

    #[test]
    fn test_bias_downtrend() {
        let mut bias = Bias::new(5);
        for i in 1..=20 {
            bias.feed(200.0 - i as f64 * 2.0);
        }
        assert!(bias.is_ready());
        // In downtrend, close < MA, so bias < 0
        assert!(bias.value() < 0.0, "Bias should be negative in downtrend");
    }

    #[test]
    fn test_bias_reset() {
        let mut bias = Bias::new(5);
        for i in 1..=20 {
            bias.feed(100.0 + i as f64);
        }
        assert!(bias.is_ready());
        bias.reset();
        assert!(!bias.is_ready());
        assert_eq!(bias.value(), 0.0);
    }

    #[test]
    fn test_bias_is_ready_timing() {
        let mut bias = Bias::new(5);
        for i in 1..=10 {
            bias.feed(100.0 + i as f64);
            if i < 5 {
                assert!(!bias.is_ready(), "Bias should not be ready before period bars");
            } else {
                assert!(bias.is_ready(), "Bias should be ready after period bars");
            }
        }
    }

    #[test]
    fn test_bias_finite_values() {
        let mut bias = Bias::new(10);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = bias.feed(price);
            assert!(value.is_finite(), "Bias should always be finite");
        }
    }

    #[test]
    fn test_bias_contract_create() {
        let cfg = <<Bias as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        // default smoother is Follow(Sma) at host period 14
        let mut bias = <Bias as Indicator>::create(cfg);
        for i in 1..=20 {
            bias.feed(100.0 + i as f64);
        }
        assert!(bias.is_ready());
    }
}
