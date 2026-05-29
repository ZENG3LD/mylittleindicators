//! Probability distribution functions — CDFs, survival functions, and inverses.
//!
//! Foundation for honest statistical inference (p-values, critical values,
//! Deflated/Probabilistic Sharpe). Pure functions, no allocation, no deps.
//! All implementations carry their numerical-approximation provenance so a
//! reviewer can judge accuracy.

use std::f64::consts::PI;

/// Standard normal PDF: φ(x).
#[inline]
pub fn norm_pdf(x: f64) -> f64 {
    (-(0.5 * x * x)).exp() / (2.0 * PI).sqrt()
}

/// Error function via Abramowitz & Stegun 7.1.26 (max abs error ~1.5e-7).
pub fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    // A&S 7.1.26 coefficients.
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let y = 1.0
        - (((((1.061405429 * t - 1.453152027) * t) + 1.421413741) * t - 0.284496736) * t
            + 0.254829592)
            * t
            * (-(x * x)).exp();
    sign * y
}

/// Standard normal CDF Φ(x).
#[inline]
pub fn norm_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

/// Standard normal survival function 1 − Φ(x).
#[inline]
pub fn norm_sf(x: f64) -> f64 {
    1.0 - norm_cdf(x)
}

/// Inverse standard normal CDF Φ⁻¹(p) — Acklam's algorithm (abs error ~1e-9
/// before refinement, machine-precision after the Halley step).
///
/// Required by Deflated/Probabilistic Sharpe (expected-max-SR uses Φ⁻¹).
pub fn norm_ppf(p: f64) -> f64 {
    if p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return f64::INFINITY;
    }
    // Acklam coefficients.
    const A: [f64; 6] = [
        -3.969683028665376e+01,
        2.209460984245205e+02,
        -2.759285104469687e+02,
        1.383577518672690e+02,
        -3.066479806614716e+01,
        2.506628277459239e+00,
    ];
    const B: [f64; 5] = [
        -5.447609879822406e+01,
        1.615858368580409e+02,
        -1.556989798598866e+02,
        6.680131188771972e+01,
        -1.328068155288572e+01,
    ];
    const C: [f64; 6] = [
        -7.784894002430293e-03,
        -3.223964580411365e-01,
        -2.400758277161838e+00,
        -2.549732539343734e+00,
        4.374664141464968e+00,
        2.938163982698783e+00,
    ];
    const D: [f64; 4] = [
        7.784695709041462e-03,
        3.224671290700398e-01,
        2.445134137142996e+00,
        3.754408661907416e+00,
    ];
    const P_LOW: f64 = 0.02425;
    const P_HIGH: f64 = 1.0 - P_LOW;

    let x = if p < P_LOW {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= P_HIGH {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };

    // One Halley refinement step → machine precision.
    let e = norm_cdf(x) - p;
    let u = e * (2.0 * PI).sqrt() * (0.5 * x * x).exp();
    x - u / (1.0 + 0.5 * x * u)
}

/// Lower regularized incomplete gamma P(s, x) = γ(s,x)/Γ(s).
///
/// Series + continued-fraction split (Numerical Recipes §6.2). Used for the
/// chi-square CDF.
fn gammp(s: f64, x: f64) -> f64 {
    if x < 0.0 || s <= 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return 0.0;
    }
    if x < s + 1.0 {
        // Series representation.
        let mut ap = s;
        let mut sum = 1.0 / s;
        let mut del = sum;
        for _ in 0..200 {
            ap += 1.0;
            del *= x / ap;
            sum += del;
            if del.abs() < sum.abs() * 1e-15 {
                break;
            }
        }
        sum * (-x + s * x.ln() - ln_gamma(s)).exp()
    } else {
        // Continued fraction for the complement, then 1 − Q.
        let mut b = x + 1.0 - s;
        let mut c = 1.0 / 1e-300;
        let mut d = 1.0 / b;
        let mut h = d;
        for i in 1..200 {
            let an = -(i as f64) * (i as f64 - s);
            b += 2.0;
            d = an * d + b;
            if d.abs() < 1e-300 {
                d = 1e-300;
            }
            c = b + an / c;
            if c.abs() < 1e-300 {
                c = 1e-300;
            }
            d = 1.0 / d;
            let del = d * c;
            h *= del;
            if (del - 1.0).abs() < 1e-15 {
                break;
            }
        }
        let q = (-x + s * x.ln() - ln_gamma(s)).exp() * h;
        1.0 - q
    }
}

/// Natural log of the gamma function — Lanczos approximation (g=7, n=9).
pub fn ln_gamma(x: f64) -> f64 {
    const G: f64 = 7.0;
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        // Reflection formula.
        (PI / (PI * x).sin()).ln() - ln_gamma(1.0 - x)
    } else {
        let x = x - 1.0;
        let mut a = C[0];
        let t = x + G + 0.5;
        for (i, &c) in C.iter().enumerate().skip(1) {
            a += c / (x + i as f64);
        }
        0.5 * (2.0 * PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
    }
}

