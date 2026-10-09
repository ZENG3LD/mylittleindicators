use crate::indicators::volatility::atr::Atr;

/// Normalized ATR (NATR) = 100 * ATR(period) / Close
#[derive(Debug, Clone)]
pub struct Natr {
    atr: Atr,
    value: f64,
}

impl Natr {
    /// Default ctor — ATR smoothed with RMA (Wilder).
    pub fn new(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Rma)
    }

    /// Build NATR from a narrow `SmootherId` for the inner ATR smoother + period.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        Self {
            atr: Atr::from_smoother(period.max(1), smoother),
            value: 0.0,
        }
    }

    /// Feed the resolved `[high, low, close]` lanes (in `SOURCE` order); normalizes the
    /// inner ATR by close.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let _ = self.atr.feed(&[high, low, close]);
        let atrv = self.atr.value();
        self.value = if close.abs() < 1e-12 {
            0.0
        } else {
            100.0 * atrv / close.abs()
        };
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.atr.is_ready()
    }
    pub fn reset(&mut self) {
        self.atr.reset();
        self.value = 0.0;
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

/// Typed dual-mode config for [`Natr`]. `atr_period` is the ATR period;
/// `atr_smoother` selects the inner ATR smoother (default `follow(Rma)`).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct NatrConfig {
    pub atr_period: Param<usize>,
    #[slot]
    pub atr_smoother: Param<SmootherChoice>,
}

impl Indicator for Natr {
    const ID: IndicatorId = IndicatorId::Natr;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to the raw range slices -- NATR normalises ATR (h/l/prev-close) by close.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// NATR's own base is O(1): one divide over the inner ATR value. The smoothing
    /// buffer is NOT here -- it belongs to the inner ATR's SLOT member, whose cost
    /// lands recursively through `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = NatrConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Natr)];
    type Config = NatrConfig;
    type Runtime = Natr;

    fn create(cfg: NatrConfig) -> Natr {
        let period = cfg.atr_period.resolved();
        let choice = cfg.atr_smoother.resolved();
        let atr_period = choice.period.resolve(period);
        Natr {
            atr: Atr::from_smoother(atr_period, choice.kind),
            value: 0.0,
        }
    }

    fn slot_members(cfg: &NatrConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for NatrConfig {
    fn defaults() -> Self {
        NatrConfig {
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


impl Render for Natr {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Natr, "NATR", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_natr_creation() {
        let natr = Natr::new(14);
        assert!(!natr.is_ready());
        assert_eq!(natr.value(), 0.0);
    }

    #[test]
    fn test_natr_warmup() {
        let mut natr = Natr::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            natr.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(natr.is_ready());
    }

    #[test]
    fn test_natr_positive() {
        let mut natr = Natr::new(14);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            let value = natr.feed(&[price + 2.0, price - 2.0, price]);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_natr_with_ema_smoother() {
        let mut natr = Natr::from_smoother(14, SmootherId::Ema);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            let v = natr.feed(&[price + 2.0, price - 2.0, price]);
            assert!(v.is_finite());
        }
        assert!(natr.is_ready());
    }

    #[test]
    fn test_natr_reset() {
        let mut natr = Natr::new(14);
        for i in 0..20 {
            natr.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        natr.reset();
        assert!(!natr.is_ready());
        assert_eq!(natr.value(), 0.0);
    }

    #[test]
    fn test_natr_contract_create() {
        let cfg = <<Natr as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.atr_period.resolved(), 14);
        let mut natr = <Natr as Indicator>::create(cfg);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            natr.feed(&[price + 2.0, price - 2.0, price]);
        }
        assert!(natr.is_ready());
    }
}
