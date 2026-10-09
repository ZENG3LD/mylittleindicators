//! ARCH-LM test p-value. REAL implementation: the Engle (1982) LM statistic is
//! `LM = T·R²` from the auxiliary regression of ε²_t on its own lags, and
//! `LM ~ χ²(L)` asymptotically under the no-ARCH null, so the p-value is the
//! upper-tail survival `chi2_sf(LM, L)`. The prior version fabricated the
//! statistic (`R²·L·10`, then `exp(-stat/2)`) — both the ×10 scaling and the
//! exponential survival were invented and unrelated to the χ² tail.

use crate::indicators::statistics::arch_lm_proxy::ArchLmProxy;
use crate::indicators::utils::math::distributions::chi2_sf;

#[derive(Debug, Clone)]
pub struct ArchLmPvalueProxy {
    inner: ArchLmProxy,
    pub value: f64,
}

impl ArchLmPvalueProxy {
    pub fn new(window: usize, lags: usize) -> Self {
        Self {
            inner: ArchLmProxy::new(window, lags),
            value: 1.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.value = 1.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed one pre-extracted close price.
    pub fn feed(&mut self, c: f64) -> f64 {
        self.inner.feed(c);
        let (r2, n_obs, lags) = self.inner.lm_components();
        if n_obs == 0 || lags == 0 {
            self.value = 1.0;
            return self.value;
        }
        // LM = T·R²  ~  χ²(L)  under H₀: no ARCH effects.
        let lm = n_obs as f64 * r2;
        self.value = chi2_sf(lm, lags as f64).clamp(0.0, 1.0);
        self.value
    }
}

impl Default for ArchLmPvalueProxy {
    /// Factory defaults: window=50, lags=5.
    fn default() -> Self {
        Self::new(50, 5)
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

/// Typed dual-mode contract config for [`ArchLmPvalueProxy`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ArchLmPvalConfig {
    /// Return window fed to the inner ARCH-LM auxiliary regression.
    pub window: Param<usize>,
    /// Number of lags in the auxiliary regression.
    pub lags: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl Indicator for ArchLmPvalueProxy {
    const ID: IndicatorId = IndicatorId::ArchLmPval;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// Outer is O(1) — it reads the cached `lm_components()` from the inner; the
    /// inner's O(n) regression scan is charged via the Port edge.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::ArchLm, &[IndicatorOutputId::ArchLm])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::ArchLmPval)];
    type Config = ArchLmPvalConfig;
    type Runtime = ArchLmPvalueProxy;

    fn create(cfg: ArchLmPvalConfig) -> ArchLmPvalueProxy {
        ArchLmPvalueProxy::new(cfg.window.resolved(), cfg.lags.resolved())
    }

    fn source_fields(cfg: &ArchLmPvalConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for ArchLmPvalConfig {
    fn defaults() -> Self {
        ArchLmPvalConfig {
            window: Param::Solo(50),
            lags: Param::Solo(5),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn machine_defaults() -> Self {
        // window: Class A → auto range(2,4048,1).
        // source: Class O → auto (all 8 fields).
        // lags: Class A.model (ARCH lag order) → range(0,5,1), corrected off auto 2..4048.
        let mut s = Self::machine_defaults_auto();
        s.lags = Param::range(0, 5, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for ArchLmPvalueProxy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::ArchLmPval, "ARCH LM P-Value", Color::hex(0xFF9800))
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arch_lm_pvalue_proxy_creation() {
        let arch_p = ArchLmPvalueProxy::new(50, 5);
        assert!(!arch_p.is_ready());
        assert_eq!(arch_p.value, 1.0);
    }

    #[test]
    fn test_arch_lm_pvalue_proxy_warmup() {
        let mut arch_p = ArchLmPvalueProxy::new(50, 5);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            arch_p.feed(price);
        }
        assert!(arch_p.is_ready());
    }

    #[test]
    fn test_arch_lm_pvalue_proxy_range() {
        let mut arch_p = ArchLmPvalueProxy::new(50, 5);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = arch_p.feed(price);
            assert!(value >= 0.0 && value <= 1.0, "P-value should be in [0, 1]");
        }
    }

    #[test]
    fn test_arch_lm_pvalue_proxy_reset() {
        let mut arch_p = ArchLmPvalueProxy::new(50, 5);
        for i in 0..60 {
            arch_p.feed(100.0 + i as f64);
        }
        arch_p.reset();
        assert!(!arch_p.is_ready());
        assert_eq!(arch_p.value, 1.0);
    }

    fn lcg(n: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            out.push(((s >> 33) as f64) / (1u64 << 31) as f64 - 1.0);
        }
        out
    }

    #[test]
    fn arch_effects_lower_pvalue_than_homoskedastic() {
        // Homoskedastic returns → no ARCH → p-value should be large (fail to
        // reject H₀). Build a price path from i.i.d. small returns.
        let mut homo = ArchLmPvalueProxy::new(120, 4);
        let mut p = 100.0;
        for &e in &lcg(160, 11) {
            p *= 1.0 + 0.01 * e;
            homo.feed(p);
        }
        // Strong volatility clustering → ARCH present → p-value should be small.
        let mut arch = ArchLmPvalueProxy::new(120, 4);
        let z = lcg(160, 29);
        let mut prev = 0.0_f64;
        let mut pa = 100.0;
        for &e in &z {
            // σ²_t depends on the prior squared shock → clustering.
            let sigma = (0.0001 + 0.85 * prev * prev).sqrt();
            let ret = sigma * e;
            prev = ret;
            pa *= 1.0 + ret;
            arch.feed(pa);
        }
        assert!(
            arch.value() < homo.value(),
            "ARCH p {} should be < homoskedastic p {}",
            arch.value(),
            homo.value()
        );
    }

    /// Factory resolves close (not wild 9999.0 high) and feeds the p-value proxy.
    #[test]
    fn factory_feeds_resolved_arch_lm_pval() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<ArchLmPvalueProxy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::ArchLmPval(cfg).build_solo().unwrap();
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        let v = f.read(IndicatorOutputId::ArchLmPval);
        assert!(v >= 0.0 && v <= 1.0, "p-value should be in [0,1], got {v}");
    }
}
