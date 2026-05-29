//! KPSS level-stationarity LM statistic over a rolling window.
//!
//! The LM construction (demean → partial sums S_t → Σ S_t² / (n²·LRV)) was
//! already correct; the only weakness was a fixed lag-1 Newey-West long-run
//! variance. Now uses the shared data-adaptive Bartlett-kernel estimator
//! (`newey_west_lrv`, automatic bandwidth). Emits the raw LM statistic; a
//! regime filter compares it to `kpss_critical_values(false)` (level): LM >
//! 0.463 ⇒ reject stationarity at 5%.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::utils::math::timeseries::newey_west_lrv;

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
            self.value = self.compute_lm();
        }
        self.value
    }

    fn compute_lm(&self) -> f64 {
        let n = self.window;
        let mean: f64 = self.buf.iter().sum::<f64>() / n as f64;
        // Demeaned residuals (level-stationarity → demean only).
        let eps: Vec<f64> = (0..n).map(|i| self.buf[(self.idx + i) % n] - mean).collect();
        // Partial sums S_t = Σ_{i=1..t} eps_i, accumulate Σ S_t².
        let mut s = 0.0;
        let mut s2_sum = 0.0;
        for &e in &eps {
            s += e;
            s2_sum += s * s;
        }
        // Long-run variance via the shared data-adaptive Bartlett estimator
        // (auto bandwidth), replacing the fixed lag-1 approximation.
        let lrvar = newey_west_lrv(&eps, None).max(1e-12);
        // KPSS LM = (1/n²) Σ S_t² / LRV.
        (s2_sum / (n as f64 * n as f64 * lrvar)).max(0.0)
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

    fn lcg_noise(n: usize, seed: u64) -> Vec<f64> {
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
    fn stationary_low_random_walk_high() {
        // Stationary level → small LM (below 5% crit 0.463).
        let mut k_stat = KpssProxy::new(80);
        let mut lvl = 0.0;
        for &e in &lcg_noise(100, 42) {
            lvl = 0.2 * lvl + e;
            k_stat.update_bar(0.0, 0.0, 0.0, 100.0 + lvl, 0.0);
        }
        // Random walk → large LM (S_t accumulates → reject stationarity).
        let mut k_rw = KpssProxy::new(80);
        let mut p = 100.0;
        for &e in &lcg_noise(100, 7) {
            p += e;
            k_rw.update_bar(0.0, 0.0, 0.0, p, 0.0);
        }
        assert!(
            k_stat.value().main() < k_rw.value().main(),
            "stationary LM {} should be < random-walk LM {}",
            k_stat.value().main(),
            k_rw.value().main()
        );
    }
}
