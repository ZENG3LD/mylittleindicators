//! Time-series statistical primitives shared by the unit-root / stationarity /
//! cointegration indicators: Newey-West long-run variance, the augmented
//! Dickey-Fuller regression, and MacKinnon/Kwiatkowski critical values.
//!
//! These replace the fake/partial implementations (PhillipsPerron-as-zscore,
//! DF-without-augmentation, normal-CDF-on-DF p-values). The indicators that
//! STAY in BarIndicatorId call these so they emit a REAL test statistic; the
//! p-value/critical-value lookup also lives here for callers that want it.

use super::linalg::ols;

/// Newey-West / Bartlett-kernel long-run variance estimate of a (mean-removed
/// internally) series. `bandwidth` = number of autocovariance lags; if `None`,
/// uses the Newey-West automatic rule ⌊4·(n/100)^(2/9)⌋.
///
/// LRV = γ₀ + 2·Σ_{l=1..L} w_l·γ_l ,  w_l = 1 − l/(L+1)  (Bartlett weights).
pub fn newey_west_lrv(series: &[f64], bandwidth: Option<usize>) -> f64 {
    let n = series.len();
    if n < 2 {
        return 0.0;
    }
    let mean = series.iter().sum::<f64>() / n as f64;
    let dev: Vec<f64> = series.iter().map(|v| v - mean).collect();

    let l = bandwidth.unwrap_or_else(|| {
        let b = 4.0 * (n as f64 / 100.0).powf(2.0 / 9.0);
        (b.floor() as usize).max(1).min(n - 1)
    });

    let gamma0 = dev.iter().map(|d| d * d).sum::<f64>() / n as f64;
    let mut lrv = gamma0;
    for lag in 1..=l.min(n - 1) {
        let mut g = 0.0;
        for t in lag..n {
            g += dev[t] * dev[t - lag];
        }
        g /= n as f64;
        let w = 1.0 - lag as f64 / (l as f64 + 1.0);
        lrv += 2.0 * w * g;
    }
    lrv.max(0.0)
}

/// Deterministic terms allowed in the ADF regression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdfTrend {
    /// No constant, no trend (`nc`).
    None,
    /// Constant only (`c`).
    Constant,
    /// Constant + linear trend (`ct`).
    ConstantTrend,
}

/// Result of an augmented Dickey-Fuller regression.
#[derive(Debug, Clone, Copy)]
pub struct AdfResult {
    /// The ADF t-statistic on the lagged-level coefficient (the test stat).
    pub t_stat: f64,
    /// Estimated ρ−1 (the lagged-level coefficient γ).
    pub gamma: f64,
    /// Number of augmentation lags actually used.
    pub lags: usize,
    /// Effective sample size after differencing + lag truncation.
    pub n_eff: usize,
}

/// Augmented Dickey-Fuller regression.
///
/// Δy_t = α + βt + γ·y_{t-1} + Σ_{i=1..p} δ_i·Δy_{t-i} + ε_t
///
/// Returns the t-statistic on γ (the ADF statistic). REAL augmentation,
/// REAL OLS (via the proper solver), with chosen deterministic terms.
/// `lags = None` selects p via the Schwert rule ⌊12·(n/100)^(1/4)⌋ capped at n/3.
pub fn adf_regression(y: &[f64], trend: AdfTrend, lags: Option<usize>) -> Option<AdfResult> {
    let n = y.len();
    if n < 8 {
        return None;
    }
    // First differences.
    let dy: Vec<f64> = (1..n).map(|t| y[t] - y[t - 1]).collect();
    let m = dy.len(); // = n-1

    let p = lags.unwrap_or_else(|| {
        let s = 12.0 * (n as f64 / 100.0).powf(0.25);
        (s.floor() as usize).min(m / 3)
    });
    if m <= p + 2 {
        return None;
    }

    // Build regression rows for t = p .. m-1 (index into dy), regressing
    // dy[t] on: [det terms] + y_{t-1 in level} + lagged Δy.
    // dy[t] corresponds to level index t+1; y_{t-1} level = y[t].
    let det = match trend {
        AdfTrend::None => 0,
        AdfTrend::Constant => 1,
        AdfTrend::ConstantTrend => 2,
    };
    let n_cols = det + 1 + p; // det + gamma + p lags
    let start = p;
    let rows = m - start;
    if rows <= n_cols {
        return None;
    }

    let mut xm = Vec::with_capacity(rows * n_cols);
    let mut yv = Vec::with_capacity(rows);
    let gamma_col = det; // column index of the lagged-level term
    for t in start..m {
        // deterministic
        if det >= 1 {
            xm.push(1.0);
        }
        if det == 2 {
            xm.push(t as f64);
        }
        // lagged level y_{t-1}: dy[t] = y[t+1]-y[t]; lagged level is y[t]
        xm.push(y[t]);
        // lagged differences Δy_{t-i}
        for i in 1..=p {
            xm.push(dy[t - i]);
        }
        yv.push(dy[t]);
    }

    let beta = ols(&xm, &yv, rows, n_cols)?;
    let gamma = beta[gamma_col];

    // Residuals → SE(gamma). Need (XᵀX)^{-1}[gamma,gamma].
    // Recompute residual variance, then SE via the diagonal of the inverse.
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

    // (XᵀX)^{-1} diagonal element for gamma_col: solve XᵀX z = e_gamma.
    let xtx_inv_diag = xtx_inverse_diag(&xm, rows, n_cols, gamma_col)?;
    let se = (sigma2 * xtx_inv_diag).sqrt();
    if !se.is_finite() || se == 0.0 {
        return None;
    }

    Some(AdfResult {
        t_stat: gamma / se,
        gamma,
        lags: p,
        n_eff: rows,
    })
}

