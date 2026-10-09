// Klinger Volume Oscillator (KVO): volume force smoothed by fast/slow MAs + a signal MA.

use crate::engine::contract_engine::{SmootherSlot, SmootherId};

/// Klinger Volume Oscillator. Signed volume (by the sign of the HLC3 change) is smoothed
/// by a fast and a slow MA; `KVO = FastMA - SlowMA`, and a signal MA smooths the KVO.
/// Three independent smoother slots (fast/slow/signal each own their period + shape).
#[derive(Clone, Debug)]
pub struct Kvo {
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    fast_ema: SmootherSlot,
    slow_ema: SmootherSlot,
    signal_ema: SmootherSlot,
    prev_hlc3: f64,
    kvo_value: f64,
    signal_value: f64,
    ready: bool,
    count: usize,
}

impl Kvo {
    /// Default ctor — all three MAs are EMA.
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self::from_smoothers(SmootherId::Ema, fast_period, slow_period, signal_period)
    }

    /// Build the three MAs from one narrow `SmootherId`.
    /// Legacy bridge; the contract path goes through `KvoConfig`.
    pub fn from_smoothers(ma: SmootherId, fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self {
            fast_period,
            slow_period,
            signal_period,
            fast_ema: SmootherSlot::new(ma, fast_period.max(1)),
            slow_ema: SmootherSlot::new(ma, slow_period.max(2)),
            signal_ema: SmootherSlot::new(ma, signal_period.max(1)),
            prev_hlc3: 0.0,
            kvo_value: 0.0,
            signal_value: 0.0,
            ready: false,
            count: 0,
        }
    }

    /// Feed the resolved `[high, low, close, volume]` lanes (in `SOURCE` order).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let volume = lanes[3];
        let hlc3 = (high + low + close) / 3.0;
        let vol = if self.count == 0 {
            self.prev_hlc3 = hlc3;
            self.count += 1;
            0.0
        } else if hlc3 > self.prev_hlc3 {
            self.prev_hlc3 = hlc3;
            self.count += 1;
            volume
        } else if hlc3 < self.prev_hlc3 {
            self.prev_hlc3 = hlc3;
            self.count += 1;
            -volume
        } else {
            self.prev_hlc3 = hlc3;
            self.count += 1;
            0.0
        };
        let fast = self.fast_ema.feed(vol);
        let slow = self.slow_ema.feed(vol);
        self.kvo_value = fast - slow;
        if self.slow_ema.is_ready() {
            self.signal_value = self.signal_ema.feed(self.kvo_value);
            self.ready = self.signal_ema.is_ready();
        }
        self.kvo_value
    }
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn reset(&mut self) {
        self.fast_ema.reset();
        self.slow_ema.reset();
        self.signal_ema.reset();
        self.prev_hlc3 = 0.0;
        self.kvo_value = 0.0;
        self.signal_value = 0.0;
        self.ready = false;
        self.count = 0;
    }

    pub fn fast_period(&self) -> usize {
        self.fast_period
    }

    pub fn slow_period(&self) -> usize {
        self.slow_period
    }

    pub fn signal_period(&self) -> usize {
        self.signal_period
    }

    /// Named output: brace `line` (the KVO line itself, FastMA - SlowMA of signed volume).
    pub fn line(&self) -> f64 { self.kvo_value }
    /// Named output: brace `signal` (the signal-MA of the KVO line).
    pub fn signal(&self) -> f64 { self.signal_value }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};

/// Typed contract config for [`Kvo`] — three independent smoother slots plus their periods.
/// Volume-driven (`NEEDS_VOLUME`); reads the raw high/low/close (HLC3 direction).
/// Each slot is `Param<SmootherSlotOrder>` carrying its own member + period; defaults are
/// EMA at the respective host period (fast_period=34, slow_period=55, signal_period=13).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct KvoConfig {
    pub fast_period: Param<usize>,
    pub slow_period: Param<usize>,
    pub signal_period: Param<usize>,
    #[slot]
    pub fast: Param<SmootherSlotOrder>,
    #[slot]
    pub slow: Param<SmootherSlotOrder>,
    #[slot]
    pub signal: Param<SmootherSlotOrder>,
}

