//! Iterative model-fit formulas as FIXED-ITERATION SEQUENTIAL kernels (one thread, refit on every bar like the
//! CPU). The CPU algorithms are ported 1:1 in f32 (same Nelder-Mead variant, same iteration caps); f32 cannot
//! reproduce the f64 simplex path bit for bit, so results agree only to f32 tolerance on well-conditioned
//! data (UNTESTED on GPU).
//!
//! - `1337` Arima (`period` = p, `fast` = d, `slow` = q): differencing, OLS AR fit (normal equations + pivoted
//!   Gaussian elimination, pivot threshold 1e-30 instead of 1e-300), constant, MA fit by Nelder-Mead (<= 8 params,
//!   max 1500 iterations, step 0.3, ftol = xtol = 1e-10) on the conditional sum of squares, fitted residuals and
//!   the one-step forecast. The forecast keeps its previous value while the model cannot be formed.

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::gpu::CubeParams;
use super::CubeFormula;

// scratch layout (f32 indices)
pub const FIT_SCR: usize = 2048;

#[cube]
fn reflect_into(x: f32, lo: f32, hi: f32) -> f32 {
    let span = hi - lo;
    let m = 2.0f32 * span;
    let y = x - lo;
    let mut t = y - (y / m).floor() * m;
    if t < 0.0f32 {
        t = t + m;
    }
    if t > span {
        t = m - t;
    }
    lo + t
}

/// Conditional sum of squares for MA parameters at `scr[th_off..th_off + q]`.
#[cube]
fn arima_css(scr: &mut [f32], th_off: usize, dn: usize, p: usize, arn: usize, q: usize, start: usize, cst: f32) -> f32 {
    // differenced series at 0, eps at 1024, ar at 1536, reflected theta at 1952
    for i in 0..q {
        scr[1952usize + i] = reflect_into(scr[th_off + i], -0.999f32, 0.999f32);
    }
    for t in 0..dn {
        scr[1024usize + t] = 0.0f32;
    }
    let mut sse = 0.0f32;
    for t in start..dn {
        let mut w = scr[t] - cst;
        for j in 0..arn {
            w = w - scr[1536usize + j] * scr[t - 1usize - j];
        }
        let mut e = w;
        for i in 0..q {
            e = e - scr[1952usize + i] * scr[1024usize + t - 1usize - i];
        }
        scr[1024usize + t] = e;
        sse = sse + e * e;
    }
    let mut res = sse;
    if sse != sse || sse > 3.0e38f32 {
        res = 3.4028234e38f32;
    }
    res
}