/// Diagonal element `[col,col]` of (XᵀX)⁻¹, by solving (XᵀX)·z = e_col and
/// reading z[col]. Avoids forming the full inverse.
fn xtx_inverse_diag(x: &[f64], n_rows: usize, n_cols: usize, col: usize) -> Option<f64> {
    let mut xtx = vec![0.0_f64; n_cols * n_cols];
    for r in 0..n_rows {
        let row = &x[r * n_cols..(r + 1) * n_cols];
        for i in 0..n_cols {
            for j in 0..n_cols {
                xtx[i * n_cols + j] += row[i] * row[j];
            }
        }
    }
    let mut e = vec![0.0_f64; n_cols];
    e[col] = 1.0;
    let z = super::linalg::solve_linear_system(&mut xtx, &mut e, n_cols)?;
    Some(z[col])
}

/// MacKinnon (1996/2010) asymptotic critical values for the ADF t-statistic,
/// by deterministic specification and significance level. These are the
/// large-sample limits (the finite-sample response-surface corrections are
/// small for n≳100). Returns (1%, 5%, 10%) critical values — all negative;
/// reject the unit-root null when t_stat < crit.
pub fn adf_critical_values(trend: AdfTrend) -> (f64, f64, f64) {
    match trend {
        AdfTrend::None => (-2.5658, -1.9393, -1.6156),
        AdfTrend::Constant => (-3.4336, -2.8621, -2.5671),
        AdfTrend::ConstantTrend => (-3.9638, -3.4126, -3.1279),
    }
}

/// KPSS asymptotic critical values (Kwiatkowski et al. 1992, Table 1).
/// Returns (10%, 5%, 2.5%, 1%). Reject the stationarity null when the LM
/// statistic EXCEEDS the critical value.
pub fn kpss_critical_values(trend: bool) -> (f64, f64, f64, f64) {
    if trend {
        // Trend-stationary (η_τ).
        (0.119, 0.146, 0.176, 0.216)
    } else {
        // Level-stationary (η_μ).
        (0.347, 0.463, 0.574, 0.739)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() < tol, "expected {b}, got {a}");
    }

    /// Deterministic LCG → uniform(-0.5,0.5)-ish increments. Long, non-periodic
    /// enough for a statistically meaningful unit-root test fixture.
    fn lcg_noise(n: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let u = ((s >> 33) as f64) / (1u64 << 31) as f64; // ~[0,2)
            out.push(u - 1.0); // ~[-1,1)
        }
        out
    }

    #[test]
    fn newey_west_iid_approximates_variance() {
        // White-noise-ish: LRV ≈ sample variance for near-zero autocorrelation.
        let s = [1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
        let lrv = newey_west_lrv(&s, Some(0));
        // bandwidth 0 → just γ₀ = variance of ±1 = 1.0
        approx(lrv, 1.0, 1e-9);
    }

    #[test]
    fn adf_rejects_on_stationary_series() {
        // Strongly mean-reverting AR(1): y_t = 0.2·y_{t-1} + noise → stationary.
        let noise = lcg_noise(300, 12345);
        let mut y = vec![0.0_f64];
        for &e in &noise {
            let prev = *y.last().unwrap();
            y.push(0.2 * prev + e);
        }
        let res = adf_regression(&y, AdfTrend::Constant, None).unwrap();
        let (c1, _c5, _c10) = adf_critical_values(AdfTrend::Constant);
        // Strongly stationary series → ADF stat well below even the 1% crit.
        assert!(
            res.t_stat < c1,
            "expected stationary reject: t_stat {} should be < {}",
            res.t_stat,
            c1
        );
    }

    #[test]
    fn adf_does_not_reject_random_walk() {
        // Pure random walk y_t = y_{t-1} + noise → unit root, should NOT reject.
        let noise = lcg_noise(300, 67890);
        let mut y = vec![0.0_f64];
        for &e in &noise {
            let prev = *y.last().unwrap();
            y.push(prev + e);
        }
        let res = adf_regression(&y, AdfTrend::Constant, None).unwrap();
        let (_c1, c5, _c10) = adf_critical_values(AdfTrend::Constant);
        // Random walk → fail to reject at 5% (stat above the crit).
        assert!(
            res.t_stat > c5,
            "random walk should not reject at 5%: t_stat {} vs {}",
            res.t_stat,
            c5
        );
    }
}
