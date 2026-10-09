// Variance Ratio Z-Aggregate: z-score over mean of multiple VR(m)

use crate::indicators::statistics::variance_ratio::VarianceRatio;

#[derive(Debug, Clone)]
pub struct VarianceRatioZAggregate {
    vr_list: Vec<VarianceRatio>,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl VarianceRatioZAggregate {
    pub fn new(configs: &[(usize, usize)], z_window: usize) -> Self {
        let mut vr_list = Vec::new();
        for &(w, m) in configs {
            vr_list.push(VarianceRatio::new(w, m));
        }
        let w = z_window.max(20);
        Self {
            vr_list,
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        for v in &mut self.vr_list {
            v.reset();
        }
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.vr_list.iter().all(|v| v.is_ready())
    }

    /// Feed one pre-extracted close price.
    pub fn feed(&mut self, c: f64) -> f64 {
        let mut sum = 0.0;
        let mut cnt = 0.0;
        for vr in &mut self.vr_list {
            sum += vr.feed(c);
            cnt += 1.0;
        }
        let mean = if cnt > 0.0 { sum / cnt } else { 1.0 };
        self.buf[self.idx] = mean;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut m = 0.0;
            for &x in &self.buf {
                m += x;
            }
            m /= self.window as f64;
            let mut s = 0.0;
            for &x in &self.buf {
                let d = x - m;
                s += d * d;
            }
            s = (s / (self.window as f64)).sqrt().max(1e-9);
            self.value = (mean - m) / s;
        } else {
            self.value = 0.0;
        }
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }
}

impl Default for VarianceRatioZAggregate {
    /// Factory defaults: configs=[(2,10),(5,10),(10,20)], z_window=100.
    fn default() -> Self {
        Self::new(&[(2, 10), (5, 10), (10, 20)], 100)
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

/// Typed dual-mode contract config for [`VarianceRatioZAggregate`].
///
/// Three (window, m) pairs plus the z-score rolling window. Matches the factory
/// default layout; the struct captures the fixed-arity default configuration.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VrZAggConfig {
    /// Window for the first VR instance (inner m1).
    pub w1: Param<usize>,
    pub m1: Param<usize>,
    /// Window for the second VR instance (inner m2).
    pub w2: Param<usize>,
    pub m2: Param<usize>,
    /// Window for the third VR instance (inner m3).
    pub w3: Param<usize>,
    pub m3: Param<usize>,
    /// Rolling z-score window.
    pub z_window: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl Indicator for VarianceRatioZAggregate {
    const ID: IndicatorId = IndicatorId::VrZAgg;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[
            Store::window(StoreKind::Vec), // vr_list inner windows
            Store::window(StoreKind::Vec), // z-score buf
        ],
        inner: &[Port::new(IndicatorId::Vr, &[IndicatorOutputId::Vr])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::VrZAgg)];
    type Config = VrZAggConfig;
    type Runtime = VarianceRatioZAggregate;

    fn create(cfg: VrZAggConfig) -> VarianceRatioZAggregate {
        VarianceRatioZAggregate::new(
            &[
                (cfg.w1.resolved(), cfg.m1.resolved()),
                (cfg.w2.resolved(), cfg.m2.resolved()),
                (cfg.w3.resolved(), cfg.m3.resolved()),
            ],
            cfg.z_window.resolved(),
        )
    }

    fn source_fields(cfg: &VrZAggConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for VrZAggConfig {
    fn defaults() -> Self {
        VrZAggConfig {
            w1: Param::Solo(2), m1: Param::Solo(10),
            w2: Param::Solo(5), m2: Param::Solo(10),
            w3: Param::Solo(10), m3: Param::Solo(20),
            z_window: Param::Solo(100),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn machine_defaults() -> Self {
        // w1/w2/w3: Class A (VR inner window sizes) → auto range(2,4048,1).
        // z_window: Class A → auto range(2,4048,1).
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


impl Render for VarianceRatioZAggregate {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::VrZAgg, "VR Z-Score", Color::hex(0x009688))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variance_ratio_z_aggregate_creation() {
        let vrza = VarianceRatioZAggregate::new(&[(50, 5), (50, 10)], 30);
        assert!(!vrza.is_ready());
        assert_eq!(vrza.value, 0.0);
    }

    #[test]
    fn test_variance_ratio_z_aggregate_warmup() {
        let mut vrza = VarianceRatioZAggregate::new(&[(50, 5), (50, 10)], 30);
        for i in 0..90 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vrza.feed(price);
        }
        assert!(vrza.is_ready());
    }

    #[test]
    fn test_variance_ratio_z_aggregate_values() {
        let mut vrza = VarianceRatioZAggregate::new(&[(50, 5), (50, 10)], 30);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vrza.feed(price);
            assert!(value.is_finite(), "Z-score should be finite");
        }
    }

    #[test]
    fn test_variance_ratio_z_aggregate_reset() {
        let mut vrza = VarianceRatioZAggregate::new(&[(50, 5), (50, 10)], 30);
        for i in 0..90 {
            vrza.feed(100.0 + i as f64);
        }
        vrza.reset();
        assert!(!vrza.is_ready());
        assert_eq!(vrza.value, 0.0);
    }

    /// Factory resolves close (not wild 9999.0 high) and feeds the z-aggregate.
    #[test]
    fn factory_feeds_resolved_vr_z_agg() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<VarianceRatioZAggregate as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::VrZAgg(cfg).build_solo().unwrap();
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.read(IndicatorOutputId::VrZAgg).is_finite(), "VR Z-aggregate should produce a finite value");
    }
}
