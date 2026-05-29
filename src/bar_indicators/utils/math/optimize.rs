//! Derivative-free optimization — Nelder-Mead downhill simplex.
//!
//! GARCH/EGARCH/ARIMA parameter estimation maximizes a Gaussian log-likelihood
//! that is smooth but nonlinear in the parameters, with no closed form. The
//! classic estimator is numerical MLE. We use the Nelder-Mead simplex (Nelder &
//! Mead 1965): it needs no gradient (the likelihood gradient is awkward and the
//! finite-difference version is noisy on short rolling windows), is robust for
//! the low dimensions here (3–25 parameters), is fully deterministic, and pulls
//! in no external crate.
//!
//! Box constraints (e.g. ARCH/GARCH weights in (0,1), ω > 0) are handled by the
//! caller: clamp inside the objective and return a large penalty when a
//! parameter is pushed to the boundary, so the simplex is steered back into the
//! feasible region. Helpers [`reflect_into`] / [`penalty`] support that.

/// Result of a Nelder-Mead run.
#[derive(Debug, Clone)]
pub struct NmResult {
    /// Best parameter vector found.
    pub x: Vec<f64>,
    /// Objective value at `x` (the minimized quantity).
    pub fx: f64,
    /// Iterations actually performed.
    pub iters: usize,
    /// True if convergence tolerance was met before `max_iters`.
    pub converged: bool,
}

/// Tuning knobs for [`minimize`]. Defaults follow the standard NM constants.
#[derive(Debug, Clone)]
pub struct NmConfig {
    pub max_iters: usize,
    /// Stop when the spread of simplex vertex values falls below this.
    pub ftol: f64,
    /// Stop when the simplex geometric size falls below this.
    pub xtol: f64,
    /// Initial step used to build the simplex around the start point.
    pub step: f64,
    pub alpha: f64, // reflection
    pub gamma: f64, // expansion
    pub rho: f64,   // contraction
    pub sigma: f64, // shrink
}

impl Default for NmConfig {
    fn default() -> Self {
        Self {
            max_iters: 2000,
            ftol: 1e-10,
            xtol: 1e-10,
            step: 0.1,
            alpha: 1.0,
            gamma: 2.0,
            rho: 0.5,
            sigma: 0.5,
        }
    }
}

