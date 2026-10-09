// Keltner Distance: distance of price from Keltner center in ATR units

use crate::indicators::channels::keltner_channel::{KeltnerChannel, KeltnerMode};
use crate::engine::ohlcv_field::OhlcvField;

#[derive(Debug, Clone)]
pub struct KeltnerDistance {
    kc: KeltnerChannel,
    value: f64,
}

impl Default for KeltnerDistance {
    fn default() -> Self {
        Self::with_smoothers(14, 2.0, SmootherId::Sma, SmootherId::Rma)
    }
}

impl KeltnerDistance {
    pub fn new(period: usize, multiplier: f64) -> Self {
        Self::with_smoothers(period, multiplier, SmootherId::Sma, SmootherId::Rma)
    }

    pub fn with_smoothers(
        period: usize,
        multiplier: f64,
        center: SmootherId,
        atr_smoother: SmootherId,
    ) -> Self {
        Self {
            kc: KeltnerChannel::from_smoothers(
                center, atr_smoother, period.max(2), multiplier.max(0.1),
                KeltnerMode::Classic, OhlcvField::Close,
            ),
            value: 0.0,
        }
    }

    /// Feed HIGH, LOW, CLOSE lanes.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let _ = self.kc.feed(lanes);
        let close = lanes[2];
        self.value = self.kc.distance_from_center_atr(close);
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
use crate::contract::{Cost, Family, Indicator, Output, Port, Slot, SourceAxis, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::{Color, Render, RenderSpec};

/// Typed dual-mode config for [`KeltnerDistance`].
///
/// ABSORB: builds `KeltnerChannel` via `KeltnerChannel::from_smoothers`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct KeltnerDistanceConfig {
    pub period: Param<usize>,
    pub multiplier: Param<f64>,
    /// Center-line smoother slot. Default Sma at the host period (14).
    #[slot]
    pub ma: Param<SmootherSlotOrder>,
    /// ATR smoother slot. Default Rma (Wilder) at the host period (14).
    #[slot]
    pub atr_ma: Param<SmootherSlotOrder>,
}

impl Indicator for KeltnerDistance {
    const ID: IndicatorId = IndicatorId::Keltdist;
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
    const SLOTS: &'static [Slot] = KeltnerDistanceConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Keltdist)];
    type Config = KeltnerDistanceConfig;
    type Runtime = Self;

    fn create(cfg: KeltnerDistanceConfig) -> Self {
        let p = cfg.period.resolved();
        let ma_order = cfg.ma.resolved();
        let atr_order = cfg.atr_ma.resolved();
        KeltnerDistance::with_smoothers(p, cfg.multiplier.resolved(), ma_order.id(), atr_order.id())
    }

    fn slot_members(cfg: &KeltnerDistanceConfig) -> Vec<IndicatorId> { cfg.slot_members() }
}

impl crate::contract::Config for KeltnerDistanceConfig {
    /// Period 14, multiplier 2.0, SMA center at period 14, RMA ATR at period 14.
    fn defaults() -> Self {
        KeltnerDistanceConfig {
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


impl Render for KeltnerDistance {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Keltdist, "Distance", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keltner_distance_creation() {
        let kd = KeltnerDistance::new(20, 2.0);
        assert!(!kd.is_ready());
        assert_eq!(kd.value(), 0.0);
    }

    #[test]
    fn test_keltner_distance_warmup() {
        let mut kd = KeltnerDistance::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kd.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(kd.is_ready());
    }

    #[test]
    fn test_keltner_distance_finite() {
        let mut kd = KeltnerDistance::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kd.feed(&[price + 1.0, price - 1.0, price]);
            assert!(value.is_finite(), "Distance should be finite");
        }
    }

    #[test]
    fn test_keltner_distance_reset() {
        let mut kd = KeltnerDistance::new(20, 2.0);
        for i in 0..25 {
            kd.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        kd.reset();
        assert!(!kd.is_ready());
        assert_eq!(kd.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_keltdist() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<KeltnerDistance as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Keltdist(cfg).build_solo().unwrap();
        for i in 0..25 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: price + 1.0, low: price - 1.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
