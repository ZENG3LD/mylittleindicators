// ARCH LM p-value proxy based on R^2 and dof

use crate::bar_indicators::statistics::arch_lm_proxy::ArchLmProxy;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct ArchLmPvalueProxy {
    inner: ArchLmProxy,
    dof: usize,
    pub value: f64,
}

impl ArchLmPvalueProxy {
    pub fn new(window: usize, lags: usize) -> Self {
        Self {
            inner: ArchLmProxy::new(window, lags),
            dof: lags,
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
        let r2 = self.inner.update_bar(o, h, l, c, v);
        let k = self.dof.max(1) as f64;
        let stat = r2 * (k.max(1.0) * 10.0); // simple scaled proxy for LM
                                             // use simple survival proxy: p ≈ exp(-stat/2)
        self.value = (-0.5 * stat).exp().clamp(0.0, 1.0);
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
}