/// Nelder-Mead on [`arima_css`]; simplex at 1560 ((n+1) x n), fvals 1632, centroid 1641, xr 1649, xt 1657,
/// order 1665. Result (best vertex) copied to `scr[1552..1552 + n]`.
#[cube]
fn arima_nm(scr: &mut [f32], dn: usize, p: usize, arn: usize, n: usize, start: usize, cst: f32) {
    let max_iters = 1500u32;
    // initial simplex around x0 = 0.1, step 0.3
    for d in 0..n {
        scr[1560usize + d] = 0.1f32;
    }
    for i in 0..n {
        for d in 0..n {
            scr[1560usize + (i + 1usize) * n + d] = 0.1f32;
        }
        scr[1560usize + (i + 1usize) * n + i] = 0.1f32 + 0.3f32 * 0.1f32;
    }
    for i in 0..(n + 1usize) {
        scr[1632usize + i] = arima_css(scr, 1560usize + i * n, dn, p, arn, n, start, cst);
    }
    let mut it = 0u32;
    let mut go = true;
    while go && it < max_iters {
        it = it + 1u32;
        // stable insertion sort of the order vector by fvals
        for i in 0..(n + 1usize) {
            scr[1665usize + i] = i as f32;
        }
        for i in 1..(n + 1usize) {
            let key = scr[1665usize + i];
            let kf = scr[1632usize + (key as usize)];
            let mut j = i;
            let mut moving = true;
            while moving && j > 0usize {
                let prev = scr[1665usize + j - 1usize];
                let pf = scr[1632usize + (prev as usize)];
                if pf > kf {
                    scr[1665usize + j] = prev;
                    j = j - 1usize;
                } else {
                    moving = false;
                }
            }
            scr[1665usize + j] = key;
        }
        let best = scr[1665usize] as usize;
        let worst = scr[1665usize + n] as usize;
        let second = scr[1665usize + n - 1usize] as usize;
        let fspread = (scr[1632usize + worst] - scr[1632usize + best]).abs();
        let mut xsize = 0.0f32;
        for i in 0..(n + 1usize) {
            if i != best {
                let mut s = 0.0f32;
                for d in 0..n {
                    let df = scr[1560usize + i * n + d] - scr[1560usize + best * n + d];
                    s = s + df * df;
                }
                xsize = xsize.max(s.sqrt());
            }
        }
        if fspread <= 1.0e-10f32 && xsize <= 1.0e-10f32 {
            go = false;
        }
        if go {
            for d in 0..n {
                scr[1641usize + d] = 0.0f32;
            }
            for k in 0..n {
                let idx = scr[1665usize + k] as usize;
                for d in 0..n {
                    scr[1641usize + d] = scr[1641usize + d] + scr[1560usize + idx * n + d];
                }
            }
            for d in 0..n {
                scr[1641usize + d] = scr[1641usize + d] / (n as f32);
            }
            // reflection xr = 2*c - w
            for d in 0..n {
                scr[1649usize + d] = 2.0f32 * scr[1641usize + d] - scr[1560usize + worst * n + d];
            }
            let fr = arima_css(scr, 1649usize, dn, p, arn, n, start, cst);
            let fbest = scr[1632usize + best];
            let fsec = scr[1632usize + second];
            let fworst = scr[1632usize + worst];
            if fr < fbest {
                // expansion xe = 3*c - 2*w
                for d in 0..n {
                    scr[1657usize + d] = 3.0f32 * scr[1641usize + d] - 2.0f32 * scr[1560usize + worst * n + d];
                }
                let fe = arima_css(scr, 1657usize, dn, p, arn, n, start, cst);
                if fe < fr {
                    for d in 0..n {
                        scr[1560usize + worst * n + d] = scr[1657usize + d];
                    }
                    scr[1632usize + worst] = fe;
                } else {
                    for d in 0..n {
                        scr[1560usize + worst * n + d] = scr[1649usize + d];
                    }
                    scr[1632usize + worst] = fr;
                }
            } else if fr < fsec {
                for d in 0..n {
                    scr[1560usize + worst * n + d] = scr[1649usize + d];
                }
                scr[1632usize + worst] = fr;
            } else {
                let mut use_ref = false;
                if fr < fworst {
                    use_ref = true;
                    // outside contraction xc = 1.5*c - 0.5*w ... (1 + rho*alpha) c - rho*alpha w
                    for d in 0..n {
                        scr[1657usize + d] = 1.5f32 * scr[1641usize + d] - 0.5f32 * scr[1560usize + worst * n + d];
                    }
                } else {
                    // inside contraction xc = 0.5*c + 0.5*w
                    for d in 0..n {
                        scr[1657usize + d] = 0.5f32 * scr[1641usize + d] + 0.5f32 * scr[1560usize + worst * n + d];
                    }
                }
                let fc = arima_css(scr, 1657usize, dn, p, arn, n, start, cst);
                let mut accept = fc < fworst;
                if use_ref {
                    accept = fc <= fr;
                }
                if accept {
                    for d in 0..n {
                        scr[1560usize + worst * n + d] = scr[1657usize + d];
                    }
                    scr[1632usize + worst] = fc;
                } else {
                    for i in 0..(n + 1usize) {
                        if i != best {
                            for d in 0..n {
                                let bx = scr[1560usize + best * n + d];
                                scr[1560usize + i * n + d] = bx + 0.5f32 * (scr[1560usize + i * n + d] - bx);
                            }
                            scr[1632usize + i] = arima_css(scr, 1560usize + i * n, dn, p, arn, n, start, cst);
                        }
                    }
                }
            }
        }
    }
    let mut bi = 0usize;
    for i in 1..(n + 1usize) {
        if scr[1632usize + i] < scr[1632usize + bi] {
            bi = i;
        }
    }
    for d in 0..n {
        scr[1552usize + d] = scr[1560usize + bi * n + d];
    }
}

