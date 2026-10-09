// Keltner Bandwidth: (upper - lower) / middle

use crate::indicators::channels::keltner_channel::{KeltnerChannel, KeltnerMode};

#[derive(Debug, Clone)]
pub struct KeltnerBandwidth {
    kc: KeltnerChannel,
    value: f64,
}

impl Default for KeltnerBandwidth {
    fn default() -> Self {
        Self::with_smoothers(14, 2.0, SmootherId::Sma, SmootherId::Rma)
    }
}

impl KeltnerBandwidth {
    pub fn new(period: usize, mult: f64) -> Self {
        Self::with_smoothers(period, mult, SmootherId::Sma, SmootherId::Rma)
    }

    /// Build with explicit smoother ids for center MA and ATR smoothing.
    pub fn with_smoothers(
        period: usize,
        mult: f64,
        center: SmootherId,
        atr_smoother: SmootherId,
    ) -> Self {
        Self {
            kc: KeltnerChannel::from_smoothers(
                center,
                atr_smoother,
                period.max(2),
                mult.max(0.1),
                KeltnerMode::Classic,
                crate::engine::ohlcv_field::OhlcvField::Close,
            ),
            value: 0.0,
        }
    }

    /// Feed HIGH, LOW, CLOSE lanes (forwarded to inner KeltnerChannel).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let (upper, middle, lower) = self.kc.feed(lanes);
        let width = upper - lower;
        self.value = if middle.abs() > 1e-12 { width / middle.abs() } else { 0.0 };
        self.value
    }

    /// Legacy bar-level update.

    #[inline] pub fn reset(&mut self) { self.kc.reset(); self.value = 0.0; }
    #[inline] pub fn is_ready(&self) -> bool { self.kc.is_ready() }
    #[inline] pub fn value(&self) -> f64 { self.value }
}

// ─── Contract ────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::{IndicatorOutputId, SmootherId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::{Param, sweep_f64};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Port, Slot, SourceAxis, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::{Color, Render, RenderSpec};

/// Typed dual-mode config for [`KeltnerBandwidth`].
///
/// ABSORB: builds `KeltnerChannel` via `KeltnerChannel::from_smoothers`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct KeltnerBandwidthConfig {
    pub period: Param<usize>,
    pub multiplier: Param<f64>,
    /// Center-line smoother slot. Default Sma at the host period (14).
    #[slot]
    pub ma: Param<SmootherSlotOrder>,
    /// ATR smoother slot. Default Rma (Wilder) at the host period (14).
    #[slot]
    pub atr_ma: Param<SmootherSlotOrder>,
}

impl Indicator for KeltnerBandwidth {
    const ID: IndicatorId = IndicatorId::Keltbw;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High, OhlcvField::Low, OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Kc, &[
            IndicatorOutputId::KcUpper,
            IndicatorOutputId::KcMiddle,
            IndicatorOutputId::KcLower,
        ])],
    };
    const SLOTS: &'static [Slot] = KeltnerBandwidthConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Keltbw)];
    type Config = KeltnerBandwidthConfig;
    type Runtime = Self;

    fn create(cfg: KeltnerBandwidthConfig) -> Self {
        let p = cfg.period.resolved();
        let ma_order = cfg.ma.resolved();
        let atr_order = cfg.atr_ma.resolved();
        KeltnerBandwidth::with_smoothers(
            p,
            cfg.multiplier.resolved(),
            ma_order.id(),
            atr_order.id(),
        )
    }

    fn slot_members(cfg: &KeltnerBandwidthConfig) -> Vec<IndicatorId> { cfg.slot_members() }
}

impl crate::contract::Config for KeltnerBandwidthConfig {
    /// Period 14, multiplier 2.0, SMA center at period 14, RMA ATR at period 14.
    fn defaults() -> Self {
        KeltnerBandwidthConfig {
            period: Param::Solo(14),
            multiplier: Param::Solo(2.0),
            ma: Param::Solo(SmootherSlotOrder::Sma(PeriodConfig { period: 14 })),
            atr_ma: Param::Solo(SmootherSlotOrder::Rma(PeriodConfig { period: 14 })),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1)
        s.multiplier = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for KeltnerBandwidth {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Keltbw, "Keltner BW", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keltner_bandwidth_creation() {
        let kb = KeltnerBandwidth::new(20, 2.0);
        assert!(!kb.is_ready());
        assert_eq!(kb.value(), 0.0);
    }

    #[test]
    fn test_keltner_bandwidth_warmup() {
        let mut kb = KeltnerBandwidth::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kb.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(kb.is_ready());
    }

    #[test]
    fn test_keltner_bandwidth_positive() {
        let mut kb = KeltnerBandwidth::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kb.feed(&[price + 1.0, price - 1.0, price]);
            assert!(value >= 0.0, "Bandwidth should be non-negative");
        }
    }

    #[test]
    fn test_keltner_bandwidth_reset() {
        let mut kb = KeltnerBandwidth::new(20, 2.0);
        for i in 0..25 {
            kb.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        kb.reset();
        assert!(!kb.is_ready());
        assert_eq!(kb.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_keltbw() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<KeltnerBandwidth as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Keltbw(cfg).build_solo().unwrap();
        for i in 0..25 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: price + 1.0, low: price - 1.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
