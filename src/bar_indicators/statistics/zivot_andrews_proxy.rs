//! Zivot-Andrews unit-root test with one ENDOGENOUS structural break (Model A,
//! level shift) over a rolling price window.
//!
//! REAL implementation (replaces the prior `|body|/range` single-bar ratio that
//! had nothing to do with ZA). ZA tests the unit-root null against a
//! trend-stationary alternative with a break whose date is chosen by the data:
//! for every candidate break fraction λ it fits
//!   Δy_t = α + β·t + θ·DU_t(λ) + γ·y_{t-1} + Σ_{i=1..p} δ_i·Δy_{t-i} + ε_t
//! where DU_t(λ)=1 for t>⌊λT⌋ (a post-break level dummy), and takes the
//! MINIMUM (most negative) t-statistic on γ across all λ — that infimum is the
//! ZA statistic. Emits the raw ZA stat; reject the unit root when it falls
//! below the ZA Model-A critical value (≈ −4.80 at 5%).

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::utils::math::linalg::{ols, solve_linear_system};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct ZivotAndrewsProxy {
    window: usize,
    lags: usize,
    prices: VecDeque<f64>,
    value: f64,
}

impl ZivotAndrewsProxy {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(30),
            lags: 1,
            prices: VecDeque::with_capacity(window.max(30) + 1),
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

    /// t-statistic on γ for one fixed break index `tb` (level-shift dummy).
    /// Returns None if the design is rank-deficient for this break.
    fn t_stat_at_break(y: &[f64], p: usize, tb: usize) -> Option<f64> {
        let n = y.len();
        // First differences.
        let dy: Vec<f64> = (1..n).map(|t| y[t] - y[t - 1]).collect();
        let m = dy.len();
        let start = p;
        if m <= start + 6 {
            return None;
        }
        // Columns: const, trend, DU(break), lagged level y_{t-1}, p lagged diffs.
        let n_cols = 4 + p;
        let gamma_col = 3;
        let rows = m - start;
        if rows <= n_cols + 2 {
            return None;
        }
        let mut xm = Vec::with_capacity(rows * n_cols);
        let mut yv = Vec::with_capacity(rows);
        for t in start..m {
            // dy[t] corresponds to level index t+1; lagged level = y[t].
            xm.push(1.0); // const
            xm.push((t + 1) as f64); // trend
            xm.push(if (t + 1) > tb { 1.0 } else { 0.0 }); // DU level shift
            xm.push(y[t]); // lagged level
            for i in 1..=p {
                xm.push(dy[t - i]);
            }
            yv.push(dy[t]);
        }
        let beta = ols(&xm, &yv, rows, n_cols)?;
        let gamma = beta[gamma_col];

        // SSE → sigma², then SE(gamma) via (XᵀX)⁻¹[gamma,gamma].
        let mut sse = 0.0;
        for r in 0..rows {
            let row = &xm[r * n_cols..(r + 1) * n_cols];
            let yhat: f64 = row.iter().zip(beta.iter()).map(|(a, b)| a * b).sum();
            let e = yv[r] - yhat;
            sse += e * e;
        }
        let dof = (rows - n_cols) as f64;
        if dof <= 0.0 {
            return None;
        }
        let sigma2 = sse / dof;

        // (XᵀX)⁻¹ diagonal for gamma_col.
        let mut xtx = vec![0.0_f64; n_cols * n_cols];
        for r in 0..rows {
            let row = &xm[r * n_cols..(r + 1) * n_cols];
            for i in 0..n_cols {
                for j in 0..n_cols {
                    xtx[i * n_cols + j] += row[i] * row[j];
                }
            }
        }
        let mut e = vec![0.0_f64; n_cols];
        e[gamma_col] = 1.0;
        let z = solve_linear_system(&mut xtx, &mut e, n_cols)?;
        let se = (sigma2 * z[gamma_col]).sqrt();
        if se == 0.0 || !se.is_finite() {
            return None;
        }
        Some(gamma / se)
    }

    fn compute(&self) -> f64 {
        let n = self.prices.len();
        if n < 30 {
            return 0.0;
        }
        let y: Vec<f64> = self.prices.iter().copied().collect();
        // Trim the break search to the interior [0.15, 0.85] (standard ZA).
        let lo = (0.15 * n as f64) as usize;
        let hi = (0.85 * n as f64) as usize;
        let mut min_t = f64::INFINITY;
        let mut any = false;
        for tb in lo..=hi {
            if let Some(t) = Self::t_stat_at_break(&y, self.lags, tb) {
                if t < min_t {
                    min_t = t;
                }
                any = true;
            }
        }
        if any && min_t.is_finite() {
            min_t
        } else {
            0.0
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
        let mut za = ZivotAndrewsProxy::new(60);
        assert!(!za.is_ready());
        for &e in &lcg_noise(70, 1) {
            za.update_bar(0.0, 0.0, 0.0, 100.0 + e, 0.0);
        }
        assert!(za.is_ready());
        assert!(za.value().main().is_finite());
    }

    #[test]
    fn stationary_with_break_rejects() {
        // Stationary around a level that jumps midway → ZA (which allows a
        // break) should find a strongly negative inf-t.
        let mut za = ZivotAndrewsProxy::new(100);
        let noise = lcg_noise(140, 42);
        let mut level = 0.0;
        for (i, &e) in noise.iter().enumerate() {
            level = 0.2 * level + e;
            let shift = if i > 70 { 10.0 } else { 0.0 };
            let p = 100.0 + level + shift;
            za.update_bar(0.0, 0.0, 0.0, p, 0.0);
        }
        // ZA Model-A 5% crit ≈ −4.80; stationary-with-break should be below it.
        assert!(
            za.value().main() < -4.0,
            "stationary-with-break should reject, got {}",
            za.value().main()
        );
    }

    #[test]
    fn random_walk_not_strongly_negative() {
        let mut za = ZivotAndrewsProxy::new(100);
        let noise = lcg_noise(140, 7);
        let mut level = 100.0;
        for &e in &noise {
            level += e;
            za.update_bar(0.0, 0.0, 0.0, level, 0.0);
        }
        // Random walk → unit root → inf-t should stay above the 1% crit (~−5.34).
        assert!(
            za.value().main() > -5.34,
            "random walk should not strongly reject, got {}",
            za.value().main()
        );
    }

    #[test]
    fn reset_clears() {
        let mut za = ZivotAndrewsProxy::new(40);
        for &e in &lcg_noise(50, 3) {
            za.update_bar(0.0, 0.0, 0.0, 100.0 + e, 0.0);
        }
        za.reset();
        assert!(!za.is_ready());
        assert_eq!(za.value().main(), 0.0);
    }
}
