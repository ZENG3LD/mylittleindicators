//! Dense linear algebra for least-squares / regression.
//!
//! The real solver that replaces the diagonal-only-OLS shortcut found in
//! arima/var/polynomial (which ignored off-diagonal X'X terms and so produced
//! wrong coefficients whenever regressors correlate — i.e. always). Small
//! systems (a few to ~16 unknowns); allocation is negligible vs correctness.

/// Solve the normal equations `(XᵀX) β = Xᵀy` for ordinary least squares.
///
/// `x` is row-major `n_rows × n_cols` design matrix, `y` length `n_rows`.
/// Returns `β` length `n_cols`, or `None` if the system is singular.
/// Uses full Gaussian elimination with partial pivoting on XᵀX — NOT the
/// diagonal approximation. Correct for correlated regressors (AR lags,
/// Vandermonde columns, etc.).
pub fn ols(x: &[f64], y: &[f64], n_rows: usize, n_cols: usize) -> Option<Vec<f64>> {
    if n_rows == 0 || n_cols == 0 || x.len() != n_rows * n_cols || y.len() != n_rows {
        return None;
    }
    // Build XᵀX (n_cols × n_cols) and Xᵀy (n_cols).
    let mut xtx = vec![0.0_f64; n_cols * n_cols];
    let mut xty = vec![0.0_f64; n_cols];
    for r in 0..n_rows {
        let row = &x[r * n_cols..(r + 1) * n_cols];
        let yr = y[r];
        for i in 0..n_cols {
            xty[i] += row[i] * yr;
            for j in 0..n_cols {
                xtx[i * n_cols + j] += row[i] * row[j];
            }
        }
    }
    solve_linear_system(&mut xtx, &mut xty, n_cols)
}

/// Solve a dense linear system `A x = b` in place via Gaussian elimination with
/// partial pivoting. `a` is row-major `n × n` (consumed), `b` length `n`
/// (consumed). Returns `x` length `n`, or `None` if singular.
pub fn solve_linear_system(a: &mut [f64], b: &mut [f64], n: usize) -> Option<Vec<f64>> {
    if a.len() != n * n || b.len() != n {
        return None;
    }
    for col in 0..n {
        // Partial pivot: largest |a[row][col]| in rows col..n.
        let mut pivot_row = col;
        let mut pivot_val = a[col * n + col].abs();
        for r in (col + 1)..n {
            let v = a[r * n + col].abs();
            if v > pivot_val {
                pivot_val = v;
                pivot_row = r;
            }
        }
        if pivot_val < 1e-300 {
            return None; // singular
        }
        if pivot_row != col {
            for c in 0..n {
                a.swap(col * n + c, pivot_row * n + c);
            }
            b.swap(col, pivot_row);
        }
        // Eliminate below.
        let diag = a[col * n + col];
        for r in (col + 1)..n {
            let factor = a[r * n + col] / diag;
            if factor != 0.0 {
                for c in col..n {
                    a[r * n + c] -= factor * a[col * n + c];
                }
                b[r] -= factor * b[col];
            }
        }
    }
    // Back-substitution.
    let mut xsol = vec![0.0_f64; n];
    for col in (0..n).rev() {
        let mut s = b[col];
        for c in (col + 1)..n {
            s -= a[col * n + c] * xsol[c];
        }
        xsol[col] = s / a[col * n + col];
    }
    Some(xsol)
}

/// Determinant of a row-major `n × n` matrix via LU with partial pivoting.
/// Returns the product of the pivots times the permutation sign. Robust for
/// the small dense matrices here (e.g. a VAR residual covariance Σ), unlike a
/// diagonal-only product which ignores off-diagonal correlation. `None` on a
/// dimension mismatch; a singular matrix yields `0.0`.
pub fn determinant(a: &[f64], n: usize) -> Option<f64> {
    if a.len() != n * n {
        return None;
    }
    if n == 0 {
        return Some(1.0);
    }
    let mut m = a.to_vec();
    let mut det = 1.0_f64;
    for col in 0..n {
        // Partial pivot.
        let mut pivot_row = col;
        let mut pivot_val = m[col * n + col].abs();
        for r in (col + 1)..n {
            let v = m[r * n + col].abs();
            if v > pivot_val {
                pivot_val = v;
                pivot_row = r;
            }
        }
        if pivot_val < 1e-300 {
            return Some(0.0); // singular
        }
        if pivot_row != col {
            for c in 0..n {
                m.swap(col * n + c, pivot_row * n + c);
            }
            det = -det; // row swap flips sign
        }
        let diag = m[col * n + col];
        det *= diag;
        for r in (col + 1)..n {
            let factor = m[r * n + col] / diag;
            if factor != 0.0 {
                for c in col..n {
                    m[r * n + c] -= factor * m[col * n + c];
                }
            }
        }
    }
    Some(det)
}

