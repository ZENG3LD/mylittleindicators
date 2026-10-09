// VWAP Distance: (Close - VWAP) / VWAP

use crate::indicators::average::vwap::Vwap;

#[derive(Debug, Clone)]
pub struct VwapDistance {
    vwap: Vwap,
    value: f64,
}

impl VwapDistance {
    pub fn new(period: usize) -> Self {
        Self {
            vwap: Vwap::new(period.max(1)),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.vwap.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.vwap.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed the resolved input lanes — `[high, low, close, volume]`.
    /// Drives the inner VWAP with H/L/C/V and computes (close - vwap) / vwap.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        let c = lanes[2];
        let v = lanes[3];
        let vwap_val = self.vwap.feed(&[h, l, c, v]);
        self.value = if vwap_val.abs() > 1e-12 {
            (c - vwap_val) / vwap_val
        } else {
            0.0
        };
        self.value
    }
}

impl Default for VwapDistance {
    fn default() -> Self {
        Self::new(14)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, SourceAxis, SourceLane, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`VwapDistance`] — VWAP window period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VwapDistConfig {
    pub period: Param<usize>,
}

impl Indicator for VwapDistance {
    const ID: IndicatorId = IndicatorId::VwapDist;
    /// Positional measure relative to VWAP — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed four-lane slice: H/L/C for VWAP typical-price, V for VWAP weighting.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Lanes(&[
        SourceLane::Fixed(OhlcvField::High),
        SourceLane::Fixed(OhlcvField::Low),
        SourceLane::Fixed(OhlcvField::Close),
        SourceLane::Fixed(OhlcvField::Volume),
    ]));
    const NEEDS_VOLUME: bool = true;
    /// Outer is O(1) — VWAP's own cost is charged via the Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Vwap, &[IndicatorOutputId::Vwap])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::VwapDist)];

    type Config = VwapDistConfig;
    type Runtime = VwapDistance;

    fn create(cfg: VwapDistConfig) -> VwapDistance {
        VwapDistance::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for VwapDistConfig {
    fn defaults() -> Self {
        VwapDistConfig { period: Param::Solo(14) }
    }
    fn machine_defaults() -> Self {
        // period: Class A VWAP window period — auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VwapDistance {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::VwapDist, "VWAP Distance", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vwap_distance_creation() {
        let vd = VwapDistance::new(20);
        assert!(!vd.is_ready());
        assert_eq!(vd.value(), 0.0);
    }

    #[test]
    fn test_vwap_distance_warmup() {
        let mut vd = VwapDistance::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vd.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
        }
        assert!(vd.is_ready());
    }

    #[test]
    fn test_vwap_distance_values() {
        let mut vd = VwapDistance::new(20);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let value = vd.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(value.is_finite(), "Distance should be finite");
        }
    }

    #[test]
    fn test_vwap_distance_reset() {
        let mut vd = VwapDistance::new(20);
        for i in 0..25 {
            vd.feed(&[101.0 + i as f64, 99.0 + i as f64, 100.0 + i as f64, 1000.0]);
        }
        vd.reset();
        assert!(!vd.is_ready());
        assert_eq!(vd.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_vwap_dist() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::VwapDist(<<VwapDistance as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..30 {
            let price = 100.0 + i as f64;
            // SOURCE is Fixed lanes — factory reads H/L/C/V; open=9999 is not used
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
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
