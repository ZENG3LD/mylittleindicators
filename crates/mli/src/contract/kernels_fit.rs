//! Iterative model-fit formulas as FIXED-ITERATION SEQUENTIAL kernels (one thread, refit on every bar like the
//! CPU). The CPU algorithms are ported 1:1 in f32 (same Nelder-Mead variant, same iteration caps); f32 cannot
//! reproduce the f64 simplex path bit for bit, so results agree only to f32 tolerance on well-conditioned
//! data (UNTESTED on GPU).
//!
//! - `1338` Garch / `1339` EGarch (`period` = p, `fast` = q, both <= 8; >= 1): the CPU feed derives each return from
//!   `ln(1 + previous return)` (first return 0), which keeps every return at exactly 0 - ported literally.
//!   AR(1) mean model, residuals, Nelder-Mead (1500 / 2000 iterations, step 0.3) on the (E)GARCH negative
//!   log-likelihood over reflected-bound parameters, conditional variance recursion; output = last volatility
//!   (initial 0.1 / 0.316 before the first fit at `max(p + q + 10, 50)` returns).
//! - `1337` Arima (`period` = p, `fast` = d, `slow` = q): differencing, OLS AR fit (normal equations + pivoted
//!   Gaussian elimination, pivot threshold 1e-30 instead of 1e-300), constant, MA fit by Nelder-Mead (<= 8 params,
//!   max 1500 iterations, step 0.3, ftol = xtol = 1e-10) on the conditional sum of squares, fitted residuals and
//!   the one-step forecast. The forecast keeps its previous value while the model cannot be formed.

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::gpu::CubeParams;
use super::CubeFormula;

// scratch layout (f32 indices)
pub const FIT_SCR: usize = 4096;

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
fn gobj(scr: &mut [f32], n_par: usize, p: usize, q: usize, nres: usize, uvar: f32, kind: u32, rawoff: usize) -> f32 {
    // theta at 1536.. from raw at `rawoff`; residuals at 0, var/logvar hist at 512, z at 1024
    let beta_lo = if kind == 0u32 { 1usize + p } else { 1usize + 2usize * p };
    for k in 0..n_par {
        let raw = scr[rawoff + k];
        if kind == 0u32 {
            if k == 0usize {
                scr[1536usize + k] = reflect_into(raw, 1.0e-12f32, uvar * 5.0f32);
            } else {
                scr[1536usize + k] = reflect_into(raw, 0.0f32, 0.999f32);
            }
        } else if k >= beta_lo {
            scr[1536usize + k] = reflect_into(raw, -0.999f32, 0.999f32);
        } else if k == 0usize {
            scr[1536usize + k] = reflect_into(raw, -10.0f32, 5.0f32);
        } else {
            scr[1536usize + k] = reflect_into(raw, -2.0f32, 2.0f32);
        }
    }
    let mut persist = 0.0f32;
    for k in 1..n_par {
        if kind == 0u32 || k >= beta_lo {
            persist = persist + scr[1536usize + k];
        }
    }
    let mut res = 0.0f32;
    let mut done = false;
    let mut pv = persist;
    if kind != 0u32 {
        pv = persist.abs();
    }
    if pv >= 0.9999f32 {
        res = 1.0e12f32 + 1.0e6f32 * (pv - 0.9999f32).abs();
        done = true;
    }
    if !done {
        let mut start = p;
        if q > start {
            start = q;
        }
        let mut ok = nres > start + 1usize;
        let omega = scr[1536usize];
        if kind == 0u32 && omega <= 0.0f32 {
            ok = false;
        }
        let two_pi_ln = 1.8378771f32;
        let mut nll = 0.0f32;
        if ok {
            if kind == 0u32 {
                for t in 0..nres {
                    scr[512usize + t] = uvar;
                }
                for t in start..nres {
                    if ok {
                        let mut v = omega;
                        for i in 0..p {
                            let r = scr[t - 1usize - i];
                            v = v + scr[1537usize + i] * r * r;
                        }
                        for j in 0..q {
                            v = v + scr[1537usize + p + j] * scr[512usize + t - 1usize - j];
                        }
                        if v != v || v > 3.0e38f32 || v <= 0.0f32 {
                            ok = false;
                        } else {
                            scr[512usize + t] = v;
                            let e = scr[t];
                            nll = nll + 0.5f32 * (v.ln() + e * e / v + two_pi_ln);
                        }
                    }
                }
            } else {
                // uvar here carries init_log_var
                for t in 0..nres {
                    scr[512usize + t] = uvar;
                    scr[1024usize + t] = 0.0f32;
                }
                let seed = (uvar.exp()).sqrt().max(1.0e-12f32);
                for i in 0..start {
                    scr[1024usize + i] = scr[i] / seed;
                }
                for t in start..nres {
                    if ok {
                        let mut lv = omega;
                        for i in 0..p {
                            let zt = scr[1024usize + t - 1usize - i];
                            lv = lv + scr[1537usize + i] * zt.abs() + scr[1537usize + p + i] * zt;
                        }
                        for j in 0..q {
                            lv = lv + scr[1537usize + 2usize * p + j] * scr[512usize + t - 1usize - j];
                        }
                        if lv != lv || lv > 3.0e38f32 || lv < -3.0e38f32 {
                            ok = false;
                        } else {
                            scr[512usize + t] = lv;
                            let v = lv.exp();
                            if v != v || v > 3.0e38f32 || v <= 0.0f32 {
                                ok = false;
                            } else {
                                let e = scr[t];
                                scr[1024usize + t] = e / v.sqrt();
                                nll = nll + 0.5f32 * (lv + e * e / v + two_pi_ln);
                            }
                        }
                    }
                }
            }
        }
        if ok && nll == nll && nll < 3.0e38f32 && nll > -3.0e38f32 {
            res = nll;
        } else {
            res = 1.0e12f32 + 1.0e6f32;
        }
    }
    res
}

