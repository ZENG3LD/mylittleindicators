// KPSS Z-proxy: standardize KPSS-proxy statistic to z-score over rolling window

use crate::bar_indicators::statistics::kpss_proxy::KpssProxy;

#[derive(Clone)]
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

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let s = self.inner.update_bar(o, h, l, c, v);
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

    pub fn value(&self) -> crate::bar_indicators::indicator_value::IndicatorValue {
        crate::bar_indicators::indicator_value::IndicatorValue::Single(self.value)
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
            kpssz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(kpssz.is_ready());
    }

    #[test]
    fn test_kpss_z_proxy_values() {
        let mut kpssz = KpssZProxy::new(50, 30);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kpssz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Z-score should be finite");
        }
    }

    #[test]
    fn test_kpss_z_proxy_reset() {
        let mut kpssz = KpssZProxy::new(50, 30);
        for i in 0..90 {
            kpssz.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        kpssz.reset();
        assert!(!kpssz.is_ready());
        assert_eq!(kpssz.value, 0.0);
    }
}