#[cube]
fn fit_scan(x: &[f32], scr: &mut [f32], out: &mut [f32], n: u32, formula: u32, p: u32, d: u32, q: u32) {
    let nu = n as usize;
    let pu = p as usize;
    let du = d as usize;
    let qu = q as usize;
    let mut min_obs = pu + du + qu + 1usize;
    if min_obs < 30usize {
        min_obs = 30usize;
    }
    let mut forecast = 0.0f32;
    if formula == 1337u32 {
        for t in 0..nu {
            let mut ws = 0usize;
            if t + 1usize > 512usize {
                ws = t + 1usize - 512usize;
            }
            let len = t + 1usize - ws;
            if len >= min_obs {
                // differencing in place: copy then d passes
                for i in 0..len {
                    scr[i] = x[ws + i];
                }
                let mut dl = len;
                for _k in 0..du {
                    for i in 1..dl {
                        scr[i - 1usize] = scr[i] - scr[i - 1usize];
                    }
                    if dl > 0usize {
                        dl = dl - 1usize;
                    }
                }
                // AR via OLS
                let mut arn = 0usize;
                if pu > 0usize && dl >= pu + 2usize {
                    arn = pu;
                    let rows = dl - pu;
                    for i in 0..(pu * pu) {
                        scr[1680usize + i] = 0.0f32;
                    }
                    for i in 0..pu {
                        scr[1936usize + i] = 0.0f32;
                    }
                    for r in pu..dl {
                        let yr = scr[r];
                        for i in 0..pu {
                            let xi = scr[r - 1usize - i];
                            scr[1936usize + i] = scr[1936usize + i] + xi * yr;
                            for j in 0..pu {
                                scr[1680usize + i * pu + j] = scr[1680usize + i * pu + j] + xi * scr[r - 1usize - j];
                            }
                        }
                    }
                    let _ = rows;
                    // pivoted Gaussian elimination
                    let mut singular = false;
                    for col in 0..pu {
                        if !singular {
                            let mut prow = col;
                            let mut pval = scr[1680usize + col * pu + col].abs();
                            for r in (col + 1usize)..pu {
                                let vv = scr[1680usize + r * pu + col].abs();
                                if vv > pval {
                                    pval = vv;
                                    prow = r;
                                }
                            }
                            if pval < 1.0e-30f32 {
                                singular = true;
                            } else {
                                if prow != col {
                                    for c2 in 0..pu {
                                        let tmp = scr[1680usize + col * pu + c2];
                                        scr[1680usize + col * pu + c2] = scr[1680usize + prow * pu + c2];
                                        scr[1680usize + prow * pu + c2] = tmp;
                                    }
                                    let tb = scr[1936usize + col];
                                    scr[1936usize + col] = scr[1936usize + prow];
                                    scr[1936usize + prow] = tb;
                                }
                                let diag = scr[1680usize + col * pu + col];
                                for r in (col + 1usize)..pu {
                                    let factor = scr[1680usize + r * pu + col] / diag;
                                    if factor != 0.0f32 {
                                        for c2 in col..pu {
                                            scr[1680usize + r * pu + c2] = scr[1680usize + r * pu + c2] - factor * scr[1680usize + col * pu + c2];
                                        }
                                        scr[1936usize + r] = scr[1936usize + r] - factor * scr[1936usize + col];
                                    }
                                }
                            }
                        }
                    }
                    if singular {
                        for i in 0..pu {
                            scr[1536usize + i] = 0.0f32;
                        }
                    } else {
                        for cc in 0..pu {
                            let col = pu - 1usize - cc;
                            let mut s = scr[1936usize + col];
                            for c2 in (col + 1usize)..pu {
                                s = s - scr[1680usize + col * pu + c2] * scr[1536usize + c2];
                            }
                            scr[1536usize + col] = s / scr[1680usize + col * pu + col];
                        }
                    }
                }
                // constant
                let mut cst = 0.0f32;
                if dl > 0usize {
                    let mut sm = 0.0f32;
                    for i in 0..dl {
                        sm = sm + scr[i];
                    }
                    let mut asum = 0.0f32;
                    for i in 0..arn {
                        asum = asum + scr[1536usize + i];
                    }
                    cst = (sm / (dl as f32)) * (1.0f32 - asum);
                }
                // MA via Nelder-Mead on the CSS
                let mut qq = qu;
                if qq > 8usize {
                    qq = 8usize;
                }
                let mut man = 0usize;
                if qq > 0usize {
                    man = qq;
                    let mut start = pu;
                    if qq > start {
                        start = qq;
                    }
                    if dl <= start + 1usize {
                        for i in 0..qq {
                            scr[1552usize + i] = 0.0f32;
                        }
                    } else {
                        arima_nm(scr, dl, pu, arn, qq, start, cst);
                        for i in 0..qq {
                            scr[1552usize + i] = reflect_into(scr[1552usize + i], -0.999f32, 0.999f32);
                        }
                    }
                }
                // fitted values / residuals (residuals reuse 512..)
                let mut sidx = pu;
                if qu > sidx {
                    sidx = qu;
                }
                let ready = dl > 0usize && dl > sidx;
                if ready {
                    let mut rl = 0usize;
                    for tt in sidx..dl {
                        let mut fv = cst;
                        for i in 0..arn {
                            if tt > i {
                                fv = fv + scr[1536usize + i] * scr[tt - 1usize - i];
                            }
                        }
                        for i in 0..man {
                            if rl > i {
                                fv = fv + scr[1552usize + i] * scr[512usize + rl - 1usize - i];
                            }
                        }
                        scr[512usize + rl] = scr[tt] - fv;
                        rl = rl + 1usize;
                    }
                    let mut fc = cst;
                    for i in 0..arn {
                        if dl > i {
                            fc = fc + scr[1536usize + i] * scr[dl - 1usize - i];
                        }
                    }
                    for i in 0..man {
                        if rl > i {
                            fc = fc + scr[1552usize + i] * scr[512usize + rl - 1usize - i];
                        }
                    }
                    forecast = fc;
                }
            }
            out[t] = forecast;
        }
    }
}

#[cube(launch_unchecked)]
fn fit_map(x: &[f32], scr: &mut [f32], out: &mut [f32], n: u32, formula: u32, p: u32, d: u32, q: u32) {
    fit_scan(x, scr, out, n, formula, p, d, q);
}

/// Launch a fit formula over a single source series.
pub fn launch_cube_fit(formula: CubeFormula, x: &[f32], params: CubeParams) -> Vec<Vec<f32>> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let xb = client.create_from_slice(f32::as_bytes(x));
    let scr = client.create_from_slice(f32::as_bytes(&vec![0.0f32; FIT_SCR]));
    let out = client.empty(n * 4);
    let (p, d, q) = (params.period.min(16), params.fast.min(3), params.slow.min(16));
    unsafe {
        fit_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(xb, n),
            BufferArg::from_raw_parts(scr, FIT_SCR),
            BufferArg::from_raw_parts(out.clone(), n),
            n as u32,
            formula.code(),
            p,
            d,
            q,
        );
    }
    vec![f32::from_bytes(&client.read_one_unchecked(out)).to_vec()]
}
