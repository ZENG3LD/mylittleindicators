//! ARCH-LM test (Engle 1982): regress squared returns ε²_t on their own lags
//! ε²_{t-1..t-L}; the auxiliary-regression R² measures volatility clustering.
//! `value()` returns that R² (a bounded [0,1] regime scalar). The LM statistic
//! T·R² ~ χ²(L) is exposed via [`ArchLmProxy::lm_components`] for the p-value
//! wrapper. The OLS fit now goes through the shared `linalg::ols` (full
//! Gaussian elimination with partial pivoting) instead of a local diagonal
//! normal-equations inverter.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::utils::math::linalg::ols;

#[derive(Clone)]
pub struct ArchLmProxy {
    window: usize,
    lags: usize,
    last_close: Option<f64>,
    returns: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
    /// Effective sample size of the last auxiliary regression (rows used).
    n_obs: usize,
}

impl ArchLmProxy {
    pub fn new(window: usize, lags: usize) -> Self {
        let w = window.clamp(50, 1024);
        let l = lags.clamp(1, 10);
        Self {
            window: w,
            lags: l,
            last_close: None,
            returns: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
            n_obs: 0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.last_close = None;
        self.idx = 0;
        self.filled = false;
        self.returns.fill(0.0);
        self.value = 0.0;
        self.n_obs = 0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// (R², effective sample size, lags) of the last auxiliary regression.
    /// The ARCH-LM statistic is `n_obs · R²`, asymptotically χ²(lags).
    #[inline]
    pub fn lm_components(&self) -> (f64, usize, usize) {
        (self.value, self.n_obs, self.lags)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        if let Some(prev) = self.last_close {
            let r = (c / prev).ln();
            self.returns[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if self.idx == 0 {
                self.filled = true;
            }
        }
        self.last_close = Some(c.max(1e-12));
        if self.filled {
            self.value = self.compute_r2();
        }
        self.value
    }

    fn compute_r2(&mut self) -> f64 {
        // Auxiliary regression  ε²_t = a₀ + Σ_{j=1..L} a_j·ε²_{t-j} + u_t,
        // walking the ring buffer oldest→newest. Build flat row-major X.
        let n = self.window;
        let l = self.lags;
        if n <= l + 2 {
            self.n_obs = 0;
            return 0.0;
        }
        let rows = n - l - 1;
        let n_cols = l + 1; // intercept + L lagged squared returns
        let mut xm = Vec::with_capacity(rows * n_cols);
        let mut yv = Vec::with_capacity(rows);
        let sq = |t: usize| self.returns[t] * self.returns[t];
        for k in 0..rows {
            let t = (self.idx + k + l) % n;
            xm.push(1.0);
            for j in 1..=l {
                let tj = (t + n - j) % n;
                xm.push(sq(tj));
            }
            yv.push(sq(t));
        }
        self.n_obs = rows;
        let beta = match ols(&xm, &yv, rows, n_cols) {
            Some(b) => b,
            None => return 0.0,
        };
        // R² = 1 − RSS/TSS.
        let ymean = yv.iter().sum::<f64>() / rows as f64;
        let mut rss = 0.0;
        let mut tss = 0.0;
        for r in 0..rows {
            let row = &xm[r * n_cols..(r + 1) * n_cols];
            let yhat: f64 = row.iter().zip(beta.iter()).map(|(a, b)| a * b).sum();
            let e = yv[r] - yhat;
            rss += e * e;
            let d = yv[r] - ymean;
            tss += d * d;
        }
        if tss <= 1e-12 {
            return 0.0;
        }
        (1.0 - rss / tss).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arch_lm_proxy_creation() {
        let arch = ArchLmProxy::new(50, 5);
        assert!(!arch.is_ready());
    }

    #[test]
    fn test_arch_lm_proxy_warmup() {
        let mut arch = ArchLmProxy::new(50, 5);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            arch.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(arch.is_ready());
    }

    #[test]
    fn test_arch_lm_proxy_range() {
        let mut arch = ArchLmProxy::new(50, 5);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = arch.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0, "R^2 should be in [0, 1]");
        }
    }

    #[test]
    fn test_arch_lm_proxy_reset() {
        let mut arch = ArchLmProxy::new(50, 5);
        for i in 0..60 {
            arch.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        arch.reset();
        assert!(!arch.is_ready());
    }
}
