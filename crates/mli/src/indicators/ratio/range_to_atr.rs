// Range-to-ATR ratio: (High-Low)/ATR

use crate::engine::contract_engine::SmootherId;
use crate::indicators::volatility::atr::Atr;

#[derive(Debug, Clone)]
pub struct RangeToAtr {
    atr: Atr,
    value: f64,
}

impl RangeToAtr {
    pub fn new(atr_period: usize, smoother: SmootherId) -> Self {
        Self {
            atr: Atr::from_smoother(atr_period, smoother),
            value: 0.0,
        }
    }

    /// Feed resolved input lanes `[high, low, close]` — the pure core computation.
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
    pub fn reset(&mut self) {
        self.atr.reset();
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.atr.is_ready()
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
}

impl Default for RangeToAtr {
    fn default() -> Self {
        Self::new(14, SmootherId::Rma)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::contract_engine::SmootherChoice;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec, ReferenceLine};
use crate::engine::stream_kind::StreamKind;
use crate::contract::Slot;

/// Typed dual-mode config for [`RangeToAtr`].
/// `atr_period` = ATR period; `atr_smoother` = inner ATR smoother (default `follow(Rma)`).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct RangeToAtrConfig {
    pub atr_period: Param<usize>,
    #[slot]
    pub atr_smoother: Param<SmootherChoice>,
}

impl Indicator for RangeToAtr {
    const ID: IndicatorId = IndicatorId::RangeAtr;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const SLOTS: &'static [Slot] = RangeToAtrConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::ratio(IndicatorOutputId::RangeAtr)];
    type Config = RangeToAtrConfig;
    type Runtime = RangeToAtr;

    fn create(cfg: RangeToAtrConfig) -> RangeToAtr {
        let period = cfg.atr_period.resolved();
        let choice = cfg.atr_smoother.resolved();
        let atr_period = choice.period.resolve(period);
        RangeToAtr::new(atr_period, choice.kind)
    }

    /// Forward the `#[slot]` field's resolved member (the `#[derive(Slots)]` helper) — kept in
    /// step with `SLOTS` (one slot, the ATR smoother). The Config-cutover (29d1d29) dropped this
    /// forward, leaving `slot_members` at the empty trait default (1 SLOT vs 0 members).
    fn slot_members(cfg: &RangeToAtrConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for RangeToAtrConfig {
    fn defaults() -> Self {
        RangeToAtrConfig {
            atr_period: Param::Solo(14),
            atr_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn machine_defaults() -> Self {
        // atr_period: Class A (ATR lookback) → auto range(2,4048,1).
        // #[slot] atr_smoother (SmootherChoice): left Solo — smoother sweep is a later wave.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for RangeToAtr {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::RangeAtr, "Range/ATR", Color::hex(0xFF9800))
            .reference_line(ReferenceLine::new(1.0, Color::hex(0x9E9E9E)))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::Param;

    #[test]
    fn test_range_to_atr_creation() {
        let ind = RangeToAtr::new(14, SmootherId::Sma);
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_range_to_atr_warmup() {
        let mut ind = RangeToAtr::new(10, SmootherId::Ema);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.feed(&[price + 2.0, price - 2.0, price + 1.0]);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_range_to_atr_values() {
        let mut ind = RangeToAtr::new(10, SmootherId::Sma);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 5.0;
            ind.feed(&[price + 2.0, price - 2.0, price + 1.0]);
        }
        assert!(ind.value().is_finite());
        assert!(ind.value() >= 0.0);
    }

    #[test]
    fn test_range_to_atr_reset() {
        let mut ind = RangeToAtr::new(10, SmootherId::Sma);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            ind.feed(&[price + 2.0, price - 2.0, price + 1.0]);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_factory_feeds_resolved_range_to_atr() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::RangeAtr(<<RangeToAtr as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..20 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 2.0,
                low: price - 2.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.primary().is_finite());
        assert!(f.primary() >= 0.0);
    }

    #[test]
    fn test_range_to_atr_own_period() {
        let cfg = RangeToAtrConfig {
            atr_period: Param::Solo(14),
            atr_smoother: Param::Solo(SmootherChoice::own(SmootherId::Ema, 10)),
        };
        let mut ind = <RangeToAtr as Indicator>::create(cfg);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            ind.feed(&[price + 2.0, price - 2.0, price + 1.0]);
        }
        assert!(ind.is_ready());
    }
}
