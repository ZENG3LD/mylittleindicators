// Volume Oscillator - difference between a fast and a slow moving average of VOLUME.

use crate::engine::contract_engine::{SmootherSlot, SmootherId};

/// Volume Oscillator = `FastMA(volume) - SlowMA(volume)`. Two independent smoother
/// slots over the volume stream (volume-driven, no price field).
#[derive(Debug, Clone)]
pub struct VolumeOscillator {
    fast: SmootherSlot,
    slow: SmootherSlot,
    value: f64,
}

impl VolumeOscillator {
    /// Default ctor — both MAs are EMA.
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self::from_smoothers(SmootherId::Ema, fast_period, SmootherId::Ema, slow_period)
    }

    /// Build from narrow `SmootherId`s for the fast + slow volume MAs.
    /// Legacy bridge; the contract path goes through `VoConfig`.
    pub fn from_smoothers(fast: SmootherId, fast_period: usize, slow: SmootherId, slow_period: usize) -> Self {
        Self {
            fast: SmootherSlot::new(fast, fast_period.max(1)),
            slow: SmootherSlot::new(slow, slow_period.max(2)),
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
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed ONE resolved scalar (volume) — the factory extracts the Volume field.
    pub fn feed(&mut self, v: f64) -> f64 {
        let f = self.fast.feed(v);
        let s = self.slow.feed(v);
        self.value = f - s;
        self.value
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`VolumeOscillator`] — fast/slow MAs of volume. Two
/// independent smoother slots plus their period fields; volume-driven (`NEEDS_VOLUME`).
/// `fast` and `slow` are `Param<SmootherSlotOrder>`: each carries its own member + period.
/// Default = EMA at the respective host period (fast_period/slow_period). The slot period
/// is its own axis swept by `SmootherSlotOrder::machine_sweep()`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct VoConfig {
    pub fast_period: Param<usize>,
    pub slow_period: Param<usize>,
    #[slot]
    pub fast: Param<SmootherSlotOrder>,
    #[slot]
    pub slow: Param<SmootherSlotOrder>,
}

impl Indicator for VolumeOscillator {
    const ID: IndicatorId = IndicatorId::Vo;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed Volume series — the factory extracts the Volume field and feeds the scalar.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::Volume]));
    const NEEDS_VOLUME: bool = true;
    /// O(1): difference of two volume smoothers; both buffers land recursively via `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = VoConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Vo)];
    type Config = VoConfig;
    type Runtime = VolumeOscillator;

    fn create(cfg: VoConfig) -> VolumeOscillator {
        // The slot order carries its own period — no host-period resolve needed.
        // `fast_period`/`slow_period` remain the host axis for their own buffers/warmup.
        VolumeOscillator {
            fast: cfg.fast.resolved().into_slot(),
            slow: cfg.slow.resolved().into_slot(),
            value: 0.0,
        }
    }

    fn slot_members(cfg: &VoConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for VoConfig {
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast_period.resolved();
        let slow = self.slow_period.resolved();
        if fast >= slow {
            return Err(format!("fast_period({fast}) >= slow_period({slow})"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        VoConfig {
            fast_period: Param::Solo(5),
            slow_period: Param::Solo(10),
            fast: Param::Solo(SmootherSlotOrder::Ema(PeriodConfig { period: 5 })),
            slow: Param::Solo(SmootherSlotOrder::Ema(PeriodConfig { period: 10 })),
        }
    }
    fn machine_defaults() -> Self {
        // fast_period/slow_period: Class A — auto gives range(2,4048,1) EACH, but both then
        // resolve to the same min (2), failing this config's OWN `valid_params` (fast < slow)
        // at the min corner (2026-07-03 fix). Split into disjoint ranges so `resolved()` stays
        // ordered.
        let mut s = Self::machine_defaults_auto();
        s.fast_period = Param::range(1, 100, 1);
        s.slow_period = Param::range(101, 10000, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VolumeOscillator {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vo, "Vol Osc", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_oscillator_creation() {
        let vo = VolumeOscillator::new(5, 10);
        assert!(!vo.is_ready());
        assert_eq!(vo.value(), 0.0);
    }

    #[test]
    fn test_volume_oscillator_warmup() {
        let mut vo = VolumeOscillator::new(5, 10);
        for i in 0..15 {
            let volume = 1000.0 + (i as f64 * 0.1).sin() * 100.0;
            vo.feed(volume);
        }
        assert!(vo.is_ready());
    }

    #[test]
    fn test_volume_oscillator_values() {
        let mut vo = VolumeOscillator::new(5, 10);
        for i in 0..20 {
            let volume = 1000.0 + i as f64 * 50.0;
            let value = vo.feed(volume);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_volume_oscillator_reset() {
        let mut vo = VolumeOscillator::new(5, 10);
        for i in 0..20 {
            vo.feed(1000.0 + i as f64 * 10.0);
        }
        vo.reset();
        assert!(!vo.is_ready());
        assert_eq!(vo.value(), 0.0);
    }

    #[test]
    fn test_volume_oscillator_contract_create() {
        let cfg = <<VolumeOscillator as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut vo = <VolumeOscillator as Indicator>::create(cfg);
        for i in 0..20 {
            vo.feed(1000.0 + i as f64 * 50.0);
        }
        assert!(vo.is_ready());
    }
}
