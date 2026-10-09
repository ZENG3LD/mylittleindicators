// AVWAP Distance: relative distance of close to anchored VWAP (monthly)

use crate::indicators::levels::anchored_vwap::{
    AnchoredVwap, AnchoredVwapParams, AvwapAnchorMode,
};

#[derive(Debug, Clone)]
pub struct AvwapDistance {
    avwap: AnchoredVwap,
    pub value: f64,
}

impl Default for AvwapDistance {
    fn default() -> Self {
        Self::new()
    }
}

impl AvwapDistance {
    pub fn new() -> Self {
        let params = AnchoredVwapParams {
            mode: AvwapAnchorMode::Monthly,
        };
        Self {
            avwap: AnchoredVwap::new(params),
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.avwap.reset();
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.avwap.is_ready()
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed `(ts, &[H,L,C,V])` — the `Fields +time` contract arm. The monthly anchor reset uses
    /// the wall-clock (canonical ms); typical-price weighting uses H/L/C/V. Returns the distance.
    pub fn feed(&mut self, ts_ms: i64, lanes: &[f64]) -> f64 {
        let close = lanes[2];
        let v = self.avwap.feed(ts_ms, lanes);
        self.value = if v != 0.0 { (close - v) / v } else { 0.0 };
        self.value
    }
}

// ── contract ──────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Port, RenderSpec, SourceAxis, UpdateComplexity,
};
use crate::contract::Render;
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`AvwapDistance`] — no parameters; always monthly anchor.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct AvwapDistanceConfig;

impl crate::contract::Config for AvwapDistanceConfig {
    fn defaults() -> Self {
        AvwapDistanceConfig
    }
    fn machine_defaults() -> Self {
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}

impl Indicator for AvwapDistance {
    const ID: IndicatorId = IndicatorId::AvwapDist;
    /// Positional distance measure — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed 4-lane slice (H/L/C/V); the `Fields +time` arm resolves these lanes and feeds
    /// `(ts, &[H,L,C,V])` to `feed`.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
    /// Outer is O(1); inner `AnchoredVwap` cost is charged via the Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Avwap, &[IndicatorOutputId::Avwap])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::AvwapDist)];

    type Config = AvwapDistanceConfig;
    type Runtime = AvwapDistance;

    fn create(_cfg: AvwapDistanceConfig) -> AvwapDistance {
        AvwapDistance::new()
    }
}


impl Render for AvwapDistance {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(
                IndicatorOutputId::AvwapDist,
                "AVWAP Distance",
                Color::hex(0x9C27B0),
            )
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avwap_distance_creation() {
        let ad = AvwapDistance::new();
        assert!(!ad.is_ready());
        assert_eq!(ad.value, 0.0);
    }

    #[test]
    fn test_avwap_distance_update() {
        let mut ad = AvwapDistance::new();
        let ts_ms = 1_700_000_000_000_i64;
        let value = ad.feed(ts_ms, &[102.0, 98.0, 101.0, 1000.0]);
        assert!(ad.is_ready());
        assert!(value.is_finite());
    }

    #[test]
    fn test_avwap_distance_values() {
        let mut ad = AvwapDistance::new();
        let ts_ms = 1_700_000_000_000_i64;
        for i in 0..10 {
            let price = 100.0 + i as f64;
            let value = ad.feed(ts_ms + i * 86_400_000, &[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(value.is_finite(), "Distance should be finite");
        }
    }

    #[test]
    fn test_avwap_distance_reset() {
        let mut ad = AvwapDistance::new();
        ad.feed(1_700_000_000_000, &[102.0, 98.0, 101.0, 1000.0]);
        ad.reset();
        assert!(!ad.is_ready());
        assert_eq!(ad.value, 0.0);
    }

    #[test]
    fn test_feed_deterministic() {
        let mut ad1 = AvwapDistance::new();
        let mut ad2 = AvwapDistance::new();
        let ts_ms = 1_700_000_000_000_i64;
        for i in 0..5 {
            let (h, l, c, v) = (102.0 + i as f64, 98.0 + i as f64, 101.0 + i as f64, 1000.0);
            let t = ts_ms + i * 86_400_000;
            ad1.feed(t, &[h, l, c, v]);
            ad2.feed(t, &[h, l, c, v]);
        }
        assert_eq!(ad1.value, ad2.value);
    }

    #[test]
    fn factory_feeds_resolved_avwap_dist() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::AvwapDist(<<AvwapDistance as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        let ts_base = 1_700_000_000_000_i64;
        for i in 0..10 {
            let price = 100.0 + i as f64;
            // `Fields +time` arm — ts is the orthogonal first feed arg (canonical ms)
            f.feed(ts_base + i * 86_400_000, MarketSample::Bar {
                open: price,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 1000.0,
            });
        }
        let v = f.primary();
        assert!(v.is_finite());
    }
}
