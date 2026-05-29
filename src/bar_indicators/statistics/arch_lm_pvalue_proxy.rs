//! ARCH-LM test p-value. REAL implementation: the Engle (1982) LM statistic is
//! `LM = T·R²` from the auxiliary regression of ε²_t on its own lags, and
//! `LM ~ χ²(L)` asymptotically under the no-ARCH null, so the p-value is the
//! upper-tail survival `chi2_sf(LM, L)`. The prior version fabricated the
//! statistic (`R²·L·10`, then `exp(-stat/2)`) — both the ×10 scaling and the
//! exponential survival were invented and unrelated to the χ² tail.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::statistics::arch_lm_proxy::ArchLmProxy;
use crate::bar_indicators::utils::math::distributions::chi2_sf;

#[derive(Clone)]
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

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        self.inner.update_bar(o, h, l, c, v);
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
            arch_p.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(arch_p.is_ready());
    }

    #[test]
    fn test_arch_lm_pvalue_proxy_range() {
        let mut arch_p = ArchLmPvalueProxy::new(50, 5);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = arch_p.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0, "P-value should be in [0, 1]");
        }
    }

    #[test]
    fn test_arch_lm_pvalue_proxy_reset() {
        let mut arch_p = ArchLmPvalueProxy::new(50, 5);
        for i in 0..60 {
            arch_p.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
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
            homo.update_bar(0.0, 0.0, 0.0, p, 0.0);
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
            arch.update_bar(0.0, 0.0, 0.0, pa, 0.0);
        }
        assert!(
            arch.value().main() < homo.value().main(),
            "ARCH p {} should be < homoskedastic p {}",
            arch.value().main(),
            homo.value().main()
        );
    }
}
