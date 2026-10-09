// Variance Ratio Aggregate: combine multiple VR(m) into a single score

use crate::indicators::statistics::variance_ratio::VarianceRatio;

#[derive(Debug, Clone)]
pub struct VarianceRatioAggregate {
    vr_list: Vec<VarianceRatio>,
    pub value: f64,
}

impl VarianceRatioAggregate {
    pub fn new(configs: &[(usize, usize)]) -> Self {
        let mut vr_list = Vec::new();
        for &(w, m) in configs {
            vr_list.push(VarianceRatio::new(w, m));
        }
        Self {
            vr_list,
            value: 1.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        for vr in &mut self.vr_list {
            vr.reset();
        }
        self.value = 1.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.vr_list.iter().all(|v| v.is_ready())
    }

    /// Feed one pre-extracted close price.
    pub fn feed(&mut self, c: f64) -> f64 {
        let mut sum = 0.0;
        let mut cnt = 0.0;
        for vr in &mut self.vr_list {
            sum += vr.feed(c);
            cnt += 1.0;
        }
        if cnt > 0.0 {
            self.value = sum / cnt;
        }
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }
}

impl Default for VarianceRatioAggregate {
    /// Factory defaults: three pairs (window=100, m=2), (100, 5), (100, 10).
    fn default() -> Self {
        Self::new(&[(100, 2), (100, 5), (100, 10)])
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Port, Render, RenderSpec, SourceAxis,
    Store, StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode contract config for [`VarianceRatioAggregate`].
///
/// The aggregate instantiates a fixed set of (window, m) pairs; exposed here as
/// three named fields matching the factory default. Multi-pair config is not
/// expressible as a simple typed struct without a Vec, so we fix the three-pair
/// default and expose it as the contract config.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VrAggConfig {
    /// Window for all three VR(m) instances.
    pub window: Param<usize>,
    /// Short-lag m.
    pub m1: Param<usize>,
    /// Mid-lag m.
    pub m2: Param<usize>,
    /// Long-lag m.
    pub m3: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl Indicator for VarianceRatioAggregate {
    const ID: IndicatorId = IndicatorId::VrAgg;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Vr, &[IndicatorOutputId::Vr])],
    };
    const OUTPUTS: &'static [Output] = &[Output::ratio(IndicatorOutputId::VrAgg)];
    type Config = VrAggConfig;
    type Runtime = VarianceRatioAggregate;

    fn create(cfg: VrAggConfig) -> VarianceRatioAggregate {
        let w = cfg.window.resolved();
        VarianceRatioAggregate::new(&[(w, cfg.m1.resolved()), (w, cfg.m2.resolved()), (w, cfg.m3.resolved())])
    }

    fn source_fields(cfg: &VrAggConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for VrAggConfig {
    fn defaults() -> Self {
        VrAggConfig {
            window: Param::Solo(100),
            m1: Param::Solo(2),
            m2: Param::Solo(5),
            m3: Param::Solo(10),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn machine_defaults() -> Self {
        // window: Class A → auto range(2,4048,1).
        // source: Class O → auto (all 8 fields).
        // m1/m2/m3: VR step horizons — A.lag-like → range(2,50,1), corrected off auto 2..4048.
        let mut s = Self::machine_defaults_auto();
        s.m1 = Param::range(2, 50, 1);
        s.m2 = Param::range(2, 50, 1);
        s.m3 = Param::range(2, 50, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VarianceRatioAggregate {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::VrAgg, "VR Aggregate", Color::hex(0x009688))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variance_ratio_aggregate_creation() {
        let vra = VarianceRatioAggregate::new(&[(50, 5), (50, 10)]);
        assert!(!vra.is_ready());
        assert_eq!(vra.value, 1.0);
    }

    #[test]
    fn test_variance_ratio_aggregate_warmup() {
        let mut vra = VarianceRatioAggregate::new(&[(50, 5), (50, 10)]);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vra.feed(price);
        }
        assert!(vra.is_ready());
    }

    #[test]
    fn test_variance_ratio_aggregate_positive() {
        let mut vra = VarianceRatioAggregate::new(&[(50, 5), (50, 10)]);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vra.feed(price);
            assert!(value > 0.0, "VR aggregate should be positive");
        }
    }

    #[test]
    fn test_variance_ratio_aggregate_reset() {
        let mut vra = VarianceRatioAggregate::new(&[(50, 5), (50, 10)]);
        for i in 0..60 {
            vra.feed(100.0 + i as f64);
        }
        vra.reset();
        assert!(!vra.is_ready());
        assert_eq!(vra.value, 1.0);
    }

    /// Factory resolves close (not the wild 9999.0 high) and feeds the aggregate.
    #[test]
    fn factory_feeds_resolved_vr_agg() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<VarianceRatioAggregate as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::VrAgg(cfg).build_solo().unwrap();
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.read(IndicatorOutputId::VrAgg) > 0.0, "VR aggregate should be positive after warmup");
    }
}
