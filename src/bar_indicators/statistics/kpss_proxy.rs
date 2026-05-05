// KPSS proxy: LM statistic proxy for level stationarity using rolling window

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct KpssProxy {
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl KpssProxy {
    pub fn new(window: usize) -> Self {
        let w = window.clamp(50, 1024);
        Self {
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.buf.fill(0.0);
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Returns LM statistic as main value
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        self.buf[self.idx] = c;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            self.value = self.compute_lm_proxy();
        }
        self.value
    }

    fn compute_lm_proxy(&self) -> f64 {
        let n = self.window;
        let mut mean = 0.0;
        for i in 0..n {
            mean += self.buf[i];
        }
        mean /= n as f64;
        // residuals from level (demeaned)
        let mut eps: Vec<f64> = Vec::with_capacity(n);
        for i in 0..n {
            eps.push(self.buf[(self.idx + i) % n] - mean);
        }
        // partial sums S_t = sum_{i=1..t} eps_i
        let mut s = 0.0;
        let mut s2_sum = 0.0;
        for &e in &eps {
            s += e;
            s2_sum += s * s;
        }
        // long-run variance proxy: variance of eps with simple lag-1 Newey-West like adjustment (cheap)
        let mut var = 0.0;
        let mut cov1 = 0.0;
        let e_mean: f64 = eps.iter().sum::<f64>() / n as f64;
        for &e in &eps {
            let d = e - e_mean;
            var += d * d;
        }
        for w in eps.windows(2) {
            cov1 += (w[1] - e_mean) * (w[0] - e_mean);
        }
        var /= n as f64;
        cov1 /= (n - 1) as f64;
        let lrvar = (var + 2.0 * cov1.max(0.0)).max(1e-12);
        (s2_sum / (n as f64 * lrvar)).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kpss_proxy_creation() {
        let kpss = KpssProxy::new(50);
        assert!(!kpss.is_ready());
    }

    #[test]
    fn test_kpss_proxy_warmup() {
        let mut kpss = KpssProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kpss.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(kpss.is_ready());
    }

    #[test]
    fn test_kpss_proxy_non_negative() {
        let mut kpss = KpssProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kpss.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "KPSS should be non-negative");
        }
    }

    #[test]
    fn test_kpss_proxy_reset() {
        let mut kpss = KpssProxy::new(50);
        for i in 0..60 {
            kpss.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        kpss.reset();
        assert!(!kpss.is_ready());
    }
}
