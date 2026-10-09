// Inverse Fisher Transform of RSI

use crate::indicators::momentum::rsi::Rsi;

/// Inverse Fisher Transform of RSI (IFT-RSI).
///
/// Applies the inverse Fisher transform (tanh) to a normalized RSI reading to
/// produce a smoother oscillator bounded in (-1, 1).
///
/// `IFT_RSI = tanh((RSI/50 - 1))`
///
/// Output is negative in downtrends and positive in uptrends, crossing zero
/// more clearly than the underlying RSI crossing 50.
#[derive(Debug, Clone)]
pub struct IftRsi {
    rsi: Rsi,
    value: f64,
}

impl IftRsi {
    pub fn new(period: usize) -> Self {
        Self::with_rsi_period(period)
    }

    /// Create IFT RSI with explicit RSI period parameter.
    ///
    /// # Arguments
    /// * `rsi_period` - RSI lookback period (minimum 1)
    pub fn with_rsi_period(rsi_period: usize) -> Self {
        Self {
            rsi: Rsi::new(rsi_period.max(1)),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.rsi.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed a pre-resolved scalar — the contracted entry point.
    ///
    /// The inner RSI is driven via the close slot (passing scalar as close, zeroing
    /// all other OHLCV fields) — math is byte-identical to the legacy `update_bar`
    /// which passed price as close through the RSI's default close source.
    pub fn feed(&mut self, v: f64) -> f64 {
        let r = self.rsi.feed(v);
        // RSI is on 0-100 scale; normalize to [-1, 1]: (r/50 - 1) = (r - 50)/50
        let normalized_rsi = (r - 0.5) * 2.0; // [0, 1] -> [-1, 1] (legacy scale kept)
        // Apply tanh for smoother Fisher Transform
        self.value = normalized_rsi.tanh();
        self.value
    }
}

impl Default for IftRsi {
    fn default() -> Self {
        Self::with_rsi_period(14)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice, SmootherId};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`IftRsi`] — RSI period + RSI smoother slot + configurable price source.
///
/// Dual-mode: every field is a `Param`. The RSI smoother slot follows the period field by
/// default (`follow(Rma)` = Wilder's original smoothing). The inner `Rsi` is built via
/// `Rsi::new(period)` (default RMA smoother) unless a `SmootherChoice` override is given.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct IftRsiConfig {
    /// RSI lookback period (minimum 1).
    pub period: Param<usize>,
    /// Configurable price source (default close).
    pub source: Param<OhlcvField>,
    /// RSI gain/loss smoother choice — default `follow(Rma)` (Wilder's RMA at `period`).
    #[slot]
    pub rsi_smoother: Param<SmootherChoice>,
}

impl Indicator for IftRsi {
    const ID: IndicatorId = IndicatorId::IftRsi;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close); scalar fed through close slot of inner RSI.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1) own state (one tanh); inner RSI declared as a Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi])],
    };
    const SLOTS: &'static [crate::contract::Slot] = IftRsiConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::IftRsi)];

    type Config = IftRsiConfig;
    type Runtime = IftRsi;

    fn create(cfg: IftRsiConfig) -> IftRsi {
        let period = cfg.period.resolved().max(1);
        let choice = cfg.rsi_smoother.resolved();
        IftRsi {
            rsi: Rsi::from_choice(choice, period),
            value: 0.0,
        }
    }

    fn source_fields(cfg: &IftRsiConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &IftRsiConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for IftRsiConfig {
    fn defaults() -> Self {
        IftRsiConfig {
            period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
            rsi_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
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
        // rsi_smoother: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for IftRsi {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::IftRsi, "IFT RSI", Color::hex(0x9C27B0))
            .bounds(-1.0, 1.0)
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ift_rsi_creation() {
        let ift = IftRsi::new(14);
        assert!(!ift.is_ready());
        assert_eq!(ift.value(), 0.0);
    }

    #[test]
    fn test_ift_rsi_with_rsi_period() {
        let mut ift = IftRsi::with_rsi_period(9);
        for i in 1..=30 {
            let p = 100.0 + i as f64 * 0.5;
            let v = ift.feed(p);
            assert!(v.is_finite() && v >= -1.0 && v <= 1.0);
        }
        assert!(ift.is_ready());
    }

    #[test]
    fn test_ift_rsi_uptrend() {
        let mut ift = IftRsi::new(14);
        for i in 1..=40 {
            let price = 100.0 + i as f64 * 2.0;
            ift.feed(price);
        }
        assert!(ift.is_ready());
        // In uptrend, RSI > 50, so IFT RSI > 0
        assert!(ift.value() > 0.0, "IFT RSI should be positive in uptrend, got {}", ift.value());
    }

    #[test]
    fn test_ift_rsi_downtrend() {
        let mut ift = IftRsi::new(14);
        for i in 1..=40 {
            let price = 200.0 - i as f64 * 2.0;
            ift.feed(price);
        }
        assert!(ift.is_ready());
        // In downtrend, RSI < 50, so IFT RSI < 0
        assert!(ift.value() < 0.0, "IFT RSI should be negative in downtrend, got {}", ift.value());
    }

    #[test]
    fn test_ift_rsi_range() {
        let mut ift = IftRsi::new(14);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = ift.feed(price);
            // tanh output is in (-1, 1)
            assert!(value >= -1.0 && value <= 1.0, "IFT RSI should be in [-1, 1], got {}", value);
        }
    }

    #[test]
    fn test_ift_rsi_reset() {
        let mut ift = IftRsi::new(14);
        for i in 1..=40 {
            let price = 100.0 + i as f64;
            ift.feed(price);
        }
        assert!(ift.is_ready());
        ift.reset();
        assert!(!ift.is_ready());
        assert_eq!(ift.value(), 0.0);
    }

    /// Factory resolves the close field (not the wild 9999 high) and feeds the scalar;
    /// ascending close prices produce positive IFT RSI (RSI > 50 → tanh > 0).
    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<IftRsi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::IftRsi(cfg).build_solo().unwrap();
        for i in 1..=40 {
            let price = 100.0 + i as f64 * 2.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(
            f.primary() > 0.0,
            "factory IFT RSI on ascending close should be positive, got {}",
            f.primary()
        );
    }

    #[test]
    fn config_dual_mode() {
        use crate::contract::{Config};
        let d = <<IftRsi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(d.cube_size(), 1);
        assert_eq!(d.iter().count(), 1);

        let swept = IftRsiConfig {
            period: Param::range(7, 21, 7),
            source: Param::Solo(OhlcvField::Close),
            rsi_smoother: Param::many(vec![
                SmootherChoice::follow(SmootherId::Rma),
                SmootherChoice::follow(SmootherId::Ema),
            ]),
        };
        assert_eq!(swept.cube_size(), 6); // 3 periods × 2 smoothers
        assert_eq!(swept.iter().count(), 6);
    }
}
