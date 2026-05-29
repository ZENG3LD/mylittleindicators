//! Engle–Granger residual unit-root statistic.
//!
//! Wraps [`CointegrationProxy`] (an AR(1) Dickey-Fuller t-stat on `close − SMA`
//! residuals) and emits the raw DF t-statistic. The prior version mapped that
//! t-stat through a *normal* CDF (`p = 1 − Φ(t)`) — statistically wrong: the
//! Engle-Granger residual statistic does NOT follow a standard normal (nor the
//! plain Dickey-Fuller distribution — it has its own tables that depend on the
//! number of regressors and sample size). A correct p-value requires
//! MacKinnon's EG response surfaces, which are out of scope here; that lives in
//! the statistical-validation layer (mlsv). We therefore emit the raw test
//! statistic and let a regime filter threshold it directly (more negative ⇒
//! stronger residual mean reversion). `p_value`/`normal_cdf`/local `erf`
//! (a duplicate of `distributions::erf`) are removed.
//!
//! NOTE: despite the name this is a SINGLE-stream residual test against an SMA,
//! not a true two-series cointegration test — that belongs in `events::`
//! (deferred). Name retained for backward-compat with existing catalog ids.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::statistics::cointegration_proxy::CointegrationProxy;

#[derive(Clone)]
pub struct EngleGrangerProxy {
    inner: CointegrationProxy,
    pub t_stat: f64,
}

impl EngleGrangerProxy {
    pub fn new(window: usize) -> Self {
        Self {
            inner: CointegrationProxy::new(window),
            t_stat: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.t_stat = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.t_stat)
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let (_phi, t) = self.inner.update_bar(open, high, low, close, volume);
        self.t_stat = t;
        self.t_stat
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
    fn test_engle_granger_proxy_emits_finite_stat() {
        let mut egp = EngleGrangerProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = egp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "EG residual t-stat should be finite");
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
    }
}
