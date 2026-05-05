// Engle–Granger p-proxy: wraps CointegrationProxy and produces a rough p-value proxy from t-stat

use crate::bar_indicators::statistics::cointegration_proxy::CointegrationProxy;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct EngleGrangerProxy {
    inner: CointegrationProxy,
    pub t_stat: f64,
    pub p_proxy: f64,
}

impl EngleGrangerProxy {
    pub fn new(window: usize) -> Self {
        Self {
            inner: CointegrationProxy::new(window),
            t_stat: 0.0,
            p_proxy: 1.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.t_stat = 0.0;
        self.p_proxy = 1.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.p_proxy)
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let (_phi, t) = self.inner.update_bar(open, high, low, close, volume);
        self.t_stat = t;
        // EG is left-tail (more negative -> stronger cointegration). Convert to one-tailed p.
        self.p_proxy = 1.0 - Self::normal_cdf(t);
        self.p_proxy
    }

    // Fast normal CDF approximation (Abramowitz-Stegun 7.1.26 via erf)
    #[inline]
    fn normal_cdf(x: f64) -> f64 {
        0.5 * (1.0 + Self::erf(x / std::f64::consts::SQRT_2))
    }

    #[inline]
    fn erf(x: f64) -> f64 {
        // constants
        let a1 = 0.254829592;
        let a2 = -0.284496736;
        let a3 = 1.421413741;
        let a4 = -1.453152027;
        let a5 = 1.061405429;
        let p = 0.3275911;
        let sign = if x < 0.0 { -1.0 } else { 1.0 };
        let x = x.abs();
        let t = 1.0 / (1.0 + p * x);
        let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();
        sign * y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engle_granger_proxy_creation() {
        let egp = EngleGrangerProxy::new(50);
        assert!(!egp.is_ready());
        assert_eq!(egp.t_stat, 0.0);
        assert_eq!(egp.p_proxy, 1.0);
    }

    #[test]
    fn test_engle_granger_proxy_warmup() {
        let mut egp = EngleGrangerProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            egp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(egp.is_ready());
    }

    #[test]
    fn test_engle_granger_proxy_range() {
        let mut egp = EngleGrangerProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = egp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0, "P-proxy should be in [0, 1]");
        }
    }

    #[test]
    fn test_engle_granger_proxy_reset() {
        let mut egp = EngleGrangerProxy::new(50);
        for i in 0..60 {
            egp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        egp.reset();
        assert!(!egp.is_ready());
        assert_eq!(egp.t_stat, 0.0);
        assert_eq!(egp.p_proxy, 1.0);
    }
}
