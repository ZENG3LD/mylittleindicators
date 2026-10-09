use crate::engine::contract_engine::SmootherSlot;

/// Qstick = MA_n(Close - Open)
#[derive(Debug, Clone)]
pub struct Qstick {
    ma: SmootherSlot,
    period: usize,
    value: f64,
}

impl Qstick {
    /// Default ctor — `Close - Open` smoothed with SMA (the classic Qstick kernel).
    pub fn new(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Sma)
    }

    /// Build from a narrow `SmootherId` for the smoother + period.
    /// Legacy bridge; the contract path goes through `QstickConfig.ma`.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        let p = period.max(1);
        Self {
            ma: SmootherSlot::new(smoother, p),
            period: p,
            value: 0.0,
        }
    }

    /// Feed the resolved input lanes — `[open, close]`. Qstick smooths `close - open`;
    /// the factory resolves the fixed Open/Close slice and feeds the pair.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let diff = lanes[1] - lanes[0];
        self.value = self.ma.feed(diff);
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.ma.is_ready()
    }
    pub fn reset(&mut self) {
        self.ma.reset();
        self.value = 0.0;
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
use crate::contract::{Color, RenderSpec};

impl Qstick {
    /// Build a Qstick from a smoother CHOICE (kind + follow/own period) at the host `period`.
    pub fn from_choice(choice: SmootherChoice, period: usize) -> Self {
        let p = period.max(1);
        Self {
            ma: choice.build(p),
            period: p,
            value: 0.0,
        }
    }
}

/// Typed contract config for [`Qstick`] — smoothed `Close - Open`. Dual-mode: every
/// field is a `Param`. The `#[slot]` smoother is a `Param<SmootherChoice>`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct QstickConfig {
    pub period: Param<usize>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl Indicator for Qstick {
    const ID: IndicatorId = IndicatorId::Qstick;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to open+close — smooths `close - open`.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::Open, OhlcvField::Close]));
    /// O(1): one subtraction fed to the smoother; the smoother buffer cost lands via the slot.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = QstickConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Qstick)];
    type Config = QstickConfig;
    type Runtime = Qstick;

    fn create(cfg: QstickConfig) -> Qstick {
        Qstick::from_choice(cfg.ma.resolved(), cfg.period.resolved())
    }

    // Input fields: the factory derives the fixed [Open, Close] lanes straight from
    // `const SOURCE = KlineSlice([..])` (the default `source_fields`) — no re-declaration.

    fn slot_members(cfg: &QstickConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for QstickConfig {
    fn defaults() -> Self {
        QstickConfig {
            period: Param::Solo(14),
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
        // period: Class A → auto range(2,4048,1).
        // ma: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for Qstick {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Qstick, "QStick", Color::hex(0x009688))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qstick_creation() {
        let qstick = Qstick::new(10);
        assert!(!qstick.is_ready());
        assert_eq!(qstick.value(), 0.0);
        assert_eq!(qstick.period(), 10);
    }

    #[test]
    fn test_qstick_min_period() {
        let qstick = Qstick::new(0);
        assert_eq!(qstick.period(), 1); // min period is 1
    }

    #[test]
    fn test_qstick_bullish_bars() {
        let mut qstick = Qstick::new(5);
        // Bullish bars: close > open -> [open, close]
        for _ in 0..20 {
            qstick.feed(&[100.0, 108.0]);
        }
        assert!(qstick.is_ready());
        // SMA of (close - open) = SMA(8) = 8
        assert!((qstick.value() - 8.0).abs() < 1e-10, "Qstick for bullish bars should be 8, got {}", qstick.value());
    }

    #[test]
    fn test_qstick_bearish_bars() {
        let mut qstick = Qstick::new(5);
        // Bearish bars: close < open
        for _ in 0..20 {
            qstick.feed(&[108.0, 100.0]);
        }
        assert!(qstick.is_ready());
        // SMA of (close - open) = SMA(-8) = -8
        assert!((qstick.value() - (-8.0)).abs() < 1e-10, "Qstick for bearish bars should be -8, got {}", qstick.value());
    }

    #[test]
    fn test_qstick_doji() {
        let mut qstick = Qstick::new(5);
        // Doji bars: close == open
        for _ in 0..20 {
            qstick.feed(&[100.0, 100.0]);
        }
        assert!(qstick.is_ready());
        assert!((qstick.value()).abs() < 1e-10, "Qstick for doji should be 0");
    }

    #[test]
    fn test_qstick_reset() {
        let mut qstick = Qstick::new(5);
        for _ in 0..20 {
            qstick.feed(&[100.0, 108.0]);
        }
        assert!(qstick.is_ready());
        qstick.reset();
        assert!(!qstick.is_ready());
        assert_eq!(qstick.value(), 0.0);
    }

    #[test]
    fn test_qstick_is_ready_timing() {
        let mut qstick = Qstick::new(5);
        for i in 1..=10 {
            qstick.feed(&[100.0, 105.0]);
            if i < 5 {
                assert!(!qstick.is_ready(), "Qstick should not be ready at bar {}", i);
            } else {
                assert!(qstick.is_ready(), "Qstick should be ready at bar {}", i);
            }
        }
    }

    #[test]
    fn test_qstick_mixed_bars() {
        let mut qstick = Qstick::new(4);
        // 2 bullish (+5), 2 bearish (-5) = average 0; lanes = [open, close]
        qstick.feed(&[100.0, 105.0]); // +5
        qstick.feed(&[100.0, 105.0]); // +5
        qstick.feed(&[105.0, 100.0]); // -5
        qstick.feed(&[105.0, 100.0]); // -5
        assert!(qstick.is_ready());
        assert!((qstick.value()).abs() < 1e-10, "Qstick for mixed equal bars should be 0");
    }

    #[test]
    fn test_qstick_finite_values() {
        let mut qstick = Qstick::new(10);
        for i in 1..=100 {
            let open = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let close = open + (i as f64 * 0.5).cos() * 5.0;
            let value = qstick.feed(&[open, close]);
            assert!(value.is_finite(), "Qstick should always be finite");
        }
    }

    #[test]
    fn test_qstick_contract_create() {
        let cfg = <<Qstick as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        // default smoother is Follow(Sma) at host period 14
        let mut qstick = <Qstick as Indicator>::create(cfg);
        for _ in 0..20 {
            qstick.feed(&[100.0, 108.0]);
        }
        assert!(qstick.is_ready());
    }

    /// The factory resolves the fixed Open/Close lanes and feeds the pair; Qstick smooths
    /// `close - open` end-to-end.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Qstick(<<Qstick as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for _ in 0..20 {
            f.feed(0, MarketSample::Bar {
                open: 100.0, high: 110.0, low: 95.0, close: 108.0, volume: 1000.0,
            });
        }
        // SMA of (108 - 100) = 8
        assert!((f.primary() - 8.0).abs() < 1e-10, "factory Qstick must smooth close-open=8, got {}", f.primary());
    }
}
