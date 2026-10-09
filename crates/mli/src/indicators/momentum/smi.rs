use crate::engine::contract_engine::{SmootherSlot, SmootherId};

/// Stochastic Momentum Index (SMI): SMI = 100 * (MA(MA(close - mid)) / (0.5 * MA(MA(high-low)))).
/// The double-smoother (applied to BOTH the price-diff and the range legs) and the signal-line
/// smoother are both configurable.
#[derive(Debug, Clone)]
pub struct Smi {
    ema1_diff: SmootherSlot,
    ema2_diff: SmootherSlot,
    ema1_range: SmootherSlot,
    ema2_range: SmootherSlot,
    signal_ma: SmootherSlot,
    value: f64,
    signal: f64,
    ready: bool,
}

impl Smi {
    /// Default ctor — EMA double-smoothing + EMA signal (the classic SMI).
    pub fn new(period: usize, signal_period: usize) -> Self {
        Self::from_smoothers(SmootherId::Ema, period, SmootherId::Ema, signal_period)
    }

    /// Build from narrow `SmootherId`s for the double-smoother (4 inner legs, shared period)
    /// and the signal smoother. Legacy bridge; the contract path goes through `SmiConfig`.
    pub fn from_smoothers(double: SmootherId, period: usize, signal: SmootherId, signal_period: usize) -> Self {
        let p = period.max(1);
        Self {
            ema1_diff: SmootherSlot::new(double, p),
            ema2_diff: SmootherSlot::new(double, p),
            ema1_range: SmootherSlot::new(double, p),
            ema2_range: SmootherSlot::new(double, p),
            signal_ma: SmootherSlot::new(signal, signal_period.max(1)),
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// Feed the resolved input lanes — `[high, low, close]` (the factory resolves the fixed
    /// HLC slice). Knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        let c = lanes[2];
        let mid = 0.5 * (h + l);
        let diff = c - mid;
        let range = (h - l).max(1e-12);
        let d1 = self.ema1_diff.feed(diff);
        let d2 = self.ema2_diff.feed(d1);
        let r1 = self.ema1_range.feed(range);
        let r2 = self.ema2_range.feed(r1);
        let denom = (0.5 * r2).max(1e-12);
        self.value = 100.0 * d2 / denom;
        if self.ema2_diff.is_ready() && self.ema2_range.is_ready() {
            self.signal_ma.feed(self.value);
            self.signal = self.signal_ma.value();
        }
        self.ready =
            self.ema2_diff.is_ready() && self.ema2_range.is_ready() && self.signal_ma.is_ready();
        self.value
    }


    /// Named output: brace `line` (the SMI oscillator line).
    pub fn line(&self) -> f64 {
        self.value
    }

    /// Значение SMI линии
    pub fn value_smi(&self) -> f64 {
        self.value
    }

    /// Значение сигнальной линии
    pub fn value_signal(&self) -> f64 {
        self.signal
    }

    /// Brace-named getter: `signal` output (signal line).
    #[inline]
    pub fn signal(&self) -> f64 {
        self.signal
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn reset(&mut self) {
        self.ema1_diff.reset();
        self.ema2_diff.reset();
        self.ema1_range.reset();
        self.ema2_range.reset();
        self.signal_ma.reset();
        self.value = 0.0;
        self.signal = 0.0;
        self.ready = false;
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderOutput, RenderSpec};

/// Dual-mode contract config for [`Smi`] — Stochastic Momentum Index (bounded ~[-100, 100]).
///
/// Two independent smoother axes: `double` (the four inner legs, all at `double_period`) and
/// `signal` (the signal line, at `signal_period`). Both slots default to `follow(Ema)`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct SmiConfig {
    /// Period for the four double-smoother inner legs.
    pub double_period: Param<usize>,
    /// Double-smoother choice (kind + follow/own). Default: follow(Ema).
    #[slot]
    pub double: Param<SmootherChoice>,
    /// Period for the signal-line smoother.
    pub signal_period: Param<usize>,
    /// Signal smoother choice (kind + follow/own). Default: follow(Ema).
    #[slot]
    pub signal: Param<SmootherChoice>,
}

impl Indicator for Smi {
    const ID: IndicatorId = IndicatorId::Smi;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed HLC slice — SMI reads High, Low, Close.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// O(1): four double-smoother running states + a signal smoother — all cost lands
    /// recursively through the two `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = SmiConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::SmiLine),
        Output::centered(IndicatorOutputId::SmiSignal),
    ];
    type Config = SmiConfig;
    type Runtime = Smi;

    fn create(cfg: SmiConfig) -> Smi {
        let double_period = cfg.double_period.resolved();
        let signal_period = cfg.signal_period.resolved();
        let double_choice = cfg.double.resolved();
        let signal_choice = cfg.signal.resolved();
        Smi {
            ema1_diff: double_choice.build(double_period),
            ema2_diff: double_choice.build(double_period),
            ema1_range: double_choice.build(double_period),
            ema2_range: double_choice.build(double_period),
            signal_ma: signal_choice.build(signal_period),
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    fn slot_members(cfg: &SmiConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for SmiConfig {
    fn defaults() -> Self {
        SmiConfig {
            double_period: Param::Solo(14),
            double: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            signal_period: Param::Solo(3),
            signal: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // double_period/signal_period: Class A → auto range(2,4048,1).
        // double/signal: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for Smi {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::SmiLine, "SMI", Color::hex(0x2196F3), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::SmiSignal, "Signal", Color::hex(0xFF9800), 1.0))
            .bounds(-100.0, 100.0)
            .reference_line(ReferenceLine::new(40.0, Color::hex(0xFF5722)))
            .reference_line(ReferenceLine::new(-40.0, Color::hex(0x4CAF50)))
            .reference_line(ReferenceLine::new(0.0, Color::hex(0x9E9E9E)))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smi_creation() {
        let smi = Smi::new(14, 3);
        assert!(!smi.is_ready());
        assert_eq!(smi.value_smi(), 0.0);
        assert_eq!(smi.value_signal(), 0.0);
    }

    #[test]
    fn test_smi_value_types() {
        let smi = Smi::new(14, 3);
        assert_eq!(smi.line(), 0.0);
        assert_eq!(smi.signal(), 0.0);
    }

    #[test]
    fn test_smi_uptrend() {
        let mut smi = Smi::new(5, 3);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            smi.feed(&[price + 3.0, price - 1.0, price + 2.0]);
        }
        assert!(smi.is_ready());
        // In uptrend with close near high, SMI should be positive
        assert!(smi.value_smi() > 0.0, "SMI should be > 0 in uptrend, got {}", smi.value_smi());
    }

    #[test]
    fn test_smi_downtrend() {
        let mut smi = Smi::new(5, 3);
        for i in 1..=50 {
            let price = 200.0 - i as f64 * 2.0;
            smi.feed(&[price + 1.0, price - 3.0, price - 2.0]);
        }
        assert!(smi.is_ready());
        // In downtrend with close near low, SMI should be negative
        assert!(smi.value_smi() < 0.0, "SMI should be < 0 in downtrend, got {}", smi.value_smi());
    }

    #[test]
    fn test_smi_reset() {
        let mut smi = Smi::new(5, 3);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            smi.feed(&[price + 2.0, price - 1.0, price + 1.0]);
        }
        assert!(smi.is_ready());
        smi.reset();
        assert!(!smi.is_ready());
        assert_eq!(smi.value_smi(), 0.0);
        assert_eq!(smi.value_signal(), 0.0);
    }

    #[test]
    fn test_smi_finite_values() {
        let mut smi = Smi::new(5, 3);
        for i in 1..=100 {
            let base = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            let h = base + 3.0;
            let l = base - 3.0;
            let c = base + (i as f64 * 0.5).cos() * 2.0;
            let value = smi.feed(&[h, l, c]);
            assert!(value.is_finite(), "SMI should always be finite");
            assert!(smi.value_signal().is_finite(), "SMI signal should always be finite");
        }
    }

    #[test]
    fn test_smi_configurable_double_smoother() {
        // SMA double-smoothing + SMA signal — the double-smoother is now a free slot choice.
        let mut smi = Smi::from_smoothers(SmootherId::Sma, 5, SmootherId::Sma, 3);
        for i in 1..=50 {
            let p = 100.0 + i as f64 * 0.5;
            let v = smi.feed(&[p + 2.0, p - 1.0, p + 0.5]);
            assert!(v.is_finite());
        }
        assert!(smi.is_ready());
    }

    #[test]
    fn test_smi_contract_create() {
        let cfg = <<Smi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.double_period.resolved(), 14);
        assert_eq!(cfg.signal_period.resolved(), 3);
        let mut smi = <Smi as Indicator>::create(cfg);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            smi.feed(&[price + 3.0, price - 1.0, price + 2.0]);
        }
        assert!(smi.is_ready());
    }

    /// The factory resolves the fixed HLC lanes and feeds the three scalars; SMI runs
    /// end-to-end (uptrend close near high -> positive).
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Smi(<<Smi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=80 {
            let price = 100.0 + i as f64 * 2.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: price + 3.0, low: price - 1.0, close: price + 2.0, volume: 1000.0,
            });
        }
        assert!(f.primary() > 0.0, "factory SMI uptrend should be > 0, got {}", f.primary());
    }
}
