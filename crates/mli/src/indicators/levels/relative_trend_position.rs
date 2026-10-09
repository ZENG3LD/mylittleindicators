// Relative Trend Position: relative distance to MA and Anchored VWAP (monthly)

use crate::engine::contract_engine::SmootherSlot;
use crate::engine::contract_engine::SmootherId;
use crate::indicators::levels::anchored_vwap::{
    AnchoredVwap, AnchoredVwapParams, AvwapAnchorMode,
};

#[derive(Debug, Clone)]
pub struct RelativeTrendPosition {
    ma: SmootherSlot,
    avwap: AnchoredVwap,
    last_sma_rel: f64,
    last_avwap_rel: f64,
}

impl RelativeTrendPosition {
    pub fn new(sma_period: usize) -> Self {
        Self::from_smoother(sma_period, SmootherId::Sma)
    }

    /// Create with a configurable MA smoother.
    pub fn from_smoother(sma_period: usize, smoother: SmootherId) -> Self {
        let params = AnchoredVwapParams { mode: AvwapAnchorMode::Monthly };
        Self {
            ma: SmootherSlot::new(smoother, sma_period),
            avwap: AnchoredVwap::new(params),
            last_sma_rel: 0.0,
            last_avwap_rel: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ma.reset();
        self.avwap.reset();
        self.last_sma_rel = 0.0;
        self.last_avwap_rel = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ma.is_ready() && self.avwap.is_ready()
    }

    /// Feed resolved input lanes `[high, low, close, volume]` — the pure core computation.
    ///
    /// Note: the cumulative AVWAP path (`feed_cumulative(&[h,l,c,v])`) does not perform monthly
    /// resets (no timestamp). The timestamp-aware anchor is the factory `feed(ts, &[lanes])` arm.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let sma_val = self.ma.feed(close);
        let avwap_val = self.avwap.feed_cumulative(&[high, low, close, lanes[3]]);
        self.last_sma_rel = if sma_val != 0.0 {
            (close - sma_val) / sma_val.abs().max(1e-9)
        } else {
            0.0
        };
        self.last_avwap_rel = if avwap_val != 0.0 {
            (close - avwap_val) / avwap_val.abs().max(1e-9)
        } else {
            0.0
        };
        (self.last_sma_rel, self.last_avwap_rel)
    }


    #[inline]
    pub fn sma_rel(&self) -> f64 {
        self.last_sma_rel
    }

    #[inline]
    pub fn avwap_rel(&self) -> f64 {
        self.last_avwap_rel
    }

    #[inline]
    pub fn values(&self) -> (f64, f64) {
        (self.last_sma_rel, self.last_avwap_rel)
    }

}

impl Default for RelativeTrendPosition {
    fn default() -> Self {
        Self::from_smoother(200, SmootherId::Sma)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, Slot, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`RelativeTrendPosition`] — one smoother slot for the MA
/// (AVWAP uses its own fixed monthly-anchor config).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct RelativeTrendPositionConfig {
    pub period: Param<usize>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl crate::contract::Config for RelativeTrendPositionConfig {
    fn defaults() -> Self {
        RelativeTrendPositionConfig {
            period: Param::Solo(200),
            ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn machine_defaults() -> Self {
        // period: Class A period — auto range(2,4048,1) is correct.
        // ma (#[slot] Param<SmootherChoice>): LEFT Solo — smoother sweep is a later wave.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}

impl Indicator for RelativeTrendPosition {
    const ID: IndicatorId = IndicatorId::RelTrendPos;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed 4-lane slice: H/L/C/V — needed for both the MA (uses close=lanes[2])
    /// and the AVWAP inner (needs H/L/C/V).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[Store::fixed(StoreKind::Scalar, 2)],
        inner: &[Port::new(IndicatorId::Avwap, &[IndicatorOutputId::Avwap])],
    };
    const SLOTS: &'static [Slot] = RelativeTrendPositionConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::RelTrendPosSmaRel),
        Output::centered(IndicatorOutputId::RelTrendPosAvwapRel),
    ];
    type Config = RelativeTrendPositionConfig;
    type Runtime = RelativeTrendPosition;

    fn create(cfg: RelativeTrendPositionConfig) -> RelativeTrendPosition {
        RelativeTrendPosition {
            ma: cfg.ma.resolved().build(cfg.period.resolved()),
            avwap: AnchoredVwap::new(AnchoredVwapParams { mode: AvwapAnchorMode::Monthly }),
            last_sma_rel: 0.0,
            last_avwap_rel: 0.0,
        }
    }

    fn slot_members(cfg: &RelativeTrendPositionConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}


impl Render for RelativeTrendPosition {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::RelTrendPosSmaRel, "Rel Trend Pos", Color::hex(0x00BCD4))
            .line_output(IndicatorOutputId::RelTrendPosAvwapRel, "vs AVWAP", Color::hex(0xFF9800))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_trend_position_creation() {
        let rtp = RelativeTrendPosition::new(20);
        assert!(!rtp.is_ready());
    }

    #[test]
    fn test_relative_trend_position_warmup() {
        let mut rtp = RelativeTrendPosition::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rtp.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
        }
        assert!(rtp.is_ready());
    }

    #[test]
    fn test_relative_trend_position_values() {
        let mut rtp = RelativeTrendPosition::new(20);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let (sma_rel, avwap_rel) = rtp.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(sma_rel.is_finite(), "SMA relative should be finite");
            assert!(avwap_rel.is_finite(), "AVWAP relative should be finite");
        }
    }

    #[test]
    fn test_relative_trend_position_reset() {
        let mut rtp = RelativeTrendPosition::new(20);
        for i in 0..25 {
            rtp.feed(&[101.0 + i as f64, 99.0 + i as f64, 100.0 + i as f64, 1000.0]);
        }
        rtp.reset();
        assert!(!rtp.is_ready());
    }

    #[test]
    fn test_relative_trend_position_legacy_update_bar() {
        let mut rtp = RelativeTrendPosition::from_smoother(20, SmootherId::Ema);
        let _ts = 1700000000_i64;
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (sr, ar) = rtp.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(sr.is_finite());
            assert!(ar.is_finite());
        }
    }

    #[test]
    fn test_factory_feeds_resolved_rel_trend_pos() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::RelTrendPos(<<RelativeTrendPosition as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..30 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, // not used
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 1000.0, // used by AVWAP
            });
        }
        // factory value() = primary output (sma_rel); both outputs must be finite
        assert!(f.primary().is_finite());
    }
}