/// Nelder-Mead over [`gobj`]: simplex 1560 ((n+1) x n, n <= 17), fvals 1866, centroid 1884, xr 1901, xt 1918,
/// order 1935. Best raw vertex copied to `scr[2000..2000 + n]`... (returns window lives at 2048..)
#[cube]
fn garch_nm(scr: &mut [f32], n: usize, p: usize, q: usize, nres: usize, uvar: f32, kind: u32, max_iters: u32) {
    for d in 0..n {
        scr[1560usize + d] = scr[2000usize + d];
    }
    for i in 0..n {
        for d in 0..n {
            scr[1560usize + (i + 1usize) * n + d] = scr[2000usize + d];
        }
        let v = scr[2000usize + i];
        let mut h = 0.3f32;
        if v.abs() > 1.0e-8f32 {
            h = 0.3f32 * v.abs();
        }
        scr[1560usize + (i + 1usize) * n + i] = v + h;
    }
    for i in 0..(n + 1usize) {
        // copy vertex to raw slot 1953.. (17 wide) then evaluate
        for d in 0..n {
            scr[1953usize + d] = scr[1560usize + i * n + d];
        }
        scr[1866usize + i] = gobj(scr, n, p, q, nres, uvar, kind, 1953usize);
    }
    let mut it = 0u32;
    let mut go = true;
    while go && it < max_iters {
        it = it + 1u32;
        for i in 0..(n + 1usize) {
            scr[1935usize + i] = i as f32;
        }
        for i in 1..(n + 1usize) {
            let key = scr[1935usize + i];
            let kf = scr[1866usize + (key as usize)];
            let mut j = i;
            let mut moving = true;
            while moving && j > 0usize {
                let prev = scr[1935usize + j - 1usize];
                let pf = scr[1866usize + (prev as usize)];
                if pf > kf {
                    scr[1935usize + j] = prev;
                    j = j - 1usize;
                } else {
                    moving = false;
                }
            }
            scr[1935usize + j] = key;
        }
        let best = scr[1935usize] as usize;
        let worst = scr[1935usize + n] as usize;
        let second = scr[1935usize + n - 1usize] as usize;
        let fspread = (scr[1866usize + worst] - scr[1866usize + best]).abs();
        let mut xsize = 0.0f32;
        for i in 0..(n + 1usize) {
            if i != best {
                let mut sq = 0.0f32;
                for d in 0..n {
                    let df = scr[1560usize + i * n + d] - scr[1560usize + best * n + d];
                    sq = sq + df * df;
                }
                xsize = xsize.max(sq.sqrt());
            }
        }
        if fspread <= 1.0e-10f32 && xsize <= 1.0e-10f32 {
            go = false;
        }
        if go {
            for d in 0..n {
                scr[1884usize + d] = 0.0f32;
            }
            for k in 0..n {
                let idx = scr[1935usize + k] as usize;
                for d in 0..n {
                    scr[1884usize + d] = scr[1884usize + d] + scr[1560usize + idx * n + d];
                }
            }
            for d in 0..n {
                scr[1884usize + d] = scr[1884usize + d] / (n as f32);
            }
            for d in 0..n {
                scr[1901usize + d] = 2.0f32 * scr[1884usize + d] - scr[1560usize + worst * n + d];
                scr[1953usize + d] = scr[1901usize + d];
            }
            let fr = gobj(scr, n, p, q, nres, uvar, kind, 1953usize);
            let fbest = scr[1866usize + best];
            let fsec = scr[1866usize + second];
            let fworst = scr[1866usize + worst];
            if fr < fbest {
                for d in 0..n {
                    scr[1918usize + d] = 3.0f32 * scr[1884usize + d] - 2.0f32 * scr[1560usize + worst * n + d];
                    scr[1953usize + d] = scr[1918usize + d];
                }
                let fe = gobj(scr, n, p, q, nres, uvar, kind, 1953usize);
                if fe < fr {
                    for d in 0..n {
                        scr[1560usize + worst * n + d] = scr[1918usize + d];
                    }
                    scr[1866usize + worst] = fe;
                } else {
                    for d in 0..n {
                        scr[1560usize + worst * n + d] = scr[1901usize + d];
                    }
                    scr[1866usize + worst] = fr;
                }
            } else if fr < fsec {
                for d in 0..n {
                    scr[1560usize + worst * n + d] = scr[1901usize + d];
                }
                scr[1866usize + worst] = fr;
            } else {
                let mut use_ref = false;
                if fr < fworst {
                    use_ref = true;
                    for d in 0..n {
                        scr[1918usize + d] = 1.5f32 * scr[1884usize + d] - 0.5f32 * scr[1560usize + worst * n + d];
                    }
                } else {
                    for d in 0..n {
                        scr[1918usize + d] = 0.5f32 * scr[1884usize + d] + 0.5f32 * scr[1560usize + worst * n + d];
                    }
                }
                for d in 0..n {
                    scr[1953usize + d] = scr[1918usize + d];
                }
                let fc = gobj(scr, n, p, q, nres, uvar, kind, 1953usize);
                let mut accept = fc < fworst;
                if use_ref {
                    accept = fc <= fr;
                }
                if accept {
                    for d in 0..n {
                        scr[1560usize + worst * n + d] = scr[1918usize + d];
                    }
                    scr[1866usize + worst] = fc;
                } else {
                    for i in 0..(n + 1usize) {
                        if i != best {
                            for d in 0..n {
                                let bx = scr[1560usize + best * n + d];
                                scr[1560usize + i * n + d] = bx + 0.5f32 * (scr[1560usize + i * n + d] - bx);
                                scr[1953usize + d] = scr[1560usize + i * n + d];
                            }
                            scr[1866usize + i] = gobj(scr, n, p, q, nres, uvar, kind, 1953usize);
                        }
                    }
                }
            }
        }
    }
    let mut bi = 0usize;
    for i in 1..(n + 1usize) {
        if scr[1866usize + i] < scr[1866usize + bi] {
            bi = i;
        }
    }
    for d in 0..n {
        scr[2000usize + d] = scr[1560usize + bi * n + d];
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
    } else if formula == 1338u32 || formula == 1339u32 {
        // returns window at 2100.. (<= 512), cond. variance / log variance results at 2700.., z at 3300..
        let kind = formula - 1338u32;
        let mut min_obs2 = pu + qu + 10usize;
        if min_obs2 < 50usize {
            min_obs2 = 50usize;
        }
        let mut vol = 0.1f32;
        if kind == 1u32 {
            vol = 0.316f32;
        }
        let mut rcount = 0usize;
        let mut rlast = 0.0f32;
        for t in 0..nu {
            // return recursion r = ln(price / (price / (1 + r_prev))) = ln(1 + r_prev)
            let mut r = 0.0f32;
            if rcount > 0usize {
                let prev_price = x[t] / (1.0f32 + rlast);
                r = (x[t] / prev_price).ln();
            }
            if rcount >= 512usize {
                for i in 1..512usize {
                    scr[2100usize + i - 1usize] = scr[2100usize + i];
                }
                rcount = 511usize;
            }
            scr[2100usize + rcount] = r;
            rcount = rcount + 1usize;
            rlast = r;
            if rcount >= min_obs2 {
                // AR(1) mean model
                let mut phi = 0.0f32;
                let mut mu = 0.0f32;
                let mut sr = 0.0f32;
                let mut srl = 0.0f32;
                let mut srr = 0.0f32;
                let mut srls = 0.0f32;
                for i in 1..rcount {
                    let rt = scr[2100usize + i];
                    let rl = scr[2100usize + i - 1usize];
                    sr = sr + rt;
                    srl = srl + rl;
                    srr = srr + rt * rl;
                    srls = srls + rl * rl;
                }
                let npairs = (rcount - 1usize) as f32;
                let mean_r = sr / npairs;
                let mean_rl = srl / npairs;
                let den = srls - npairs * mean_rl * mean_rl;
                if den.abs() > 1.0e-10f32 {
                    phi = (srr - npairs * mean_r * mean_rl) / den;
                    mu = mean_r - phi * mean_rl;
                } else {
                    mu = mean_r;
                }
                phi = phi.max(-0.99f32).min(0.99f32);
                scr[0] = scr[2100usize] - mu;
                for i in 1..rcount {
                    scr[i] = scr[2100usize + i] - (mu + phi * scr[2100usize + i - 1usize]);
                }
                let nres = rcount;
                let mut start = pu;
                if qu > start {
                    start = qu;
                }
                let mut uv = 0.0f32;
                for i in 0..nres {
                    uv = uv + scr[i] * scr[i];
                }
                uv = uv / (nres as f32);
                let uvar = uv.max(1.0e-10f32);
                let mut n_par = 1usize + pu + qu;
                if kind == 1u32 {
                    n_par = 1usize + 2usize * pu + qu;
                }
                let mut ctx = uvar;
                if kind == 1u32 {
                    ctx = uvar.ln();
                    scr[2000usize] = ctx * 0.1f32;
                    for i in 0..pu {
                        scr[2001usize + i] = 0.15f32;
                        scr[2001usize + pu + i] = -0.05f32;
                    }
                    for j in 0..qu {
                        scr[2001usize + 2usize * pu + j] = 0.9f32 / (qu as f32);
                    }
                } else {
                    scr[2000usize] = uvar * 0.1f32;
                    for i in 0..pu {
                        scr[2001usize + i] = 0.05f32 / (pu as f32);
                    }
                    for j in 0..qu {
                        scr[2001usize + pu + j] = 0.85f32 / (qu as f32);
                    }
                }
                let mut iters = 1500u32;
                if kind == 1u32 {
                    iters = 2000u32;
                }
                garch_nm(scr, n_par, pu, qu, nres, ctx, kind, iters);
                // final parameters -> theta at 1536.. via the objective's reflection (cheap re-evaluation)
                let _fx = gobj(scr, n_par, pu, qu, nres, ctx, kind, 2000usize);
                // conditional variance recursion over the residuals
                if kind == 0u32 {
                    let omega = scr[1536usize];
                    for i in 0..start {
                        scr[2700usize + i] = uv;
                    }
                    let mut clen = start;
                    for tt in start..nres {
                        let mut variance = omega;
                        for i in 0..pu {
                            if tt > i {
                                let rr = scr[tt - 1usize - i];
                                variance = variance + scr[1537usize + i] * rr * rr;
                            }
                        }
                        for j in 0..qu {
                            if clen > j {
                                variance = variance + scr[1537usize + pu + j] * scr[2700usize + clen - 1usize - j];
                            }
                        }
                        variance = variance.max(1.0e-8f32);
                        scr[2700usize + clen] = variance;
                        clen = clen + 1usize;
                    }
                    if clen > 0usize {
                        vol = scr[2700usize + clen - 1usize].sqrt();
                    }
                } else {
                    let omega = scr[1536usize];
                    let sd0 = 0.31663677f32;
                    for i in 0..start {
                        scr[2700usize + i] = -2.3f32;
                    }
                    let mut zl = 0usize;
                    for i in 0..start {
                        if i < nres {
                            scr[3300usize + zl] = scr[i] / sd0;
                            zl = zl + 1usize;
                        }
                    }
                    let mut clen = start;
                    for tt in start..nres {
                        let mut lv = omega;
                        for i in 0..pu {
                            if zl > i {
                                let z = scr[3300usize + zl - 1usize - i];
                                lv = lv + scr[1537usize + i] * z.abs() + scr[1537usize + pu + i] * z;
                            }
                        }
                        for j in 0..qu {
                            if clen > j {
                                lv = lv + scr[1537usize + 2usize * pu + j] * scr[2700usize + clen - 1usize - j];
                            }
                        }
                        scr[2700usize + clen] = lv;
                        clen = clen + 1usize;
                        scr[3300usize + zl] = scr[tt] / lv.exp().sqrt();
                        zl = zl + 1usize;
                    }
                    if clen > 0usize {
                        vol = scr[2700usize + clen - 1usize].exp().sqrt();
                    }
                }
            }
            out[t] = vol;
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
    let (p, d, q) = if formula == CubeFormula::ArimaBar { (params.period.min(16), params.fast.min(3), params.slow.min(16)) } else { (params.period.min(8), 0, params.fast.min(8)) };
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