impl Indicator for Kvo {
    const ID: IndicatorId = IndicatorId::Kvo;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads raw high/low/close for the HLC3 direction PLUS volume (signed by direction).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
    /// O(1): signed-volume force through three smoothers; all buffers land recursively
    /// through `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = KvoConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::flow(IndicatorOutputId::KvoLine),
        Output::flow(IndicatorOutputId::KvoSignal),
    ];
    type Config = KvoConfig;
    type Runtime = Kvo;

    fn create(cfg: KvoConfig) -> Kvo {
        // The slot order carries its own period. `fast_period`/`slow_period`/`signal_period`
        // remain host axes for the Kvo struct fields (warmup tracking).
        let fast_slot = cfg.fast.resolved().into_slot();
        let slow_slot = cfg.slow.resolved().into_slot();
        let signal_slot = cfg.signal.resolved().into_slot();
        let fp = fast_slot.period().max(1);
        let sp = slow_slot.period().max(2);
        let sigp = signal_slot.period().max(1);
        Kvo {
            fast_period: fp,
            slow_period: sp,
            signal_period: sigp,
            fast_ema: fast_slot,
            slow_ema: slow_slot,
            signal_ema: signal_slot,
            prev_hlc3: 0.0,
            kvo_value: 0.0,
            signal_value: 0.0,
            ready: false,
            count: 0,
        }
    }

    fn slot_members(cfg: &KvoConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for KvoConfig {
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast_period.resolved();
        let slow = self.slow_period.resolved();
        if fast >= slow {
            return Err(format!("fast_period({fast}) >= slow_period({slow})"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        KvoConfig {
            fast_period: Param::Solo(34),
            slow_period: Param::Solo(55),
            signal_period: Param::Solo(13),
            fast: Param::Solo(SmootherSlotOrder::Ema(PeriodConfig { period: 34 })),
            slow: Param::Solo(SmootherSlotOrder::Ema(PeriodConfig { period: 55 })),
            signal: Param::Solo(SmootherSlotOrder::Ema(PeriodConfig { period: 13 })),
        }
    }
    fn machine_defaults() -> Self {
        // fast_period/slow_period/signal_period: Class A — auto gives range(2,4048,1) EACH,
        // but fast_period and slow_period then both resolve to the same min (2), failing this
        // config's OWN `valid_params` (fast < slow) at the min corner (2026-07-03 fix). Split
        // fast/slow into disjoint ranges so `resolved()` stays ordered; signal_period (an
        // independent lane) keeps the full auto range.
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


impl Render for Kvo {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::KvoLine, "KVO", Color::hex(0x2196F3), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::KvoSignal, "Signal", Color::hex(0xFF9800), 1.0))
            .zero_baseline()
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kvo_creation() {
        let kvo = Kvo::new(34, 55, 13);
        assert!(!kvo.is_ready());
        assert_eq!(kvo.line(), 0.0);
    }

    #[test]
    fn test_kvo_smoother_choice() {
        let kvo = Kvo::from_smoothers(SmootherId::Sma, 34, 55, 13);
        assert!(!kvo.is_ready());
    }

    #[test]
    fn test_kvo_basic_calculation() {
        let mut kvo = Kvo::new(34, 55, 13);
        for i in 1..=100 {
            let price = 100.0 + i as f64;
            let value = kvo.feed(&[price + 2.0, price - 1.0, price + 1.0, 10000.0 * (1.0 + (i as f64 * 0.1).sin())]);
            if kvo.is_ready() {
                assert!(value.is_finite());
            }
        }
        assert!(kvo.is_ready());
    }

    #[test]
    fn test_kvo_uptrend_with_volume() {
        let mut kvo = Kvo::new(34, 55, 13);
        for i in 1..=100 {
            let price = 100.0 + i as f64;
            kvo.feed(&[price + 1.0, price - 0.5, price + 0.5, 10000.0 + i as f64 * 100.0]);
        }
        if kvo.is_ready() {
            assert!(kvo.line().is_finite());
        }
    }

    #[test]
    fn test_kvo_downtrend_with_volume() {
        let mut kvo = Kvo::new(34, 55, 13);
        for i in 1..=100 {
            let price = 200.0 - i as f64;
            kvo.feed(&[price + 0.5, price - 1.0, price - 0.5, 10000.0 + i as f64 * 100.0]);
        }
        if kvo.is_ready() {
            assert!(kvo.line().is_finite());
        }
    }

    #[test]
    fn test_kvo_reset() {
        let mut kvo = Kvo::new(34, 55, 13);
        for i in 1..=100 {
            let price = 100.0 + i as f64;
            kvo.feed(&[price + 1.0, price - 1.0, price, 10000.0]);
        }
        assert!(kvo.is_ready());
        kvo.reset();
        assert!(!kvo.is_ready());
        assert_eq!(kvo.line(), 0.0);
    }

    #[test]
    fn test_kvo_periods() {
        let kvo = Kvo::new(34, 55, 13);
        assert_eq!(kvo.fast_period(), 34);
        assert_eq!(kvo.slow_period(), 55);
        assert_eq!(kvo.signal_period(), 13);
    }

    #[test]
    fn test_kvo_no_volume() {
        let mut kvo = Kvo::new(34, 55, 13);
        for i in 1..=100 {
            let price = 100.0 + i as f64;
            kvo.feed(&[price + 1.0, price - 1.0, price, 0.0]);
        }
        assert!(kvo.line().is_finite());
    }

    #[test]
    fn test_kvo_contract_create() {
        let cfg = <<Kvo as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.fast_period, Param::Solo(34));
        let mut kvo = <Kvo as Indicator>::create(cfg);
        for i in 1..=120 {
            let price = 100.0 + i as f64;
            kvo.feed(&[price + 1.0, price - 1.0, price, 10000.0]);
        }
        assert!(kvo.is_ready());
    }
}