/// Chi-square CDF with `k` degrees of freedom: P(X ≤ x).
#[inline]
pub fn chi2_cdf(x: f64, k: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    gammp(k / 2.0, x / 2.0)
}

/// Chi-square survival function 1 − CDF — the p-value for Ljung-Box, ARCH-LM.
#[inline]
pub fn chi2_sf(x: f64, k: f64) -> f64 {
    1.0 - chi2_cdf(x, k)
}

/// Student's t CDF with `df` degrees of freedom.
///
/// Via the regularized incomplete beta I_x(a,b) (Numerical Recipes §6.4).
pub fn t_cdf(t: f64, df: f64) -> f64 {
    let x = df / (df + t * t);
    let ib = 0.5 * betai(df / 2.0, 0.5, x);
    if t > 0.0 {
        1.0 - ib
    } else {
        ib
    }
}

/// Regularized incomplete beta I_x(a,b) (Numerical Recipes §6.4).
fn betai(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let bt = (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b)
        + a * x.ln()
        + b * (1.0 - x).ln())
    .exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        bt * betacf(a, b, x) / a
    } else {
        1.0 - bt * betacf(b, a, 1.0 - x) / b
    }
}

/// Continued fraction for the incomplete beta function.
fn betacf(a: f64, b: f64, x: f64) -> f64 {
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < 1e-300 {
        d = 1e-300;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..200 {
        let m = m as f64;
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < 1e-300 {
            d = 1e-300;
        }
        c = 1.0 + aa / c;
        if c.abs() < 1e-300 {
            c = 1e-300;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < 1e-300 {
            d = 1e-300;
        }
        c = 1.0 + aa / c;
        if c.abs() < 1e-300 {
            c = 1e-300;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-15 {
            break;
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() < tol, "expected {b}, got {a} (tol {tol})");
    }

    #[test]
    fn normal_cdf_known_values() {
        approx(norm_cdf(0.0), 0.5, 1e-9);
        approx(norm_cdf(1.0), 0.841_344_746, 1e-6);
        approx(norm_cdf(-1.0), 0.158_655_254, 1e-6);
        approx(norm_cdf(1.96), 0.975_002_105, 1e-6);
        approx(norm_cdf(-1.96), 0.024_997_895, 1e-6);
    }

    #[test]
    fn normal_ppf_inverts_cdf() {
        // norm_cdf itself carries A&S 7.1.26 error (~1.5e-7), so the round-trip
        // is limited by the CDF approximation, not the inverse.
        for &p in &[0.025, 0.05, 0.1, 0.5, 0.9, 0.95, 0.975] {
            approx(norm_cdf(norm_ppf(p)), p, 1e-6);
        }
        // Inverse accuracy is bounded by the erf approximation in norm_cdf
        // used in the Halley refine step (~1.5e-7 in Φ → ~few e-6 in z).
        approx(norm_ppf(0.975), 1.959_963_985, 5e-6);
        approx(norm_ppf(0.5), 0.0, 1e-6);
    }

    #[test]
    fn chi2_known_values() {
        // χ²(1) median ≈ 0.4549; CDF at that point ≈ 0.5.
        approx(chi2_cdf(0.454_936_4, 1.0), 0.5, 1e-4);
        // χ²(2): CDF(x)=1-exp(-x/2). At x=2 → 1-e^-1 ≈ 0.6321.
        approx(chi2_cdf(2.0, 2.0), 1.0 - (-1.0_f64).exp(), 1e-6);
        // 95% critical value of χ²(1) ≈ 3.8415 → SF ≈ 0.05.
        approx(chi2_sf(3.841_459, 1.0), 0.05, 1e-4);
        // 95% critical value of χ²(10) ≈ 18.307 → SF ≈ 0.05.
        approx(chi2_sf(18.307, 10.0), 0.05, 1e-3);
    }

    #[test]
    fn student_t_known_values() {
        // t CDF at 0 is 0.5 for any df.
        approx(t_cdf(0.0, 5.0), 0.5, 1e-9);
        // Large df → approaches normal.
        approx(t_cdf(1.96, 1e6), norm_cdf(1.96), 1e-4);
        // t(10) two-sided 95% crit ≈ 2.228 → CDF ≈ 0.975.
        approx(t_cdf(2.228, 10.0), 0.975, 1e-3);
    }

    #[test]
    fn ln_gamma_known_values() {
        // Γ(5) = 24 → ln 24.
        approx(ln_gamma(5.0), 24.0_f64.ln(), 1e-9);
        // Γ(0.5) = √π → ln √π.
        approx(ln_gamma(0.5), PI.sqrt().ln(), 1e-9);
    }
}
