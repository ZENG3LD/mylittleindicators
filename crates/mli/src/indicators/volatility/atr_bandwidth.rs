// ATR Bandwidth: (High-Low)/ATR over rolling period

use crate::indicators::volatility::atr::Atr;

#[derive(Debug, Clone)]
pub struct AtrBandwidth {
    atr: Atr,
    value: f64,
}

impl AtrBandwidth {
    /// Default ctor — inner ATR smoothed with RMA (Wilder).
    pub fn new(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Rma)
    }

    /// Build from a narrow `SmootherId` for the inner ATR smoother + period.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        Self {
            atr: Atr::from_smoother(period, smoother),
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.atr.reset();
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.atr.is_ready()
    }

    /// Feed the resolved `[high, low, close]` lanes (in `SOURCE` order).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let atr_v = self.atr.feed(&[high, low, close]);
        let hl = (high - low).max(0.0);
        self.value = if atr_v > 1e-12 { hl / atr_v } else { 0.0 };
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::{SmootherId, SmootherChoice};
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed dual-mode config for [`AtrBandwidth`] — `(High-Low)/ATR`. Self-contained:
/// owns its `atr_period`; the inner ATR's `atr_smoother` (`SmootherChoice`) owns its
/// own period via `Follow` (default) or `Own`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct AtrBandwidthConfig {
    pub atr_period: Param<usize>,
    #[slot]
    pub atr_smoother: Param<SmootherChoice>,
}

impl Indicator for AtrBandwidth {
    const ID: IndicatorId = IndicatorId::Atrbw;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to the raw range slices -- ATR is h/l/prev-close, ratio over the bar range.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// O(1) over the inner ATR value; the smoothing buffer cost lands recursively via the slot.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = AtrBandwidthConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::ratio(IndicatorOutputId::Atrbw)];
    type Config = AtrBandwidthConfig;
    type Runtime = AtrBandwidth;

    fn create(cfg: AtrBandwidthConfig) -> AtrBandwidth {
        let period = cfg.atr_period.resolved();
        let choice = cfg.atr_smoother.resolved();
        let atr_period = choice.period.resolve(period);
        AtrBandwidth {
            atr: Atr::from_smoother(atr_period, choice.kind),
            value: 0.0,
        }
    }

    fn slot_members(cfg: &AtrBandwidthConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for AtrBandwidthConfig {
    fn defaults() -> Self {
        AtrBandwidthConfig {
            atr_period: Param::Solo(14),
            atr_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn machine_defaults() -> Self {
        // atr_period: Class A usize — auto range(2,4048,1)
        // #[slot] atr_smoother: leave Solo (deferred sweep wave)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for AtrBandwidth {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Atrbw, "ATR Bandwidth", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::Param;

    #[test]
    fn test_atr_bandwidth_creation() {
        let ab = AtrBandwidth::new(14);
        assert!(!ab.is_ready());
        assert_eq!(ab.value(), 0.0);
    }

    #[test]
    fn test_atr_bandwidth_warmup() {
        let mut ab = AtrBandwidth::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ab.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ab.is_ready());
    }

    #[test]
    fn test_atr_bandwidth_values() {
        let mut ab = AtrBandwidth::new(14);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            let value = ab.feed(&[price + 2.0, price - 2.0, price]);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_atr_bandwidth_reset() {
        let mut ab = AtrBandwidth::new(14);
        for i in 0..20 {
            ab.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        ab.reset();
        assert!(!ab.is_ready());
        assert_eq!(ab.value(), 0.0);
    }

    #[test]
    fn test_atr_bandwidth_contract_create() {
        let cfg = <<AtrBandwidth as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.atr_period.resolved(), 14);
        let mut ab = <AtrBandwidth as Indicator>::create(cfg);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            ab.feed(&[price + 2.0, price - 2.0, price]);
        }
        assert!(ab.is_ready());
    }

    #[test]
    fn test_atr_bandwidth_own_period() {
        let cfg = AtrBandwidthConfig {
            atr_period: Param::Solo(14),
            atr_smoother: Param::Solo(SmootherChoice::own(SmootherId::Sma, 10)),
        };
        let mut ab = <AtrBandwidth as Indicator>::create(cfg);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            ab.feed(&[price + 2.0, price - 2.0, price]);
        }
        assert!(ab.is_ready());
    }
}
