//! Phillips-Perron unit-root test (Z_τ statistic) over a rolling price window.
//!
//! REAL implementation (replaces the prior rolling-z-score-of-price stub that
//! had nothing to do with PP). PP fits the Dickey-Fuller level regression then
//! corrects the t-statistic non-parametrically for serial correlation +
//! heteroskedasticity in the residuals via a Newey-West long-run variance —
//! this is what distinguishes PP from ADF (which augments with lagged diffs
//! instead). Emits the raw Z_τ statistic; a regime filter thresholds it
//! directly (Z_τ < ~−2.86 ⇒ reject unit root at 5%, series stationary).

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::utils::math::linalg::ols;
use crate::bar_indicators::utils::math::timeseries::newey_west_lrv;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct PhillipsPerronProxy {
    window: usize,
    prices: VecDeque<f64>,
    value: f64,
}

impl PhillipsPerronProxy {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(20),
            prices: VecDeque::with_capacity(window.max(20) + 1),
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.prices.clear();
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.prices.len() >= self.window
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Compute the PP Z_τ statistic on the current price window.
    fn compute(&self) -> f64 {
        let n = self.prices.len();
        if n < 20 {
            return 0.0;
        }
        let y: Vec<f64> = self.prices.iter().copied().collect();
        // Regress y_t = α + ρ·y_{t-1}  (t = 1..n-1).
        let rows = n - 1;
        let mut xm = Vec::with_capacity(rows * 2);
        let mut yv = Vec::with_capacity(rows);
        for t in 1..n {
            xm.push(1.0); // intercept
            xm.push(y[t - 1]); // lagged level
            yv.push(y[t]);
        }
        let beta = match ols(&xm, &yv, rows, 2) {
            Some(b) => b,
            None => return 0.0,
        };
        let rho = beta[1];

        // Residuals + their variance, and SE(rho) from the OLS fit.
        let mut resid = Vec::with_capacity(rows);
        let mut sse = 0.0;
        let mut sxx = 0.0;
        let ybar_lag = y[..n - 1].iter().sum::<f64>() / rows as f64;
        for t in 1..n {
            let yhat = beta[0] + rho * y[t - 1];
            let e = y[t] - yhat;
            resid.push(e);
            sse += e * e;
            let d = y[t - 1] - ybar_lag;
            sxx += d * d;
        }
        let dof = (rows - 2) as f64;
        if dof <= 0.0 || sxx <= 0.0 {
            return 0.0;
        }
        let sigma2 = sse / dof; // short-run residual variance (s²)
        let se_rho = (sigma2 / sxx).sqrt();
        if se_rho == 0.0 || !se_rho.is_finite() {
            return 0.0;
        }
        let t_rho = (rho - 1.0) / se_rho;

        // Newey-West long-run variance of residuals (λ²) vs short-run (σ²=s²·dof/rows ≈ mean sq resid).
        let lambda2 = newey_west_lrv(&resid, None);
        let sigma2_hat = sse / rows as f64; // 1/T Σ ê²
        if sigma2_hat <= 0.0 || lambda2 <= 0.0 {
            return t_rho; // fall back to the uncorrected DF t-stat
        }
        let lambda = lambda2.sqrt();
        let sigma = sigma2_hat.sqrt();

        // PP Z_τ correction (Phillips-Perron 1988):
        // Z_τ = (σ/λ)·t_ρ − (λ² − σ²)·(T·SE(ρ)) / (2·λ·s)
        // with s = sqrt(sigma2) the regression standard error.
        let s = sigma2.sqrt();
        let term1 = (sigma / lambda) * t_rho;
        let term2 = (lambda2 - sigma2_hat) * (rows as f64 * se_rho) / (2.0 * lambda * s);
        let z_tau = term1 - term2;
        if z_tau.is_finite() {
            z_tau
        } else {
            t_rho
        }
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        self.prices.push_back(c);
        while self.prices.len() > self.window {
            self.prices.pop_front();
        }
        if self.is_ready() {
            self.value = self.compute();
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic LCG noise for reproducible unit-root fixtures.
    fn lcg_noise(n: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let u = ((s >> 33) as f64) / (1u64 << 31) as f64;
            out.push(u - 1.0);
        }
        out
    }

    #[test]
    fn creation_and_warmup() {
        let mut pp = PhillipsPerronProxy::new(50);
        assert!(!pp.is_ready());
        for &e in &lcg_noise(60, 1) {
            pp.update_bar(100.0 + e, 0.0, 0.0, 100.0 + e, 0.0);
        }
        assert!(pp.is_ready());
        assert!(pp.value().main().is_finite());
    }

    #[test]
    fn stationary_series_gives_negative_stat() {
        // Mean-reverting around 100 → PP should reject unit root (Z_τ negative).
        let mut pp = PhillipsPerronProxy::new(80);
        let noise = lcg_noise(120, 42);
        let mut level = 0.0;
        for &e in &noise {
            level = 0.2 * level + e; // stationary AR(1)
            pp.update_bar(100.0 + level, 0.0, 0.0, 100.0 + level, 0.0);
        }
        assert!(
            pp.value().main() < -2.0,
            "stationary → Z_τ should be clearly negative, got {}",
            pp.value().main()
        );
    }

    #[test]
    fn random_walk_stat_not_strongly_negative() {
        // Random walk → unit root → PP should NOT strongly reject.
        let mut pp = PhillipsPerronProxy::new(80);
        let noise = lcg_noise(120, 7);
        let mut level = 100.0;
        for &e in &noise {
            level += e; // random walk
            pp.update_bar(level, 0.0, 0.0, level, 0.0);
        }
        // 5% PP/DF crit ≈ −2.86; random walk should sit above it.
        assert!(
            pp.value().main() > -2.86,
            "random walk should not reject at 5%, got {}",
            pp.value().main()
        );
    }

    #[test]
    fn reset_clears() {
        let mut pp = PhillipsPerronProxy::new(50);
        for &e in &lcg_noise(60, 3) {
            pp.update_bar(100.0 + e, 0.0, 0.0, 100.0 + e, 0.0);
        }
        pp.reset();
        assert!(!pp.is_ready());
        assert_eq!(pp.value().main(), 0.0);
    }
}
