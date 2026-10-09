//! Absolute Price Oscillator (APO) indicator.

use crate::engine::contract_engine::SmootherSlot;

/// Absolute Price Oscillator (APO) - momentum from the difference of two MAs.
///
/// APO = Fast MA - Slow MA. The two MAs are independent slots (own period + shape each).
/// PURE core — owns no source; the factory feeds it the resolved scalar via [`Apo::feed`]
/// (the chosen `OhlcvField` lives on the `ContractFactory` variant, lifted via `source`).
#[derive(Debug, Clone)]
pub struct Apo {
    fast_ma: SmootherSlot,
    slow_ma: SmootherSlot,
    value: f64,
    ready: bool,
}

impl Apo {
    /// Default ctor — both MAs are EMA.
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self::from_smoothers(SmootherId::Ema, fast_period, SmootherId::Ema, slow_period)
    }

    /// Build from narrow `SmootherId`s for the fast + slow MAs.
    /// Legacy bridge; the contract path goes through `ApoConfig`.
    pub fn from_smoothers(fast: SmootherId, fast_period: usize, slow: SmootherId, slow_period: usize) -> Self {
        Self {
            fast_ma: SmootherSlot::new(fast, fast_period.max(1)),
            slow_ma: SmootherSlot::new(slow, slow_period.max(1)),
            value: 0.0,
            ready: false,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, value: f64) -> f64 {
        let fast = self.fast_ma.feed(value);
        let slow = self.slow_ma.feed(value);
        self.value = fast - slow;
        self.ready = self.fast_ma.is_ready() && self.slow_ma.is_ready();
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn reset(&mut self) {
        self.fast_ma.reset();
        self.slow_ma.reset();
        self.value = 0.0;
        self.ready = false;
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

/// Typed contract config for [`Apo`] — `FastMA - SlowMA`. Two independent smoother slots,
/// each following its lane's period field; the normal fast<slow lockstep is the upstream
/// resolver's job, not enforced here. `source` is the price field.
///
/// Dual-mode: every field is a `Param`. The `#[slot]` smoothers are `Param<SmootherChoice>`
/// that follow their respective period fields.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct ApoConfig {
    pub source: Param<OhlcvField>,
    pub fast_period: Param<usize>,
    pub slow_period: Param<usize>,
    #[slot]
    pub fast: Param<SmootherChoice>,
    #[slot]
    pub slow: Param<SmootherChoice>,
}

impl Indicator for Apo {
    const ID: IndicatorId = IndicatorId::Apo;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1): difference of two smoothers; both buffers land recursively through `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = ApoConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Apo)];
    type Config = ApoConfig;
    type Runtime = Apo;

    fn create(cfg: ApoConfig) -> Apo {
        let fast_period = cfg.fast_period.resolved().max(1);
        let slow_period = cfg.slow_period.resolved().max(1);
        Apo {
            fast_ma: cfg.fast.resolved().build(fast_period),
            slow_ma: cfg.slow.resolved().build(slow_period),
            value: 0.0,
            ready: false,
        }
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the resolved scalar.
    fn source_fields(cfg: &ApoConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &ApoConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for ApoConfig {
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast_period.resolved();
        let slow = self.slow_period.resolved();
        if fast >= slow {
            return Err(format!("fast_period({fast}) >= slow_period({slow})"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        ApoConfig {
            source: Param::Solo(OhlcvField::Close),
            fast_period: Param::Solo(12),
            slow_period: Param::Solo(26),
            fast: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            slow: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // fast_period/slow_period: Class A → auto range(2,4048,1) EACH, but both then resolve
        // to the same min (2), failing this config's OWN `valid_params` (fast < slow) at the
        // min corner (2026-07-03 fix). Split into disjoint ranges so `resolved()` stays ordered.
        // source: Class O → auto all-8.
        // fast/slow: #[slot] SmootherChoice → left Solo (deferred wave).
        let mut s = Self::machine_defaults_auto();
        s.fast_period = Param::range(1, 100, 1);
        s.slow_period = Param::range(101, 10000, 1);
        s
    }
}


impl Render for Apo {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Apo, "APO", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apo_basic_calculation() {
        let mut apo = Apo::new(12, 26);
        for i in 1..=50 {
            apo.feed(100.0 + i as f64);
        }
        assert!(apo.is_ready());
        assert!(apo.value() > 0.0, "APO in uptrend should be positive");
    }

    #[test]
    fn test_apo_downtrend() {
        let mut apo = Apo::new(12, 26);
        for i in 1..=50 {
            apo.feed(200.0 - i as f64);
        }
        assert!(apo.is_ready());
        assert!(apo.value() < 0.0, "APO in downtrend should be negative");
    }

    #[test]
    fn test_apo_constant_price() {
        let mut apo = Apo::new(5, 10);
        for _ in 1..=30 {
            apo.feed(100.0);
        }
        assert!(apo.is_ready());
        assert!(apo.value().abs() < 0.01, "APO with constant price should be near 0");
    }

    #[test]
    fn test_apo_reset() {
        let mut apo = Apo::new(12, 26);
        for i in 1..=50 {
            apo.feed(100.0 + i as f64);
        }
        assert!(apo.is_ready());
        apo.reset();
        assert!(!apo.is_ready());
        assert!(apo.value().abs() < 1e-10);
    }

    #[test]
    fn test_apo_with_sma() {
        let mut apo = Apo::from_smoothers(SmootherId::Sma, 5, SmootherId::Sma, 10);
        for i in 1..=20 {
            apo.feed(100.0 + i as f64);
        }
        assert!(apo.is_ready());
        assert!(apo.value() > 0.0);
    }

    #[test]
    fn test_apo_mixed_smoothers() {
        // fast EMA, slow SMA — independent slot shapes.
        let mut apo = Apo::from_smoothers(SmootherId::Ema, 5, SmootherId::Sma, 10);
        for i in 1..=30 {
            apo.feed(100.0 + i as f64);
        }
        assert!(apo.is_ready());
        assert!(apo.value().is_finite());
    }

    #[test]
    fn test_apo_contract_create() {
        let cfg = <<Apo as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.fast_period.resolved(), 12);
        assert_eq!(cfg.slow_period.resolved(), 26);
        let mut apo = <Apo as Indicator>::create(cfg);
        for i in 1..=50 {
            apo.feed(100.0 + i as f64);
        }
        assert!(apo.is_ready());
    }
}