/// Cholesky decomposition of a symmetric positive-definite matrix `a`
/// (row-major `n × n`). Returns lower-triangular `L` (row-major) with
/// `A = L Lᵀ`, or `None` if not positive-definite. Used where SPD is
/// guaranteed (covariance matrices) — faster + more stable than Gaussian.
pub fn cholesky(a: &[f64], n: usize) -> Option<Vec<f64>> {
    if a.len() != n * n {
        return None;
    }
    let mut l = vec![0.0_f64; n * n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = a[i * n + j];
            for k in 0..j {
                sum -= l[i * n + k] * l[j * n + k];
            }
            if i == j {
                if sum <= 0.0 {
                    return None; // not positive-definite
                }
                l[i * n + j] = sum.sqrt();
            } else {
                l[i * n + j] = sum / l[j * n + j];
            }
        }
    }
    Some(l)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() < tol, "expected {b}, got {a}");
    }

    #[test]
    fn ols_recovers_known_line() {
        // y = 2 + 3x exactly; design has intercept col + x col.
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mut x = Vec::new();
        let mut y = Vec::new();
        for &xi in &xs {
            x.push(1.0);
            x.push(xi);
            y.push(2.0 + 3.0 * xi);
        }
        let beta = ols(&x, &y, xs.len(), 2).unwrap();
        approx(beta[0], 2.0, 1e-9);
        approx(beta[1], 3.0, 1e-9);
    }

    #[test]
    fn ols_correlated_regressors_not_diagonal() {
        // Two highly-correlated columns: x2 = x1 + tiny. Diagonal-only OLS
        // would give wrong coeffs; full solve must recover the true plane.
        // y = 1*x1 + 0*x2 (+ intercept 0). Build so solution is well-defined.
        let x1 = [1.0, 2.0, 3.0, 4.0, 6.0];
        let x2 = [1.1, 2.0, 2.9, 4.2, 5.7];
        let mut x = Vec::new();
        let mut y = Vec::new();
        for i in 0..x1.len() {
            x.push(x1[i]);
            x.push(x2[i]);
            // true: y = 2*x1 - 1*x2
            y.push(2.0 * x1[i] - 1.0 * x2[i]);
        }
        let beta = ols(&x, &y, x1.len(), 2).unwrap();
        approx(beta[0], 2.0, 1e-6);
        approx(beta[1], -1.0, 1e-6);
    }

    #[test]
    fn solve_linear_system_basic() {
        // 2x + y = 5 ; x + 3y = 10 → x=1, y=3.
        let mut a = vec![2.0, 1.0, 1.0, 3.0];
        let mut b = vec![5.0, 10.0];
        let sol = solve_linear_system(&mut a, &mut b, 2).unwrap();
        approx(sol[0], 1.0, 1e-9);
        approx(sol[1], 3.0, 1e-9);
    }

    #[test]
    fn singular_returns_none() {
        let mut a = vec![1.0, 2.0, 2.0, 4.0]; // rank 1
        let mut b = vec![1.0, 2.0];
        assert!(solve_linear_system(&mut a, &mut b, 2).is_none());
    }

    #[test]
    fn cholesky_spd() {
        // A = [[4,2],[2,3]] SPD → L=[[2,0],[1,√2]].
        let a = vec![4.0, 2.0, 2.0, 3.0];
        let l = cholesky(&a, 2).unwrap();
        approx(l[0], 2.0, 1e-9);
        approx(l[2], 1.0, 1e-9);
        approx(l[3], 2.0_f64.sqrt(), 1e-9);
        // reconstruct A = L Lᵀ
        approx(l[0] * l[0], 4.0, 1e-9);
        approx(l[2] * l[0], 2.0, 1e-9);
        approx(l[2] * l[2] + l[3] * l[3], 3.0, 1e-9);
    }

    #[test]
    fn determinant_known() {
        // 2×2: det[[4,2],[2,3]] = 4·3 − 2·2 = 8.
        approx(determinant(&[4.0, 2.0, 2.0, 3.0], 2).unwrap(), 8.0, 1e-9);
        // 3×3 with a row swap needed (zero leading pivot): det = 1·(forced).
        // [[0,1,2],[3,4,5],[6,7,9]] → det = 0·(..) expansion = -3. Verify.
        let m = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 9.0];
        // cofactor: 0·(4·9−5·7) − 1·(3·9−5·6) + 2·(3·7−4·6) = -(27-30)+2(21-24)
        //         = -(-3) + 2(-3) = 3 - 6 = -3.
        approx(determinant(&m, 3).unwrap(), -3.0, 1e-9);
        // Identity → 1; singular (duplicate row) → 0.
        approx(determinant(&[1.0, 0.0, 0.0, 1.0], 2).unwrap(), 1.0, 1e-12);
        approx(determinant(&[2.0, 4.0, 1.0, 2.0], 2).unwrap(), 0.0, 1e-12);
    }
}
