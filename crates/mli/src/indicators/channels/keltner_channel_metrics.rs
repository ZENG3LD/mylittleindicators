use crate::indicators::channels::keltner_channel::{KeltnerChannel, KeltnerMode};
use crate::engine::ohlcv_field::OhlcvField;

/// Lightweight metrics over Keltner Channel: width and position
#[derive(Debug, Clone)]
pub struct KeltnerMetrics {
    kc: KeltnerChannel,
    width: f64,
    position: f64,
}

impl Default for KeltnerMetrics {
    fn default() -> Self {
        Self::with_smoothers(14, 2.0, SmootherId::Sma, SmootherId::Rma)
    }
}

impl KeltnerMetrics {
    pub fn new(period: usize, atr_mult: f64) -> Self {
        Self::with_smoothers(period, atr_mult, SmootherId::Sma, SmootherId::Rma)
    }

    pub fn with_smoothers(
        period: usize,
        atr_mult: f64,
        center: SmootherId,
        atr_smoother: SmootherId,
    ) -> Self {
        Self {
            kc: KeltnerChannel::from_smoothers(
                center, atr_smoother, period, atr_mult, KeltnerMode::Classic, OhlcvField::Close,
            ),
            width: 0.0,
            position: 0.5,
        }
    }

    /// Feed HIGH, LOW, CLOSE lanes (const SOURCE order).
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let (upper, _middle, lower) = self.kc.feed(lanes);
        self.width = upper - lower;
        let close = lanes[2];
        self.position = if self.width > 0.0 { (close - lower) / self.width } else { 0.5 };
        (self.width, self.position)
    }

    /// Legacy bar-level update.

    pub fn width(&self) -> f64 { self.width }
    pub fn position(&self) -> f64 { self.position }
    #[inline] pub fn is_ready(&self) -> bool { self.kc.is_ready() }
    pub fn reset(&mut self) { self.kc.reset(); self.width = 0.0; self.position = 0.5; }
}

// ─── Contract ────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::{IndicatorOutputId, SmootherId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::{Param, sweep_f64};
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Port, Slot, SourceAxis, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::{Color, Render, RenderSpec};

/// Typed dual-mode config for [`KeltnerMetrics`].
///
/// ABSORB: builds `KeltnerChannel` via `KeltnerChannel::from_smoothers`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct KeltnerMetricsConfig {
    pub period: Param<usize>,
    pub multiplier: Param<f64>,
    /// Center-line smoother slot. Default Sma at the host period (14).
    #[slot]
    pub ma: Param<SmootherSlotOrder>,
    /// ATR smoother slot. Default Rma (Wilder) at the host period (14).
    #[slot]
    pub atr_ma: Param<SmootherSlotOrder>,
}

impl Indicator for KeltnerMetrics {
    const ID: IndicatorId = IndicatorId::Kcmetrics;
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
    const SLOTS: &'static [Slot] = KeltnerMetricsConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::magnitude(IndicatorOutputId::KcmetricsWidth),
        Output::percent(IndicatorOutputId::KcmetricsPosition),
    ];
    type Config = KeltnerMetricsConfig;
    type Runtime = Self;

    fn create(cfg: KeltnerMetricsConfig) -> Self {
        let p = cfg.period.resolved();
        let ma_order = cfg.ma.resolved();
        let atr_order = cfg.atr_ma.resolved();
        KeltnerMetrics::with_smoothers(p, cfg.multiplier.resolved(), ma_order.id(), atr_order.id())
    }

    fn slot_members(cfg: &KeltnerMetricsConfig) -> Vec<IndicatorId> { cfg.slot_members() }
}

impl crate::contract::Config for KeltnerMetricsConfig {
    /// Period 14, multiplier 2.0, SMA center at period 14, RMA ATR at period 14.
    fn defaults() -> Self {
        KeltnerMetricsConfig {
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


impl Render for KeltnerMetrics {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::KcmetricsWidth, "KC Width", Color::hex(0x2196F3))
            .line_output(IndicatorOutputId::KcmetricsPosition, "KC Position", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keltner_metrics_creation() {
        let km = KeltnerMetrics::new(20, 2.0);
        assert!(!km.is_ready());
        assert_eq!(km.width(), 0.0);
        assert_eq!(km.position(), 0.5);
    }

    #[test]
    fn test_keltner_metrics_warmup() {
        let mut km = KeltnerMetrics::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            km.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(km.is_ready());
    }

    #[test]
    fn test_keltner_metrics_reset() {
        let mut km = KeltnerMetrics::new(20, 2.0);
        for i in 0..25 {
            km.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        km.reset();
        assert!(!km.is_ready());
        assert_eq!(km.position(), 0.5);
    }

    #[test]
    fn factory_feeds_resolved_kcmetrics() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<KeltnerMetrics as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Kcmetrics(cfg).build_solo().unwrap();
        for i in 0..25 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: price + 1.0, low: price - 1.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