/// Minimize `f` starting from `x0` with the Nelder-Mead simplex.
///
/// The initial simplex is `x0` plus one vertex per dimension offset by
/// `cfg.step` (scaled by the coordinate magnitude so it works across scales).
pub fn minimize<F: FnMut(&[f64]) -> f64>(mut f: F, x0: &[f64], cfg: &NmConfig) -> NmResult {
    let n = x0.len();
    if n == 0 {
        return NmResult {
            x: vec![],
            fx: f(&[]),
            iters: 0,
            converged: true,
        };
    }

    // Build the initial simplex: n+1 vertices.
    let mut simplex: Vec<Vec<f64>> = Vec::with_capacity(n + 1);
    simplex.push(x0.to_vec());
    for i in 0..n {
        let mut v = x0.to_vec();
        let h = if v[i].abs() > 1e-8 {
            cfg.step * v[i].abs()
        } else {
            cfg.step.max(1e-4)
        };
        v[i] += h;
        simplex.push(v);
    }

    let mut fvals: Vec<f64> = simplex.iter().map(|v| f(v)).collect();
    let mut converged = false;
    let mut iters = 0;

    while iters < cfg.max_iters {
        iters += 1;

        // Order vertices by objective (ascending: best first).
        let mut order: Vec<usize> = (0..=n).collect();
        order.sort_by(|&a, &b| {
            fvals[a]
                .partial_cmp(&fvals[b])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let best = order[0];
        let worst = order[n];
        let second_worst = order[n - 1];

        // Convergence: function spread AND simplex size both small.
        let fspread = (fvals[worst] - fvals[best]).abs();
        let xsize = simplex_size(&simplex, best);
        if fspread <= cfg.ftol && xsize <= cfg.xtol {
            converged = true;
            break;
        }
        if !fvals[best].is_finite() {
            // Degenerate objective — bail with what we have.
            break;
        }

        // Centroid of all vertices except the worst (the first n in `order`).
        let mut centroid = vec![0.0; n];
        for &idx in order.iter().take(n) {
            for d in 0..n {
                centroid[d] += simplex[idx][d];
            }
        }
        for c in centroid.iter_mut() {
            *c /= n as f64;
        }

        // Reflection: x_r = centroid + alpha (centroid - worst).
        let xr = axpby(&centroid, 1.0 + cfg.alpha, &simplex[worst], -cfg.alpha);
        let fr = f(&xr);

        if fr < fvals[best] {
            // Expansion.
            let xe = axpby(&centroid, 1.0 + cfg.alpha * cfg.gamma, &simplex[worst], -cfg.alpha * cfg.gamma);
            let fe = f(&xe);
            if fe < fr {
                simplex[worst] = xe;
                fvals[worst] = fe;
            } else {
                simplex[worst] = xr;
                fvals[worst] = fr;
            }
        } else if fr < fvals[second_worst] {
            // Accept reflection.
            simplex[worst] = xr;
            fvals[worst] = fr;
        } else {
            // Contraction (outside if reflection improved on worst, else inside).
            let (xc, use_reflected) = if fr < fvals[worst] {
                (axpby(&centroid, 1.0 + cfg.rho * cfg.alpha, &simplex[worst], -cfg.rho * cfg.alpha), true)
            } else {
                (axpby(&centroid, 1.0 - cfg.rho, &simplex[worst], cfg.rho), false)
            };
            let fc = f(&xc);
            let accept = if use_reflected { fc <= fr } else { fc < fvals[worst] };
            if accept {
                simplex[worst] = xc;
                fvals[worst] = fc;
            } else {
                // Shrink all vertices toward the best.
                let bx = simplex[best].clone();
                for i in 0..=n {
                    if i == best {
                        continue;
                    }
                    for d in 0..n {
                        simplex[i][d] = bx[d] + cfg.sigma * (simplex[i][d] - bx[d]);
                    }
                    fvals[i] = f(&simplex[i]);
                }
            }
        }
    }

    // Return the best vertex.
    let mut best = 0;
    for i in 1..=n {
        if fvals[i] < fvals[best] {
            best = i;
        }
    }
    NmResult {
        x: simplex[best].clone(),
        fx: fvals[best],
        iters,
        converged,
    }
}

/// `a*u + b*v` elementwise.
#[inline]
fn axpby(u: &[f64], a: f64, v: &[f64], b: f64) -> Vec<f64> {
    u.iter().zip(v).map(|(&ui, &vi)| a * ui + b * vi).collect()
}

/// Max Euclidean distance from `simplex[anchor]` to any other vertex.
fn simplex_size(simplex: &[Vec<f64>], anchor: usize) -> f64 {
    let a = &simplex[anchor];
    let mut max_d = 0.0_f64;
    for (i, v) in simplex.iter().enumerate() {
        if i == anchor {
            continue;
        }
        let d: f64 = v
            .iter()
            .zip(a)
            .map(|(&vi, &ai)| (vi - ai) * (vi - ai))
            .sum::<f64>()
            .sqrt();
        if d > max_d {
            max_d = d;
        }
    }
    max_d
}

/// Reflect a scalar into `[lo, hi]` (mirror at the bounds), so an unconstrained
/// simplex coordinate maps smoothly to a box-constrained parameter.
#[inline]
pub fn reflect_into(x: f64, lo: f64, hi: f64) -> f64 {
    if hi <= lo {
        return lo;
    }
    let span = hi - lo;
    let mut t = (x - lo) % (2.0 * span);
    if t < 0.0 {
        t += 2.0 * span;
    }
    if t > span {
        t = 2.0 * span - t;
    }
    lo + t
}

/// A large finite penalty for infeasible points — keeps the simplex from
/// chasing `inf`/`NaN` while still pointing it back toward the feasible set.
#[inline]
pub fn penalty(violation: f64) -> f64 {
    1e12 + 1e6 * violation.abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimizes_sphere() {
        // f(x) = Σ (x_i - c_i)²  → minimum at c.
        let c = [1.5, -2.0, 0.75];
        let f = |x: &[f64]| -> f64 {
            x.iter()
                .zip(c.iter())
                .map(|(&xi, &ci)| (xi - ci) * (xi - ci))
                .sum()
        };
        let res = minimize(f, &[0.0, 0.0, 0.0], &NmConfig::default());
        assert!(res.converged, "sphere should converge");
        for (i, &ci) in c.iter().enumerate() {
            assert!(
                (res.x[i] - ci).abs() < 1e-4,
                "x[{i}] = {} should be ≈ {ci}",
                res.x[i]
            );
        }
        assert!(res.fx < 1e-8);
    }

    #[test]
    fn minimizes_rosenbrock() {
        // Classic NM stress test: f = (1-x)² + 100(y-x²)², min 0 at (1,1).
        let f = |v: &[f64]| -> f64 {
            let (x, y) = (v[0], v[1]);
            (1.0 - x).powi(2) + 100.0 * (y - x * x).powi(2)
        };
        let cfg = NmConfig {
            max_iters: 5000,
            step: 0.3,
            ..Default::default()
        };
        let res = minimize(f, &[-1.2, 1.0], &cfg);
        assert!((res.x[0] - 1.0).abs() < 1e-3, "x → 1, got {}", res.x[0]);
        assert!((res.x[1] - 1.0).abs() < 1e-3, "y → 1, got {}", res.x[1]);
    }

    #[test]
    fn reflect_into_stays_in_bounds() {
        for &x in &[-5.0, -0.3, 0.0, 0.5, 1.0, 1.7, 3.4, 10.0] {
            let r = reflect_into(x, 0.0, 1.0);
            assert!((0.0..=1.0).contains(&r), "reflect({x}) = {r} out of [0,1]");
        }
        // Inside the box → identity.
        assert!((reflect_into(0.3, 0.0, 1.0) - 0.3).abs() < 1e-12);
    }

    #[test]
    fn handles_box_constrained_objective() {
        // Minimize (a-0.2)² + (b-0.7)² with a,b constrained to (0,1) via penalty.
        let f = |v: &[f64]| -> f64 {
            let a = v[0];
            let b = v[1];
            if !(0.0..=1.0).contains(&a) || !(0.0..=1.0).contains(&b) {
                return penalty((a - a.clamp(0.0, 1.0)).abs() + (b - b.clamp(0.0, 1.0)).abs());
            }
            (a - 0.2).powi(2) + (b - 0.7).powi(2)
        };
        let res = minimize(f, &[0.5, 0.5], &NmConfig::default());
        assert!((res.x[0] - 0.2).abs() < 1e-3);
        assert!((res.x[1] - 0.7).abs() < 1e-3);
    }
}
