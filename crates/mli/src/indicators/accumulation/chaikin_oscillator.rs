use crate::engine::contract_engine::{SmootherSlot, SmootherId};

/// Chaikin Oscillator: MA_fast(ADL) - MA_slow(ADL), where ADL = money flow volume cumulative
#[derive(Debug, Clone)]
pub struct ChaikinOscillator {
    fast_ma: SmootherSlot,
    slow_ma: SmootherSlot,
    adl_value: f64,
    value: f64,
}

impl Default for ChaikinOscillator {
    /// Factory default: fast = 3, slow = 10, EMA.
    fn default() -> Self {
        Self::from_smoothers(SmootherId::Ema, 3, SmootherId::Ema, 10)
    }
}

impl ChaikinOscillator {
    /// Create Chaikin Oscillator with default MA type (EMA)
    pub fn new(fast: usize, slow: usize) -> Self {
        Self::from_smoothers(SmootherId::Ema, fast, SmootherId::Ema, slow)
    }

    pub fn from_smoothers(fast_id: SmootherId, fast: usize, slow_id: SmootherId, slow: usize) -> Self {
        let f = fast.max(1);
        let s = slow.max(1);
        Self {
            fast_ma: SmootherSlot::new(fast_id, f),
            slow_ma: SmootherSlot::new(slow_id, s),
            adl_value: 0.0,
            value: 0.0,
        }
    }

    fn money_flow_multiplier(high: f64, low: f64, close: f64) -> f64 {
        let hl = (high - low).abs().max(1e-12);
        ((close - low) - (high - close)) / hl
    }

    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high   = lanes[0];
        let low    = lanes[1];
        let close  = lanes[2];
        let volume = lanes[3];
        let mfm = Self::money_flow_multiplier(high, low, close);
        self.adl_value += mfm * volume;
        let fast = self.fast_ma.feed(self.adl_value);
        let slow = self.slow_ma.feed(self.adl_value);
        self.value = fast - slow;
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.fast_ma.is_ready() && self.slow_ma.is_ready()
    }

    pub fn reset(&mut self) {
        self.fast_ma.reset();
        self.slow_ma.reset();
        self.adl_value = 0.0;
        self.value = 0.0;
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Config for [`ChaikinOscillator`] — two independent smoother slots (fast / slow) each
/// with their own period field. Default: fast EMA(3), slow EMA(10).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct ChaikinOscillatorConfig {
    pub fast_period: Param<usize>,
    pub slow_period: Param<usize>,
    #[slot]
    pub fast: Param<SmootherSlotOrder>,
    #[slot]
    pub slow: Param<SmootherSlotOrder>,
}

impl Indicator for ChaikinOscillator {
    const ID: IndicatorId = IndicatorId::Cho;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed fields: High, Low, Close, Volume — all required for ADL.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = ChaikinOscillatorConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Cho)];
    type Config = ChaikinOscillatorConfig;
    type Runtime = ChaikinOscillator;

    fn create(cfg: ChaikinOscillatorConfig) -> ChaikinOscillator {
        ChaikinOscillator {
            fast_ma: cfg.fast.resolved().into_slot(),
            slow_ma: cfg.slow.resolved().into_slot(),
            adl_value: 0.0,
            value: 0.0,
        }
    }

    fn slot_members(cfg: &ChaikinOscillatorConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for ChaikinOscillatorConfig {
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast_period.resolved();
        let slow = self.slow_period.resolved();
        if fast >= slow {
            return Err(format!("fast_period({fast}) >= slow_period({slow})"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        ChaikinOscillatorConfig {
            fast_period: Param::Solo(3),
            slow_period: Param::Solo(10),
            fast: Param::Solo(SmootherSlotOrder::Ema(PeriodConfig { period: 3 })),
            slow: Param::Solo(SmootherSlotOrder::Ema(PeriodConfig { period: 10 })),
        }
    }
    fn machine_defaults() -> Self {
        // fast_period, slow_period: Class A → auto range(2,4048,1) EACH, but both then resolve
        // to the same min (2), failing this config's OWN `valid_params` (fast < slow) at the
        // min corner (2026-07-03 fix). Split into disjoint ranges so `resolved()` stays ordered.
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


impl Render for ChaikinOscillator {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Cho, "Chaikin Osc", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn test_chaikin_oscillator_creation() {
        let co = ChaikinOscillator::new(3, 10);
        assert!(!co.is_ready());
        assert_eq!(co.value(), 0.0);
    }

    #[test]
    fn test_chaikin_oscillator_warmup() {
        let mut co = ChaikinOscillator::new(3, 10);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            co.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
        }
        assert!(co.is_ready());
    }

    #[test]
    fn test_chaikin_oscillator_values_finite() {
        let mut co = ChaikinOscillator::new(3, 10);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = co.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_chaikin_oscillator_reset() {
        let mut co = ChaikinOscillator::new(3, 10);
        for i in 0..20 {
            co.feed(&[105.0, 95.0, 100.0 + i as f64, 1000.0]);
        }
        co.reset();
        assert!(!co.is_ready());
        assert_eq!(co.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_cho() {
        let mut f = IndicatorOrder::Cho(<<ChaikinOscillator as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 1000.0,
            });
        }
        assert!(f.read(IndicatorOutputId::Cho).is_finite());
    }

    #[test]
    fn config_defaults_preserved() {
        let cfg = <ChaikinOscillatorConfig as crate::contract::Config>::defaults();
        assert_eq!(cfg.fast_period.resolved(), 3);
        assert_eq!(cfg.slow_period.resolved(), 10);
    }
}
