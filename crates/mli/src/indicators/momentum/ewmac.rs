// EWMAC (Exponential Weighted Moving Average Crossover) signal

use crate::engine::contract_engine::SmootherSlot;
use crate::engine::ohlcv_field::OhlcvField;

/// EWMAC = FastMA - SlowMA (trend-following crossover). Two independent smoother slots.
#[derive(Debug, Clone)]
pub struct Ewmac {
    fast: SmootherSlot,
    slow: SmootherSlot,
    value: f64,
}

impl Ewmac {
    /// Default ctor — both MAs are EMA over close.
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self::from_smoothers(SmootherId::Ema, fast_period, SmootherId::Ema, slow_period)
    }

    /// Build from narrow `SmootherId`s for the fast + slow MAs (over close).
    /// Legacy bridge; the contract path goes through `EwmacConfig`.
    pub fn from_smoothers(fast: SmootherId, fast_period: usize, slow: SmootherId, slow_period: usize) -> Self {
        Self {
            fast: SmootherSlot::new(fast, fast_period.max(1)),
            slow: SmootherSlot::new(slow, slow_period.max(1)),
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.fast.reset();
        self.slow.reset();
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.fast.is_ready() && self.slow.is_ready()
    }

    /// Feed ONE resolved scalar (the configured source field). MA-crossover fast-slow.
    pub fn feed(&mut self, v: f64) -> f64 {
        let f = self.fast.feed(v);
        let s = self.slow.feed(v);
        self.value = f - s;
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`Ewmac`] — `FastMA - SlowMA` trend crossover. Two independent
/// smoother slots, each following its lane's period field; `source` is the price field.
///
/// Dual-mode: every field is a `Param`. The two `#[slot]` smoothers are `Param<SmootherChoice>`
/// that follow their respective period fields (fast=16, slow=64 by default).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct EwmacConfig {
    pub source: Param<OhlcvField>,
    pub fast_period: Param<usize>,
    pub slow_period: Param<usize>,
    #[slot]
    pub fast: Param<SmootherChoice>,
    #[slot]
    pub slow: Param<SmootherChoice>,
}

impl Indicator for Ewmac {
    const ID: IndicatorId = IndicatorId::Ewmac;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1): difference of two smoothers; both buffers land recursively through `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = EwmacConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Ewmac)];
    type Config = EwmacConfig;
    type Runtime = Ewmac;

    fn create(cfg: EwmacConfig) -> Ewmac {
        let fast_period = cfg.fast_period.resolved().max(1);
        let slow_period = cfg.slow_period.resolved().max(1);
        Ewmac {
            fast: cfg.fast.resolved().build(fast_period),
            slow: cfg.slow.resolved().build(slow_period),
            value: 0.0,
        }
    }

    fn slot_members(cfg: &EwmacConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }

    /// Field-source core: the factory holds `cfg.source` and feeds the resolved scalar.
    fn source_fields(cfg: &EwmacConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for EwmacConfig {
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast_period.resolved();
        let slow = self.slow_period.resolved();
        if fast >= slow {
            return Err(format!("fast_period({fast}) >= slow_period({slow})"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        EwmacConfig {
            source: Param::Solo(OhlcvField::Close),
            fast_period: Param::Solo(16),
            slow_period: Param::Solo(64),
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


impl Render for Ewmac {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Ewmac, "EWMAC", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ewmac_creation() {
        let ewmac = Ewmac::new(8, 32);
        assert!(!ewmac.is_ready());
        assert_eq!(ewmac.value(), 0.0);
    }

    #[test]
    fn test_ewmac_uptrend() {
        let mut ewmac = Ewmac::new(8, 32);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            ewmac.feed(price);
        }
        assert!(ewmac.is_ready());
        assert!(ewmac.value() > 0.0, "EWMAC should be positive in uptrend, got {}", ewmac.value());
    }

    #[test]
    fn test_ewmac_downtrend() {
        let mut ewmac = Ewmac::new(8, 32);
        for i in 1..=50 {
            let price = 200.0 - i as f64 * 2.0;
            ewmac.feed(price);
        }
        assert!(ewmac.is_ready());
        assert!(ewmac.value() < 0.0, "EWMAC should be negative in downtrend, got {}", ewmac.value());
    }

    #[test]
    fn test_ewmac_reset() {
        let mut ewmac = Ewmac::new(8, 32);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            ewmac.feed(price);
        }
        assert!(ewmac.is_ready());
        ewmac.reset();
        assert!(!ewmac.is_ready());
        assert_eq!(ewmac.value(), 0.0);
    }

    #[test]
    fn test_ewmac_finite_values() {
        let mut ewmac = Ewmac::new(8, 32);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = ewmac.feed(price);
            assert!(value.is_finite(), "EWMAC should always be finite");
        }
    }

    #[test]
    fn test_ewmac_contract_create() {
        let cfg = <<Ewmac as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.fast_period.resolved(), 16);
        assert_eq!(cfg.slow_period.resolved(), 64);
        let mut ewmac = <Ewmac as Indicator>::create(cfg);
        for i in 1..=80 {
            let price = 100.0 + i as f64 * 2.0;
            ewmac.feed(price);
        }
        assert!(ewmac.is_ready());
    }
}
