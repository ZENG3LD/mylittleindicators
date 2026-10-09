// KPSS Z-proxy: standardize KPSS-proxy statistic to z-score over rolling window

use crate::indicators::statistics::kpss_proxy::KpssProxy;

#[derive(Debug, Clone)]
pub struct KpssZProxy {
    inner: KpssProxy,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl KpssZProxy {
    pub fn new(window_stat: usize, window_z: usize) -> Self {
        let inner = KpssProxy::new(window_stat);
        let w = window_z.max(20);
        Self {
            inner,
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.inner.is_ready()
    }

    /// Feed one pre-extracted close price.
    pub fn feed(&mut self, c: f64) -> f64 {
        let s = self.inner.feed(c);
        self.buf[self.idx] = s;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let n = self.window;
            let mut mean = 0.0;
            for i in 0..n {
                mean += self.buf[i];
            }
            mean /= n as f64;
            let mut var = 0.0;
            for i in 0..n {
                let d = self.buf[i] - mean;
                var += d * d;
            }
            let std = (var / (n as f64)).sqrt().max(1e-9);
            self.value = (s - mean) / std;
        }
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }
}

impl Default for KpssZProxy {
    /// Factory defaults: window_stat=50, window_z=100.
    fn default() -> Self {
        Self::new(50, 100)
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

/// Typed dual-mode contract config for [`KpssZProxy`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct KpssZConfig {
    /// Window for the inner KPSS statistic computation.
    pub window_stat: Param<usize>,
    /// Rolling window over which the z-score is computed.
    pub window_z: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl Indicator for KpssZProxy {
    const ID: IndicatorId = IndicatorId::KpssZ;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// Outer O(n) rescan over the z-score buf; inner KPSS cost via Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Kpss, &[IndicatorOutputId::Kpss])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::KpssZ)];
    type Config = KpssZConfig;
    type Runtime = KpssZProxy;

    fn create(cfg: KpssZConfig) -> KpssZProxy {
        KpssZProxy::new(cfg.window_stat.resolved(), cfg.window_z.resolved())
    }

    fn source_fields(cfg: &KpssZConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for KpssZConfig {
    fn defaults() -> Self {
        KpssZConfig {
            window_stat: Param::Solo(50),
            window_z: Param::Solo(100),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn machine_defaults() -> Self {
        // window_stat: Class A → auto range(2,4048,1).
        // window_z: Class A → auto range(2,4048,1).
        // source: Class O → auto (all 8 fields).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for KpssZProxy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::KpssZ, "KPSS Z", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kpss_z_proxy_creation() {
        let kpssz = KpssZProxy::new(50, 30);
        assert!(!kpssz.is_ready());
        assert_eq!(kpssz.value, 0.0);
    }

    #[test]
    fn test_kpss_z_proxy_warmup() {
        let mut kpssz = KpssZProxy::new(50, 30);
        for i in 0..90 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kpssz.feed(price);
        }
        assert!(kpssz.is_ready());
    }

    #[test]
    fn test_kpss_z_proxy_values() {
        let mut kpssz = KpssZProxy::new(50, 30);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kpssz.feed(price);
            assert!(value.is_finite(), "Z-score should be finite");
        }
    }

    #[test]
    fn test_kpss_z_proxy_reset() {
        let mut kpssz = KpssZProxy::new(50, 30);
        for i in 0..90 {
            kpssz.feed(100.0 + i as f64);
        }
        kpssz.reset();
        assert!(!kpssz.is_ready());
        assert_eq!(kpssz.value, 0.0);
    }

    /// Factory resolves close (not wild 9999.0 high) and feeds the KPSS-Z proxy.
    #[test]
    fn factory_feeds_resolved_kpss_z() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<KpssZProxy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::KpssZ(cfg).build_solo().unwrap();
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.read(IndicatorOutputId::KpssZ).is_finite(), "KPSS-Z should produce a finite value");
    }
}
