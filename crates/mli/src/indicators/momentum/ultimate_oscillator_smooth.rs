// Smoothed Ultimate Oscillator: EMA of existing UltimateOscillator

use crate::engine::contract_engine::{SmootherSlot, SmootherId};
use crate::indicators::momentum::ultimate_oscillator::UltimateOscillator;


#[derive(Debug, Clone)]
pub struct UltimateOscillatorSmooth {
    uo: UltimateOscillator,
    ema: SmootherSlot,
    value: f64,
}

impl UltimateOscillatorSmooth {
    pub fn new(p1: usize, p2: usize, p3: usize, smooth: usize) -> Self {
        Self::from_smoother(p1, p2, p3, smooth, SmootherId::Ema)
    }

    pub fn from_smoother(p1: usize, p2: usize, p3: usize, smooth: usize, id: SmootherId) -> Self {
        Self {
            uo: UltimateOscillator::with_periods(p1, p2, p3),
            ema: SmootherSlot::new(id, smooth.max(1)),
            value: 50.0,
        }
    }

    pub fn reset(&mut self) {
        self.uo.reset();
        self.ema.reset();
        self.value = 50.0;
    }

    pub fn is_ready(&self) -> bool {
        self.ema.is_ready()
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed bar lanes as [high, low, close] (Sources order matching UO's SOURCE).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let u = self.uo.feed(lanes);
        self.value = self.ema.feed(u);
        self.value
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderSpec, Slot};
use crate::engine::contract_engine::SmootherChoice;

/// Typed contract config for [`UltimateOscillatorSmooth`].
///
/// Dual-mode: every field is a `Param`. The `#[slot]` smoother `smooth` is a
/// `Param<SmootherChoice>` following `smooth_period` by default (EMA).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct UoSmoothConfig {
    pub period1: Param<usize>,
    pub period2: Param<usize>,
    pub period3: Param<usize>,
    pub smooth_period: Param<usize>,
    #[slot]
    pub smooth: Param<SmootherChoice>,
}

impl Indicator for UltimateOscillatorSmooth {
    const ID: IndicatorId = IndicatorId::UoSmooth;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    // UO uses High, Low, Close — same KlineSlice as the inner UO.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Uo, &[IndicatorOutputId::Uo])],
    };
    const SLOTS: &'static [Slot] = UoSmoothConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::UoSmooth)];
    type Config = UoSmoothConfig;
    type Runtime = UltimateOscillatorSmooth;

    fn create(cfg: UoSmoothConfig) -> UltimateOscillatorSmooth {
        let smooth_period = cfg.smooth_period.resolved();
        UltimateOscillatorSmooth {
            uo: UltimateOscillator::with_periods(
                cfg.period1.resolved(),
                cfg.period2.resolved(),
                cfg.period3.resolved(),
            ),
            ema: cfg.smooth.resolved().build(smooth_period),
            value: 50.0,
        }
    }

    fn slot_members(cfg: &UoSmoothConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for UoSmoothConfig {
    fn valid_params(&self) -> Result<(), String> {
        let p1 = self.period1.resolved();
        let p2 = self.period2.resolved();
        let p3 = self.period3.resolved();
        if !(p1 < p2 && p2 < p3) {
            return Err(format!("period1({p1}) < period2({p2}) < period3({p3}) required"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        UoSmoothConfig {
            period1: Param::Solo(7),
            period2: Param::Solo(14),
            period3: Param::Solo(28),
            smooth_period: Param::Solo(9),
            smooth: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period1/period2/period3/smooth_period: Class A → auto range(2,4048,1) EACH, but
        // period1/period2/period3 then all resolve to the same min (2), failing this config's
        // OWN `valid_params` (period1 < period2 < period3) at the min corner (2026-07-03 fix).
        // Split into three disjoint ranges so `resolved()` stays strictly ordered;
        // smooth_period (an independent lane) keeps the full auto range.
        // smooth: #[slot] SmootherChoice → left Solo (deferred wave).
        let mut s = Self::machine_defaults_auto();
        s.period1 = Param::range(1, 50, 1);
        s.period2 = Param::range(51, 200, 1);
        s.period3 = Param::range(201, 10000, 1);
        s
    }
}


impl Render for UltimateOscillatorSmooth {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::UoSmooth, "UO Smooth", Color::hex(0x3F51B5))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(70.0, Color::hex(0xFF5722)))
            .reference_line(ReferenceLine::new(30.0, Color::hex(0x4CAF50)))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uo_smooth_creation() {
        let uo = UltimateOscillatorSmooth::new(7, 14, 28, 9);
        assert!(!uo.is_ready());
        assert_eq!(uo.value(), 50.0);
    }

    #[test]
    fn test_uo_smooth_uptrend() {
        let mut uo = UltimateOscillatorSmooth::new(7, 14, 28, 9);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            uo.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(uo.is_ready());
        assert!(uo.value() > 50.0, "UO Smooth should be > 50 in uptrend, got {}", uo.value());
    }

    #[test]
    fn test_uo_smooth_downtrend() {
        let mut uo = UltimateOscillatorSmooth::new(7, 14, 28, 9);
        for i in 1..=50 {
            let price = 200.0 - i as f64 * 2.0;
            uo.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(uo.is_ready());
        assert!(uo.value() < 50.0, "UO Smooth should be < 50 in downtrend, got {}", uo.value());
    }

    #[test]
    fn test_uo_smooth_finite() {
        let mut uo = UltimateOscillatorSmooth::new(7, 14, 28, 9);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = uo.feed(&[price + 2.0, price - 2.0, price]);
            assert!(value.is_finite(), "UO Smooth should always be finite");
        }
    }

    #[test]
    fn test_uo_smooth_reset() {
        let mut uo = UltimateOscillatorSmooth::new(7, 14, 28, 9);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            uo.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(uo.is_ready());
        uo.reset();
        assert!(!uo.is_ready());
        assert_eq!(uo.value(), 50.0);
    }

    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<UltimateOscillatorSmooth as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::UoSmooth(cfg).build_solo().unwrap();
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            // open=9999.0 and volume=9999.0 prove only H/L/C are resolved
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: price + 1.0, low: price - 1.0, close: price, volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary() > 50.0);
    }
}

impl Default for UltimateOscillatorSmooth {
    fn default() -> Self {
        Self::new(7, 14, 28, 9)
    }
}
