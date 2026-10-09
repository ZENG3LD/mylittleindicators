// RSIOMA - RSI of Moving Average or MA of RSI combined

use crate::engine::contract_engine::SmootherSlot;
use crate::indicators::momentum::rsi::Rsi;

#[derive(Debug, Clone)]
pub struct RsiOma {
    rsi: Rsi,
    ema: SmootherSlot,
    value: f64,
}

impl RsiOma {
    /// Default ctor — RSI smoothed by an outer EMA (the classic RSIOMA kernel).
    pub fn new(rsi_period: usize, ema_period: usize) -> Self {
        Self::from_smoother(rsi_period, ema_period, SmootherId::Ema)
    }

    /// Build from a narrow `SmootherId` for the OUTER smoother + the RSI/MA periods.
    /// The inner RSI keeps its own default smoother. Legacy bridge; the contract path
    /// goes through `RsiOmaConfig`.
    pub fn from_smoother(rsi_period: usize, ma_period: usize, smoother: SmootherId) -> Self {
        Self {
            rsi: Rsi::new(rsi_period.max(1)),
            ema: SmootherSlot::new(smoother, ma_period.max(1)),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.rsi.reset();
        self.ema.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ema.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed ONE resolved scalar (the configured source, default close). The factory
    /// extracts the source field; the inner RSI consumes the scalar directly.
    pub fn feed(&mut self, value: f64) -> f64 {
        let _ = self.rsi.feed(value);
        self.value = self.ema.feed(self.rsi.value());
        self.value
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice, SmootherId};
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderSpec};

/// Typed contract config for [`RsiOma`] — an OUTER smoother over RSI.
///
/// `rsi_period` = the inner RSI lookback (RSI keeps its default Wilder RMA, wired as a fixed
/// inner `Port`). `ma_period` = outer smoother period. The outer smoother slot follows
/// `ma_period` by default (`follow(Ema)`).
///
/// Dual-mode: every field is a `Param`. The `#[slot]` smoother is `Param<SmootherChoice>`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct RsiOmaConfig {
    pub rsi_period: Param<usize>,
    pub ma_period: Param<usize>,
    /// Outer smoother choice — default `follow(Ema)` (EMA at `ma_period`).
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl Indicator for RsiOma {
    const ID: IndicatorId = IndicatorId::Rsioma;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Operates on close through the inner RSI.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1) over the inner RSI value; the inner RSI is a fixed `Port`, the outer
    /// smoother is the configurable `SLOTS` member.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi])],
    };
    const SLOTS: &'static [Slot] = RsiOmaConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Rsioma)];
    type Config = RsiOmaConfig;
    type Runtime = RsiOma;

    fn create(cfg: RsiOmaConfig) -> RsiOma {
        let rsi_period = cfg.rsi_period.resolved().max(1);
        let ma_period = cfg.ma_period.resolved().max(1);
        let choice = cfg.ma.resolved();
        RsiOma {
            rsi: Rsi::new(rsi_period),
            ema: choice.build(ma_period),
            value: 0.0,
        }
    }

    fn slot_members(cfg: &RsiOmaConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for RsiOmaConfig {
    fn defaults() -> Self {
        RsiOmaConfig {
            rsi_period: Param::Solo(14),
            ma_period: Param::Solo(9),
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
        // rsi_period/ma_period: Class A → auto range(2,4048,1).
        // ma: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for RsiOma {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Rsioma, "RSIOMA", Color::hex(0x9C27B0))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(70.0, Color::hex(0x9E9E9E)))
            .reference_line(ReferenceLine::new(30.0, Color::hex(0x9E9E9E)))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsioma_creation() {
        let rsioma = RsiOma::new(14, 9);
        assert!(!rsioma.is_ready());
        assert_eq!(rsioma.value(), 0.0);
    }

    #[test]
    fn test_rsioma_uptrend() {
        let mut rsioma = RsiOma::new(14, 9);
        for i in 1..=40 {
            let price = 100.0 + i as f64 * 2.0;
            rsioma.feed(price);
        }
        assert!(rsioma.is_ready());
        // In uptrend, RSI > 0.5 (50%), so smoothed RSI should also be > 0.5
        assert!(rsioma.value() > 0.5, "RSIOMA should be > 0.5 in uptrend, got {}", rsioma.value());
    }

    #[test]
    fn test_rsioma_downtrend() {
        let mut rsioma = RsiOma::new(14, 9);
        for i in 1..=40 {
            let price = 200.0 - i as f64 * 2.0;
            rsioma.feed(price);
        }
        assert!(rsioma.is_ready());
        // In downtrend, RSI < 0.5 (50%), so smoothed RSI should also be < 0.5
        assert!(rsioma.value() < 0.5, "RSIOMA should be < 0.5 in downtrend, got {}", rsioma.value());
    }

    #[test]
    fn test_rsioma_reset() {
        let mut rsioma = RsiOma::new(14, 9);
        for i in 1..=40 {
            let price = 100.0 + i as f64;
            rsioma.feed(price);
        }
        assert!(rsioma.is_ready());
        rsioma.reset();
        assert!(!rsioma.is_ready());
        assert_eq!(rsioma.value(), 0.0);
    }

    #[test]
    fn test_rsioma_finite_values() {
        let mut rsioma = RsiOma::new(14, 9);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = rsioma.feed(price);
            assert!(value.is_finite(), "RSIOMA should always be finite");
        }
    }

    #[test]
    fn test_rsioma_with_smoother() {
        let mut rsioma = RsiOma::from_smoother(14, 9, SmootherId::Sma);
        for i in 1..=40 {
            let p = 100.0 + i as f64 * 0.5;
            let v = rsioma.feed(p);
            assert!(v.is_finite());
        }
        assert!(rsioma.is_ready());
    }

    #[test]
    fn test_rsioma_contract_create() {
        let cfg = <<RsiOma as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.ma_period.resolved(), 9);
        let mut rsioma = <RsiOma as Indicator>::create(cfg);
        for i in 1..=40 {
            let price = 100.0 + i as f64 * 2.0;
            rsioma.feed(price);
        }
        assert!(rsioma.is_ready());
    }

    #[test]
    fn config_dual_mode() {
        use crate::contract::{Config};
        let d = <<RsiOma as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(d.cube_size(), 1);
        assert_eq!(d.iter().count(), 1);

        let swept = RsiOmaConfig {
            rsi_period: Param::range(7, 21, 7),
            ma_period: Param::range(5, 15, 5),
            ma: Param::many(vec![
                SmootherChoice::follow(SmootherId::Ema),
                SmootherChoice::follow(SmootherId::Sma),
            ]),
        };
        // 3 rsi_period × 3 ma_period × 2 smoothers = 18
        assert_eq!(swept.cube_size(), 18);
    }
}
