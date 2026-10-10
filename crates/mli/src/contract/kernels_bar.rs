//! Stateful bar-series formulas, codes 1200..=1399: one sequential scan over the five OHLCV
//! lanes (`open`, `high`, `low`, `close`, `volume`), up to five output columns
//! (`out[col * n + i]`). Used by indicators whose CPU state does not fit the smoother-chain
//! composites (ring buffers read in storage order, held values, state machines).
//! Parameters: `period` / `p2` integers, `a` / `b` / `c` floats, `flag`.
//! UNTESTED on GPU (no GPU on the authoring box).

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::gpu::CubeParams;
use super::kernels::smooth_series;
use super::gpu_sample::GpuSample;
use super::kernels_comp::lane_series;
use super::kernels_post::kth_in;
use super::CubeFormula;
use crate::engine::ohlcv_field::OhlcvField;

/// `(close - min low) / (max high - min low)` building blocks of the Nautilus stochastic:
/// returns `[c1, h1, ok]` where `ok = 1` when the range of the window ending at bar `t` is
/// not degenerate.
#[cube]
fn stoch_win(h: &[f32], l: &[f32], c: &[f32], t: usize, pk: usize, which: u32) -> f32 {
    let mut hi = h[t + 1 - pk];
    let mut lo = l[t + 1 - pk];
    for j in (t + 1 - pk)..(t + 1) {
        if h[j] > hi {
            hi = h[j];
        }
        if l[j] < lo {
            lo = l[j];
        }
    }
    let mut r = 0.0f32;
    if which == 0u32 {
        r = c[t] - lo;
    } else if which == 1u32 {
        r = hi - lo;
    }
    r
}

/// Midpoint of the highest high and lowest low over the `w` bars ending at `q` (needs `q + 1 >= w`).
#[cube]
fn hl_mid(h: &[f32], l: &[f32], q: usize, w: usize) -> f32 {
    let mut hi = h[q + 1 - w];
    let mut lo = l[q + 1 - w];
    for j in (q + 1 - w)..(q + 1) {
        if h[j] > hi {
            hi = h[j];
        }
        if l[j] < lo {
            lo = l[j];
        }
    }
    (hi + lo) / 2.0f32
}

/// Senkou span A at bar `q` (0 until both Tenkan and Kijun windows are full).
#[cube]
fn ichi_a(h: &[f32], l: &[f32], q: usize, tp: usize, kp: usize) -> f32 {
    let mut r = 0.0f32;
    if q + 1 >= tp && q + 1 >= kp {
        r = (hl_mid(h, l, q, tp) + hl_mid(h, l, q, kp)) / 2.0f32;
    }
    r
}

/// Senkou span B at bar `q` (0 until the window is full).
#[cube]
fn ichi_b(h: &[f32], l: &[f32], q: usize, sp: usize) -> f32 {
    let mut r = 0.0f32;
    if q + 1 >= sp {
        r = hl_mid(h, l, q, sp);
    }
    r
}

/// Median (`len / 2` order statistic) of `|c[j] - mid|` over a window, by rank counting.
#[cube]
fn kth_dev(src: &[f32], start: usize, len: usize, k: u32, mid: f32) -> f32 {
    let mut res = (src[start] - mid).abs();
    for j in 0..len {
        let x = (src[start + j] - mid).abs();
        let mut lt = 0u32;
        let mut le = 0u32;
        for m in 0..len {
            let y = (src[start + m] - mid).abs();
            if y < x {
                lt = lt + 1u32;
            }
            if y <= x {
                le = le + 1u32;
            }
        }
        if lt <= k && k < le {
            res = x;
        }
    }
    res
}

/// Whether the `len`-long patterns starting at `base + i` and `base + j` agree within `r`.
#[cube]
fn pat_match(c: &[f32], base: usize, n: usize, i: usize, j: usize, len: usize, r: f32) -> bool {
    let mut ok = true;
    for k in 0..len {
        if i + k >= n || j + k >= n {
            ok = false;
        } else if (c[base + i + k] - c[base + j + k]).abs() > r {
            ok = false;
        }
    }
    ok
}

/// Approximate-entropy `phi` of pattern length `len` over the window `[base, base + n)`.
#[cube]
fn apen_phi(c: &[f32], base: usize, n: usize, len: usize, r: f32) -> f32 {
    let mut res = 0.0f32;
    if len < n {
        let cnt = n - len + 1;
        let mut sum = 0.0f32;
        let mut valid = 0.0f32;
        for i in 0..cnt {
            let mut m = 0u32;
            for j in 0..cnt {
                if pat_match(c, base, n, i, j, len, r) {
                    m = m + 1u32;
                }
            }
            if m > 0u32 {
                sum = sum + ((m as f32) / (cnt as f32)).ln();
                valid = valid + 1.0f32;
            }
        }
        if valid > 0.0f32 {
            res = sum / valid;
        }
    }
    res
}

/// Number of ordered matching pattern pairs `(i != j)` of length `len`.
#[cube]
fn sampen_count(c: &[f32], base: usize, n: usize, len: usize, r: f32) -> f32 {
    let mut m = 0.0f32;
    if len < n {
        let cnt = n - len + 1;
        for i in 0..cnt {
            for j in 0..cnt {
                if i != j && pat_match(c, base, n, i, j, len, r) {
                    m = m + 1.0f32;
                }
            }
        }
    }
    m
}

/// Population std of the window `[base, base + n)`.
#[cube]
fn win_std(c: &[f32], base: usize, n: usize) -> f32 {
    let mut sm = 0.0f32;
    for j in 0..n {
        sm = sm + c[base + j];
    }
    let mean = sm / (n as f32);
    let mut var = 0.0f32;
    for j in 0..n {
        var = var + (c[base + j] - mean) * (c[base + j] - mean);
    }
    (var / (n as f32)).sqrt()
}

/// Log return held in ring slot `i` of a `w`-slot return ring (first return at bar 1 goes to slot 0)
/// after bar `t` (needs `t >= w`).
#[cube]
fn ring_ret(c: &[f32], t: usize, w: usize, i: usize) -> f32 {
    let m = (t - 1) - ((t - 1 + w - i) % w);
    (c[m + 1] / c[m]).ln()
}

/// One DFA scale step at bar `t` (needs `t >= n`): rebuilds the CPU ring (slot 0 holds the
/// previous fluctuation `f_prev` unless the current close lands there) and returns the new
/// fluctuation `f`.
#[cube]
fn dfa_f(c: &[f32], t: usize, n: usize, f_prev: f32, scr: &mut [f32]) -> f32 {
    let mut sm = 0.0f32;
    for i in 0..n {
        let mut val = c[t - ((t + n - i) % n)];
        if i == 0usize {
            if t % n == 0usize {
                val = c[t];
            } else if t > n {
                val = f_prev;
            } else {
                val = c[0];
            }
        }
        scr[i] = val;
        sm = sm + val;
    }
    let nf = n as f32;
    let mean = sm / nf;
    let mut sxy = 0.0f32;
    let mut sx = 0.0f32;
    let mut sy = 0.0f32;
    let mut sxx = 0.0f32;
    for i in 0..n {
        let x = i as f32;
        let y = scr[i] - mean;
        sx = sx + x;
        sy = sy + y;
        sxy = sxy + x * y;
        sxx = sxx + x * x;
    }
    let den = nf * sxx - sx * sx;
    let mut a = 0.0f32;
    let mut beta = 0.0f32;
    if den.abs() > 1.0e-12f32 {
        beta = (nf * sxy - sx * sy) / den;
        a = (sy - beta * sx) / nf;
    }
    let mut rss = 0.0f32;
    for i in 0..n {
        let r = (scr[i] - mean) - (a + beta * (i as f32));
        rss = rss + r * r;
    }
    (rss / nf).sqrt()
}

/// Newey-West long-run variance (automatic bandwidth) of `scr[off..off + n]`.
#[cube]
fn nw_lrv(scr: &mut [f32], off: usize, n: usize) -> f32 {
    let nf = n as f32;
    let mut res = 0.0f32;
    if n >= 2usize {
        let mut em = 0.0f32;
        for i in 0..n {
            em = em + scr[off + i];
        }
        em = em / nf;
        let bwf = 4.0f32 * ((nf / 100.0f32).ln() * 0.22222222f32).exp();
        let mut l = bwf.floor() as usize;
        if l < 1usize {
            l = 1usize;
        }
        if l > n - 1 {
            l = n - 1;
        }
        let mut g0 = 0.0f32;
        for i in 0..n {
            let d = scr[off + i] - em;
            g0 = g0 + d * d;
        }
        let mut lrv = g0 / nf;
        for lag in 1..(l + 1) {
            let mut g = 0.0f32;
            for k in lag..n {
                g = g + (scr[off + k] - em) * (scr[off + k - lag] - em);
            }
            g = g / nf;
            let wgt = 1.0f32 - (lag as f32) / ((l as f32) + 1.0f32);
            lrv = lrv + 2.0f32 * wgt * g;
        }
        res = lrv.max(0.0f32);
    }
    res
}

/// OLS of `y` on the design `X` (row-major `rows x k`, column 0 a constant) held in `scr`
/// (`xo` / `yo`), reporting the coefficient and t-statistic of column `gc`. Non-constant columns are
/// centred and scaled in place first (the t-statistic is invariant; it keeps `f32` normal
/// equations usable). Work area at `wo` (needs `k * k + 4 * k` floats). Writes `scr[wo] = gamma`,
/// `scr[wo + 1] = t`, `scr[wo + 2] = sse` (-1 when singular) and returns 1.0 on success, 0.0 when singular / degenerate.
#[cube]
fn ols_gamma(scr: &mut [f32], xo: usize, yo: usize, rows: usize, k: usize, wo: usize, gc: usize) -> f32 {
    let mut ok = 1.0f32;
    let rf = rows as f32;
    // column scaling: sd stored at wo + 2 + k*k + j (j >= 1)
    let so = wo + 3;
    scr[wo + 2] = 0.0f32 - 1.0f32;
    let a_o = so + k;
    let b1 = a_o + k * k;
    let b2 = b1 + k;
    scr[so] = 1.0f32;
    for j in 1..k {
        let mut m = 0.0f32;
        for r in 0..rows {
            m = m + scr[xo + r * k + j];
        }
        m = m / rf;
        let mut v = 0.0f32;
        for r in 0..rows {
            let d = scr[xo + r * k + j] - m;
            v = v + d * d;
        }
        let mut sd = (v / rf).sqrt();
        if sd < 1.0e-20f32 {
            sd = 1.0f32;
        }
        scr[so + j] = sd;
        for r in 0..rows {
            scr[xo + r * k + j] = (scr[xo + r * k + j] - m) / sd;
        }
    }
    for i in 0..k {
        scr[b1 + i] = 0.0f32;
        scr[b2 + i] = 0.0f32;
        for j in 0..k {
            scr[a_o + i * k + j] = 0.0f32;
        }
    }
    scr[b2 + gc] = 1.0f32;
    for r in 0..rows {
        for i in 0..k {
            let xi = scr[xo + r * k + i];
            scr[b1 + i] = scr[b1 + i] + xi * scr[yo + r];
            for j in 0..k {
                scr[a_o + i * k + j] = scr[a_o + i * k + j] + xi * scr[xo + r * k + j];
            }
        }
    }
    // Gaussian elimination with partial pivoting, two right-hand sides
    for col in 0..k {
        let mut pr = col;
        let mut pv = scr[a_o + col * k + col].abs();
        for r in (col + 1)..k {
            let v = scr[a_o + r * k + col].abs();
            if v > pv {
                pv = v;
                pr = r;
            }
        }
        if pv < 1.0e-20f32 {
            ok = 0.0f32;
        } else {
            if pr != col {
                for c in 0..k {
                    let tmp = scr[a_o + col * k + c];
                    scr[a_o + col * k + c] = scr[a_o + pr * k + c];
                    scr[a_o + pr * k + c] = tmp;
                }
                let t1 = scr[b1 + col];
                scr[b1 + col] = scr[b1 + pr];
                scr[b1 + pr] = t1;
                let t2 = scr[b2 + col];
                scr[b2 + col] = scr[b2 + pr];
                scr[b2 + pr] = t2;
            }
            let diag = scr[a_o + col * k + col];
            for r in (col + 1)..k {
                let f = scr[a_o + r * k + col] / diag;
                if f != 0.0f32 {
                    for c in col..k {
                        scr[a_o + r * k + c] = scr[a_o + r * k + c] - f * scr[a_o + col * k + c];
                    }
                    scr[b1 + r] = scr[b1 + r] - f * scr[b1 + col];
                    scr[b2 + r] = scr[b2 + r] - f * scr[b2 + col];
                }
            }
        }
    }
    let mut gamma = 0.0f32;
    let mut tst = 0.0f32;
    if ok > 0.5f32 {
        // back substitution (solutions overwrite b1 / b2)
        for ci in 0..k {
            let col = k - 1 - ci;
            let mut s1 = scr[b1 + col];
            let mut s2 = scr[b2 + col];
            for c in (col + 1)..k {
                s1 = s1 - scr[a_o + col * k + c] * scr[b1 + c];
                s2 = s2 - scr[a_o + col * k + c] * scr[b2 + c];
            }
            scr[b1 + col] = s1 / scr[a_o + col * k + col];
            scr[b2 + col] = s2 / scr[a_o + col * k + col];
        }
        let mut sse = 0.0f32;
        for r in 0..rows {
            let mut yh = 0.0f32;
            for j in 0..k {
                yh = yh + scr[xo + r * k + j] * scr[b1 + j];
            }
            let e = scr[yo + r] - yh;
            sse = sse + e * e;
        }
        scr[wo + 2] = sse;
        if rows <= k {
            ok = 0.0f32;
        } else {
            let sigma2 = sse / ((rows - k) as f32);
            let se = (sigma2 * scr[b2 + gc]).sqrt();
            if se == 0.0f32 || se != se {
                ok = 0.0f32;
            } else {
                gamma = scr[b1 + gc] / scr[so + gc];
                tst = scr[b1 + gc] / se;
            }
        }
    }
    scr[wo] = gamma;
    scr[wo + 1] = tst;
    ok
}

/// ln Gamma(x) (Lanczos, Numerical Recipes `gammln`).
#[cube]
fn lngam(x: f32) -> f32 {
    let ser = 1.000000000190015f32
        + 76.18009172947146f32 / (x + 1.0f32)
        - 86.50532032941677f32 / (x + 2.0f32)
        + 24.01409824083091f32 / (x + 3.0f32)
        - 1.231739572450155f32 / (x + 4.0f32)
        + 0.001208650973866179f32 / (x + 5.0f32)
        - 0.000005395239384953f32 / (x + 6.0f32);
    (x - 0.5f32) * (x + 4.5f32).ln() - (x + 4.5f32) + (2.5066282746310005f32 * ser / x).ln()
}

/// Chi-square survival function `1 - cdf(x; k)` (regularised incomplete gamma, series / continued fraction).
#[cube]
fn chi2_sf_k(xv: f32, kk: f32) -> f32 {
    let mut res = 1.0f32;
    if xv > 0.0f32 {
        let s = kk * 0.5f32;
        let x = xv * 0.5f32;
        let lg = lngam(s);
        let pref = (0.0f32 - x + s * x.ln() - lg).exp();
        if x < s + 1.0f32 {
            let mut ap = s;
            let mut sum = 1.0f32 / s;
            let mut del = sum;
            let mut go = true;
            for _ in 0..200 {
                if go {
                    ap = ap + 1.0f32;
                    del = del * x / ap;
                    sum = sum + del;
                    if del.abs() < sum.abs() * 1.0e-7f32 {
                        go = false;
                    }
                }
            }
            res = 1.0f32 - sum * pref;
        } else {
            let mut b = x + 1.0f32 - s;
            let mut cc = (b / b) * 1.0e30f32;
            let mut d = 1.0f32 / b;
            let mut h = d;
            let mut go = true;
            for i in 1..200 {
                if go {
                    let fi = i as f32;
                    let an = 0.0f32 - fi * (fi - s);
                    b = b + 2.0f32;
                    d = an * d + b;
                    if d.abs() < 1.0e-30f32 {
                        d = 1.0e-30f32;
                    }
                    cc = b + an / cc;
                    if cc.abs() < 1.0e-30f32 {
                        cc = 1.0e-30f32;
                    }
                    d = 1.0f32 / d;
                    let del = d * cc;
                    h = h * del;
                    if (del - 1.0f32).abs() < 1.0e-7f32 {
                        go = false;
                    }
                }
            }
            res = pref * h;
        }
    }
    res
}

/// 1200 Stochastik-D `[k, d]` (`period` %K window, `p2` %D window), 1201 Donchian stop
/// (lower stop: `period` lower window, `p2` upper window unused in the output, `a` offset, `flag`
/// percentage), 1202 projection bands `[upper, middle, lower]` (`period` window clamped 2..=512,
/// `a` k, ring-storage order like the CPU).
/// Theil-Sen ring buffer value at ring position `i` (time order is rotated once the ring wrapped, like the CPU buffer).
#[cube]
fn ts_val(c: &[f32], t: usize, w: usize, i: usize) -> f32 {
    c[i + w * ((t - i) / w)]
}

/// Number of pairwise slopes `(y[j] - y[i]) / (j - i)` (ring positions, i < j) that are `<= x`.
#[cube]
fn ts_cnt_le(c: &[f32], t: usize, w: usize, x: f32) -> usize {
    let mut cnt = 0usize;
    for i in 0..w {
        let yi = ts_val(c, t, w, i);
        for j in (i + 1)..w {
            let s = (ts_val(c, t, w, j) - yi) / ((j - i) as f32);
            if s <= x {
                cnt = cnt + 1usize;
            }
        }
    }
    cnt
}

/// `k`-th smallest (0-based) of `scr[off..off + n]` by rank counting (ties broken by index).
#[cube]
fn ts_kth(scr: &mut [f32], off: usize, n: usize, k: usize) -> f32 {
    let mut res = 0.0f32;
    for i in 0..n {
        let vi = scr[off + i];
        let mut rank = 0usize;
        for j in 0..n {
            let vj = scr[off + j];
            if vj < vi {
                rank = rank + 1usize;
            } else if vj == vi && j < i {
                rank = rank + 1usize;
            }
        }
        if rank == k {
            res = vi;
        }
    }
    res
}

#[cube]
fn bar_scan(
    o: &[f32],
    h: &[f32],
    l: &[f32],
    c: &[f32],
    v: &[f32],
    out: &mut [f32],
    scr: &mut [f32],
    formula: u32,
    period: u32,
    p2: u32,
    p3: u32,
    p4: u32,
    a: f32,
    b: f32,
    _cc: f32,
    flag: u32,
) {
    let n = c.len();
    let mut k = 0.0f32;
    let mut d = 0.0f32;
    let mut dh = 0.0f32;
    let mut dl = 0.0f32;
    let mut cnt_a = 0usize;
    let mut hset = 0u32;
    let mut f0 = 0.0f32;
    let mut f1 = 0.0f32;
    let mut f2 = 0.0f32;
    let mut f3 = 0.0f32;
    let mut rstate = a;
    let mut vw = 0.0f32;
    let mut held0 = 0.0f32;
    let mut held1 = 0.0f32;
    let mut held2 = 0.0f32;
    for t in 0..n {
        let mut v0 = 0.0f32;
        let mut v1 = 0.0f32;
        let mut v2 = 0.0f32;
        let mut v3 = 0.0f32;
        let mut v4 = 0.0f32;
        let mut v5 = 0.0f32;
        let mut v6 = 0.0f32;
        let mut v7 = 0.0f32;
        let mut v8 = 0.0f32;
        let mut v9 = 0.0f32;
        if formula == 1200u32 {
            let mut pk = period as usize;
            if pk < 1 {
                pk = 1;
            }
            let mut pd = p2 as usize;
            if pd < 1 {
                pd = 1;
            }
            if t + 1 > pk {
                let range = stoch_win(h, l, c, t, pk, 1u32);
                if range.abs() >= 1.0e-12f32 {
                    k = 100.0f32 * (stoch_win(h, l, c, t, pk, 0u32) / range);
                    // the last `pd` non-degenerate pairs up to this bar
                    let mut cnt = 0usize;
                    let mut sc = 0.0f32;
                    let mut sh = 0.0f32;
                    let mut q = t + 1;
                    while cnt < pd && q > pk {
                        q = q - 1;
                        let rq = stoch_win(h, l, c, q, pk, 1u32);
                        if rq.abs() >= 1.0e-12f32 {
                            cnt = cnt + 1;
                            sc = sc + stoch_win(h, l, c, q, pk, 0u32);
                            sh = sh + rq;
                        }
                    }
                    if sh.abs() < 1.0e-12f32 {
                        d = 0.0f32;
                    } else {
                        d = 100.0f32 * (sc / sh);
                    }
                }
            }
            v0 = k;
            v1 = d;
        } else if formula == 1201u32 {
            let mut lp = period as usize;
            if lp < 1 {
                lp = 1;
            }
            if t + 1 >= lp {
                let mut lo = l[t + 1 - lp];
                for j in (t + 1 - lp)..(t + 1) {
                    if l[j] < lo {
                        lo = l[j];
                    }
                }
                if flag == 1u32 {
                    v0 = lo * (1.0f32 - a / 100.0f32);
                } else {
                    v0 = lo - a;
                }
            }
        } else if formula == 1202u32 {
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            if w > 512 {
                w = 512;
            }
            let mut kk = a;
            if kk <= 0.0f32 {
                kk = 2.0f32;
            }
            if t + 1 >= w {
                let nf = w as f32;
                // ring slot i holds the latest bar b <= t with b % w == i
                let mut sum = 0.0f32;
                for i in 0..w {
                    let r = (t - i) % w;
                    sum = sum + c[t - r];
                }
                let mean = sum / nf;
                let mut sxx = 0.0f32;
                let mut sxy = 0.0f32;
                let mut sx = 0.0f32;
                let mut var = 0.0f32;
                for i in 0..w {
                    // bar stored in slot i: t - ((t - i) mod w) taken in unsigned arithmetic
                    let back = (t + w - i % w) % w;
                    let y = c[t - back];
                    let xi = i as f32;
                    sx = sx + xi;
                    sxx = sxx + xi * xi;
                    sxy = sxy + xi * (y - mean);
                    var = var + (y - mean) * (y - mean);
                }
                let mut den = nf * sxx - sx * sx;
                if den < 1.0e-9f32 {
                    den = 1.0e-9f32;
                }
                let slope = sxy / den;
                let mid = mean + slope * (nf - 1.0f32);
                let sd = (var / nf).sqrt();
                v0 = mid + kk * sd;
                v1 = mid;
                v2 = mid - kk * sd;
            }
        } else if formula == 1203u32 {
            // raw price channel: rolling max high / min low over min(t+1, period) bars
            let mut w = period as usize;
            if w < 1 {
                w = 1;
            }
            let mut st = 0usize;
            if t + 1 > w {
                st = t + 1 - w;
            }
            let mut hi = h[st];
            let mut lo = l[st];
            for j in st..(t + 1) {
                if h[j] > hi {
                    hi = h[j];
                }
                if l[j] < lo {
                    lo = l[j];
                }
            }
            v0 = hi;
            v1 = (hi + lo) / 2.0f32;
            v2 = lo;
        } else if formula == 1204u32 {
            // Darvas box: cumulative extremes since the first bar
            if t == 0usize {
                dh = h[0];
                dl = l[0];
            } else {
                if h[t] > dh {
                    dh = h[t];
                }
                if l[t] < dl {
                    dl = l[t];
                }
            }
            v0 = dh;
            v1 = dl;
        } else if formula == 1205u32 {
            // quantile-regression channel: median +- k * 1.4826 * MAD over the window
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            if w > 512 {
                w = 512;
            }
            let mut kk = a;
            if kk <= 0.0f32 {
                kk = 2.0f32;
            }
            if t + 1 >= w {
                let mid = kth_in(c, t + 1 - w, w, (w / 2) as u32);
                let mad = kth_dev(c, t + 1 - w, w, (w / 2) as u32, mid);
                let sg = 1.4826f32 * mad;
                held0 = mid + kk * sg;
                held1 = mid;
                held2 = mid - kk * sg;
            }
            v0 = held0;
            v1 = held1;
            v2 = held2;
        } else if formula == 1206u32 || formula == 1207u32 {
            // windowed VWAP channels (1206: [upper, vwap, lower]; 1207: |upper - lower| width)
            let mut w = period as usize;
            if w < 1 {
                w = 1;
            }
            let mut st = 0usize;
            if t + 1 > w {
                st = t + 1 - w;
            }
            let mut spv = 0.0f32;
            let mut sv = 0.0f32;
            for j in st..(t + 1) {
                let tp = (h[j] + l[j] + c[j]) / 3.0f32;
                spv = spv + tp * v[j];
                sv = sv + v[j];
            }
            if sv > 0.0f32 {
                vw = spv / sv;
            }
            let mut up = 0.0f32;
            let mut lw = 0.0f32;
            if t + 1 >= w {
                if flag == 1u32 {
                    let band = vw * (a / 100.0f32);
                    up = vw + band;
                    lw = vw - band;
                } else {
                    let mut var = 0.0f32;
                    for j in st..(t + 1) {
                        let tp = (h[j] + l[j] + c[j]) / 3.0f32;
                        var = var + (tp - vw) * (tp - vw);
                    }
                    let sd = (var / (w as f32)).sqrt();
                    up = vw + a * sd;
                    lw = vw - a * sd;
                }
            }
            if formula == 1206u32 {
                v0 = up;
                v1 = vw;
                v2 = lw;
            } else {
                v0 = (up - lw).abs();
            }
        } else if formula == 1208u32 {
            // volatility percentile-rank bands: the `volume` lane carries the ATR series
            let mut w = p2 as usize;
            if w < 5 {
                w = 5;
            }
            if w > 10000 {
                w = 10000;
            }
            v1 = c[t];
            if t + 1 >= w {
                let p20 = kth_in(v, t + 1 - w, w, ((w * 20) / 100) as u32);
                let p80 = kth_in(v, t + 1 - w, w, ((w * 80) / 100) as u32);
                v0 = c[t] + p80;
                v2 = c[t] - p20;
            }
        } else if formula >= 1209u32 && formula <= 1211u32 {
            // Ichimoku cloud: period/p2/p3 = tenkan/kijun/senkou-B windows, p4 = displacement
            let mut tp = period as usize;
            let mut kp = p2 as usize;
            let mut sp = p3 as usize;
            let mut dp = p4 as usize;
            if tp < 1 {
                tp = 1;
            }
            if kp < 1 {
                kp = 1;
            }
            if sp < 1 {
                sp = 1;
            }
            if dp < 1 {
                dp = 1;
            }
            let mut ten = 0.0f32;
            let mut kij = 0.0f32;
            if t + 1 >= tp {
                ten = hl_mid(h, l, t, tp);
            }
            if t + 1 >= kp {
                kij = hl_mid(h, l, t, kp);
            }
            let sa = ichi_a(h, l, t, tp, kp);
            let sb = ichi_b(h, l, t, sp);
            let mut top = 0.0f32;
            let mut bot = 0.0f32;
            let mut chi = 0.0f32;
            if t + 1 >= dp {
                let q = t + 1 - dp;
                let pa = ichi_a(h, l, q, tp, kp);
                let pb = ichi_b(h, l, q, sp);
                top = pa.max(pb);
                bot = pa.min(pb);
                chi = c[q];
            }
            if formula == 1209u32 {
                v0 = ten;
                v1 = kij;
                v2 = sa;
                v3 = sb;
                v4 = chi;
            } else if formula == 1210u32 {
                let width = (top - bot).abs();
                v0 = 0.5f32;
                if width > 0.0f32 {
                    v0 = ((c[t] - bot) / width).max(0.0f32).min(1.0f32);
                }
            } else {
                v0 = (top - bot).abs();
            }
        } else if formula == 1212u32 || formula == 1213u32 {
            // median channels (Simple mode, close source): median +- 1.4826 * MAD over `period`
            let mut w = period as usize;
            if formula == 1213u32 {
                if w < 3 {
                    w = 3;
                }
            } else if w < 2 {
                w = 2;
            }
            if t + 1 >= w {
                let med = kth_in(c, t + 1 - w, w, (w / 2) as u32);
                let mad = 1.4826f32 * kth_dev(c, t + 1 - w, w, (w / 2) as u32, med);
                if formula == 1212u32 {
                    v0 = med + mad;
                    v1 = med;
                    v2 = med - mad;
                } else {
                    // CPU position reads (median, upper, lower) from the swapped tuple
                    let width = (med - (med - mad)).abs();
                    v0 = 0.5f32;
                    if width > 0.0f32 {
                        v0 = ((c[t] - (med - mad)) / width).max(0.0f32).min(1.0f32);
                    }
                }
            } else if formula == 1213u32 {
                v0 = 0.5f32;
            }
        } else if formula == 1214u32 {
            // DPO bands: the `volume` slot carries the DPO series; window `p2` clamped 5..=512
            let mut w = p2 as usize;
            if w < 5 {
                w = 5;
            }
            if w > 512 {
                w = 512;
            }
            let mut kk = a;
            if kk <= 0.0f32 {
                kk = 2.0f32;
            }
            if t + 1 >= w {
                let mut sum = 0.0f32;
                for j in (t + 1 - w)..(t + 1) {
                    sum = sum + v[j];
                }
                let mean = sum / (w as f32);
                let mut var = 0.0f32;
                for j in (t + 1 - w)..(t + 1) {
                    var = var + (v[j] - mean) * (v[j] - mean);
                }
                let sd = (var / (w as f32)).sqrt();
                v0 = kk * sd;
                v2 = -kk * sd;
            }
        } else if formula == 1215u32 {
            // volatility stop (long stop): v = smoothed typical price, o = ATR (flag 1)
            // flag 0 = std of closes, 1 = ATR, 2 = mean range
            let mut w = period as usize;
            if w < 1 {
                w = 1;
            }
            let mut st = 0usize;
            if t + 1 > w {
                st = t + 1 - w;
            }
            let cnt = t + 1 - st;
            let mut vol = 0.0f32;
            if flag == 1u32 {
                vol = o[t];
            } else if flag == 2u32 {
                let mut sr = 0.0f32;
                for j in st..(t + 1) {
                    sr = sr + (h[j] - l[j]);
                }
                vol = sr / (cnt as f32);
            } else if cnt >= 2usize {
                let mut sm = 0.0f32;
                for j in st..(t + 1) {
                    sm = sm + c[j];
                }
                let mean = sm / (cnt as f32);
                let mut var = 0.0f32;
                for j in st..(t + 1) {
                    var = var + (c[j] - mean) * (c[j] - mean);
                }
                vol = (var / (cnt as f32)).sqrt();
            }
            v0 = v[t] - vol * a;
        } else if formula == 1216u32 {
            // distance to levels: `o` = rolling midline, `v` = percentile-channel middle
            if o[t] != 0.0f32 {
                v0 = (c[t] - o[t]) / o[t].abs().max(1.0e-9f32);
            }
            if v[t] != 0.0f32 {
                v1 = (c[t] - v[t]) / v[t].abs().max(1.0e-9f32);
            }
        } else if formula == 1217u32 {
            // Shannon entropy (normalised): ring of accepted log returns in scr[0..period]
            let pr = period as usize;
            let mut bins = p2 as usize;
            if bins < 2 {
                bins = 2;
            }
            if t > 0usize {
                let lr = (c[t] / c[t - 1]).ln();
                if lr.abs() < 1.0f32 {
                    let wp = cnt_a % pr;
                    scr[wp] = lr;
                    cnt_a = cnt_a + 1usize;
                    let mut len = cnt_a;
                    if len > pr {
                        len = pr;
                    }
                    let mut need = pr;
                    if need > 10usize {
                        need = 10usize;
                    }
                    if len >= need {
                        let mut mn = scr[0];
                        let mut mx = scr[0];
                        for j in 0..len {
                            if scr[j] < mn {
                                mn = scr[j];
                            }
                            if scr[j] > mx {
                                mx = scr[j];
                            }
                        }
                        if (mx - mn).abs() < 1.0e-10f32 {
                            held0 = 0.0f32;
                        } else {
                            let bw = (mx - mn) / (bins as f32);
                            for b in 0..bins {
                                scr[pr + b] = 0.0f32;
                            }
                            for j in 0..len {
                                let mut bi = ((scr[j] - mn) / bw) as usize;
                                if bi > bins - 1 {
                                    bi = bins - 1;
                                }
                                scr[pr + bi] = scr[pr + bi] + 1.0f32;
                            }
                            let mut ent = 0.0f32;
                            for b in 0..bins {
                                let cn = scr[pr + b];
                                if cn > 0.0f32 {
                                    let pb = cn / (len as f32);
                                    ent = ent - pb * pb.ln() / 0.6931472f32;
                                }
                            }
                            let mxe = (bins as f32).ln() / 0.6931472f32;
                            held0 = (ent / mxe).max(0.0f32).min(1.0f32);
                        }
                    }
                }
            }
            v0 = held0;
        } else if formula == 1218u32 {
            // rolling Fisher information: n / variance of the last `period` (>= 10) log returns
            let mut w = period as usize;
            if w < 10 {
                w = 10;
            }
            if t >= w {
                let mut sm = 0.0f32;
                for j in (t + 1 - w)..(t + 1) {
                    sm = sm + (c[j] / c[j - 1]).ln();
                }
                let mean = sm / (w as f32);
                let mut var = 0.0f32;
                for j in (t + 1 - w)..(t + 1) {
                    let d = (c[j] / c[j - 1]).ln() - mean;
                    var = var + d * d;
                }
                var = var / (w as f32);
                if var <= 1.0e-12f32 {
                    held0 = 0.0f32;
                } else {
                    held0 = (w as f32) / var;
                }
            }
            v0 = held0;
        } else if formula == 1219u32 {
            // information gain between consecutive binned returns, window >= 20, bins >= 4
            let mut w = period as usize;
            if w < 20 {
                w = 20;
            }
            let mut bn = p2 as usize;
            if bn < 4 {
                bn = 4;
            }
            let mut clip = a;
            if clip < 1.0e-6f32 {
                clip = 1.0e-6f32;
            }
            let mn = -clip;
            let mx = clip;
            if t >= w {
                // scr layout: [0..bn) hy, [bn..bn + bn*bn) hxy (index by * bn + bx)
                for b in 0..(bn + bn * bn) {
                    scr[b] = 0.0f32;
                }
                for j in (t + 2 - w)..(t + 1) {
                    let y = (c[j] / c[j - 1]).ln();
                    let x = (c[j - 1] / c[j - 2]).ln();
                    let mut bx = (((x.min(mx).max(mn) - mn) / (mx - mn)) * (bn as f32)) as usize;
                    let mut by = (((y.min(mx).max(mn) - mn) / (mx - mn)) * (bn as f32)) as usize;
                    if bx > bn - 1 {
                        bx = bn - 1;
                    }
                    if by > bn - 1 {
                        by = bn - 1;
                    }
                    scr[by] = scr[by] + 1.0f32;
                    scr[bn + by * bn + bx] = scr[bn + by * bn + bx] + 1.0f32;
                }
                let total = (w - 1) as f32;
                let mut hy = 0.0f32;
                for b in 0..bn {
                    if scr[b] > 0.0f32 {
                        let pb = scr[b] / total;
                        hy = hy - pb * pb.ln();
                    }
                }
                let mut hyx = 0.0f32;
                for by in 0..bn {
                    let mut rs = 0.0f32;
                    for bx in 0..bn {
                        rs = rs + scr[bn + by * bn + bx];
                    }
                    if rs > 0.0f32 {
                        for bx in 0..bn {
                            let cn = scr[bn + by * bn + bx];
                            if cn > 0.0f32 {
                                let pb = cn / rs;
                                hyx = hyx - (rs / total) * pb * pb.ln();
                            }
                        }
                    }
                }
                held0 = (hy - hyx).max(0.0f32);
            }
            v0 = held0;
        } else if formula == 1220u32 || formula == 1221u32 {
            // approximate (1220) / sample (1221) entropy of the last `period` closes
            let pr = period as usize;
            let m = p2 as usize;
            let mut n = t + 1;
            if n > pr {
                n = pr;
            }
            let base = t + 1 - n;
            let mut need = m + 1;
            if need < 3usize {
                need = 3usize;
            }
            if need > pr {
                need = pr;
            }
            if n >= need {
                if n >= 2usize && rstate < 0.01f32 {
                    rstate = 0.15f32 * win_std(c, base, n);
                }
                if n >= m + 1 {
                    if formula == 1220u32 {
                        held0 = apen_phi(c, base, n, m, rstate) - apen_phi(c, base, n, m + 1, rstate);
                    } else {
                        let m0 = sampen_count(c, base, n, m, rstate);
                        let m1 = sampen_count(c, base, n, m + 1, rstate);
                        if m0 > 0.0f32 && m1 > 0.0f32 {
                            held0 = -(m1 / m0).ln();
                        } else if m0 > 0.0f32 {
                            held0 = 3.0f32;
                        } else {
                            held0 = 1.5f32;
                        }
                    }
                }
            }
            v0 = held0;
        } else if formula == 1222u32 {
            // permutation entropy (normalised): period window, p2 order, p3 delay; ordinal
            // patterns are stored as index permutations in scr[pat * order ..]
            let pr = period as usize;
            let od = p2 as usize;
            let mut dl = p3 as usize;
            if dl < 1 {
                dl = 1;
            }
            let mut n = t + 1;
            if n > pr {
                n = pr;
            }
            let base = t + 1 - n;
            let mut need = od + (od - 1) * dl;
            if need < 10usize {
                need = 10usize;
            }
            if n >= need && n >= od && n >= od * dl {
                let np = n - od * dl + 1;
                for i in 0..np {
                    for j in 0..od {
                        let vj = c[base + i + j * dl];
                        let mut rank = 0usize;
                        for k in 0..od {
                            let vk = c[base + i + k * dl];
                            if vk < vj || (vk == vj && k < j) {
                                rank = rank + 1;
                            }
                        }
                        scr[i * od + rank] = j as f32;
                    }
                }
                let mut ent = 0.0f32;
                for i in 0..np {
                    let mut first = true;
                    let mut cn = 0.0f32;
                    for j in 0..np {
                        let mut eq = true;
                        for k in 0..od {
                            if scr[i * od + k] != scr[j * od + k] {
                                eq = false;
                            }
                        }
                        if eq {
                            cn = cn + 1.0f32;
                            if j < i {
                                first = false;
                            }
                        }
                    }
                    if first {
                        let pb = cn / (np as f32);
                        ent = ent - pb * pb.ln();
                    }
                }
                let mut fact = 1.0f32;
                for k in 2..(od + 1) {
                    fact = fact * (k as f32);
                }
                let mxe = fact.ln();
                if mxe > 0.0f32 {
                    held0 = (ent / mxe).max(0.0f32).min(1.0f32);
                } else {
                    held0 = 0.0f32;
                }
            }
            v0 = held0;
        } else if formula == 1223u32 {
            // conditional entropy H(y | x) of consecutive binned returns
            let mut w = period as usize;
            if w < 20 {
                w = 20;
            }
            let mut bn = p2 as usize;
            if bn < 4 {
                bn = 4;
            }
            let mut clip = a;
            if clip < 1.0e-6f32 {
                clip = 1.0e-6f32;
            }
            if t >= w {
                // scr: [0..bn) px, [bn..bn + bn*bn) joint[y * bn + x]
                for b in 0..(bn + bn * bn) {
                    scr[b] = 0.0f32;
                }
                for j in (t + 2 - w)..(t + 1) {
                    let y = (c[j] / c[j - 1]).ln();
                    let x = (c[j - 1] / c[j - 2]).ln();
                    let yy = y.max(-clip).min(clip);
                    let xx = x.max(-clip).min(clip);
                    let mut by = ((((yy + clip) / (2.0f32 * clip)) * (bn as f32)).floor()) as usize;
                    let mut bx = ((((xx + clip) / (2.0f32 * clip)) * (bn as f32)).floor()) as usize;
                    if by > bn - 1 {
                        by = bn - 1;
                    }
                    if bx > bn - 1 {
                        bx = bn - 1;
                    }
                    scr[bx] = scr[bx] + 1.0f32;
                    scr[bn + by * bn + bx] = scr[bn + by * bn + bx] + 1.0f32;
                }
                let total = (w - 1) as f32;
                let mut hh = 0.0f32;
                for bx in 0..bn {
                    if scr[bx] > 0.0f32 {
                        let px = scr[bx] / total;
                        let mut hyx = 0.0f32;
                        for by in 0..bn {
                            let cn = scr[bn + by * bn + bx];
                            if cn > 0.0f32 {
                                let pyx = cn / scr[bx];
                                hyx = hyx - pyx * pyx.ln();
                            }
                        }
                        hh = hh + px * hyx;
                    }
                }
                held0 = hh.max(0.0f32);
            }
            v0 = held0;
        } else if formula == 1224u32 || formula == 1225u32 {
            // Jensen-Shannon (1224) / Kullback-Leibler (1225) divergence between the older and
            // newer half of the last `w` returns; scr[0..bn) = p, scr[bn..2bn) = q
            let mut w = period as usize;
            if w < 10 {
                w = 10;
            }
            if formula == 1225u32 {
                w = w | 1usize;
            }
            let mut bn = p2 as usize;
            if bn < 8 {
                bn = 8;
            }
            let mut clip = a;
            if clip < 1.0e-6f32 {
                clip = 1.0e-6f32;
            }
            if t >= w {
                let half = w / 2;
                for b in 0..(2 * bn) {
                    scr[b] = 1.0e-12f32;
                }
                for i in 0..w {
                    let j = t + 1 - w + i;
                    let r = (c[j] / c[j - 1]).ln();
                    let rr = r.max(-clip).min(clip);
                    let x = (rr + clip) / (2.0f32 * clip);
                    let mut bi = (x * (bn as f32)).floor() as usize;
                    if bi > bn - 1 {
                        bi = bn - 1;
                    }
                    if i < half {
                        scr[bi] = scr[bi] + 1.0f32;
                    } else {
                        scr[bn + bi] = scr[bn + bi] + 1.0f32;
                    }
                }
                let mut ps = 0.0f32;
                let mut qs = 0.0f32;
                for b in 0..bn {
                    ps = ps + scr[b];
                    qs = qs + scr[bn + b];
                }
                let mut kl_pm = 0.0f32;
                let mut kl_qm = 0.0f32;
                let mut kl_pq = 0.0f32;
                for b in 0..bn {
                    let pi = scr[b] / ps;
                    let qi = scr[bn + b] / qs;
                    let mi = 0.5f32 * (pi + qi);
                    if pi > 0.0f32 && mi > 0.0f32 {
                        kl_pm = kl_pm + pi * (pi / mi).ln();
                    }
                    if qi > 0.0f32 && mi > 0.0f32 {
                        kl_qm = kl_qm + qi * (qi / mi).ln();
                    }
                    if pi > 0.0f32 && qi > 0.0f32 {
                        kl_pq = kl_pq + pi * (pi / qi).ln();
                    }
                }
                if formula == 1224u32 {
                    held0 = (0.5f32 * kl_pm + 0.5f32 * kl_qm).max(0.0f32);
                } else {
                    held0 = kl_pq.max(0.0f32);
                }
            }
            v0 = held0;
        } else if formula == 1226u32 {
            // Lempel-Ziv complexity of the sign bits, read in ring-storage order like the CPU
            let mut w = period as usize;
            if w < 32 {
                w = 32;
            }
            if t >= w {
                for i in 0..w {
                    let m = (t - 1) - ((t - 1 + w - i) % w);
                    let j = m + 1;
                    if c[j] - c[j - 1] >= 0.0f32 {
                        scr[i] = 1.0f32;
                    } else {
                        scr[i] = 0.0f32;
                    }
                }
                let mut ii = 0usize;
                let mut cc = 1u32;
                let mut ll = 1usize;
                let mut kk = 1usize;
                let mut kmax = 1usize;
                let mut done = false;
                while !done && ii + ll <= w {
                    if ii + kk >= w || scr[ii + kk] != scr[kk - 1] {
                        if kk > kmax {
                            kmax = kk;
                        }
                        ii = ii + 1;
                        if ii == kk {
                            cc = cc + 1u32;
                            kk = kk + kmax;
                            if kk >= w {
                                done = true;
                            } else {
                                ii = 0;
                                ll = 1;
                                kmax = 1;
                            }
                        } else {
                            kk = 1;
                            ll = 1;
                        }
                    } else {
                        kk = kk + 1;
                        ll = ll + 1;
                    }
                }
                let nf = w as f32;
                let norm = nf / nf.ln().max(1.0001f32);
                held0 = (cc as f32) / norm;
            }
            v0 = held0;
        } else if formula == 1227u32 {
            // Ljung-Box Q over the ring-ordered returns (period window >= 20, fast lags)
            let mut w = period as usize;
            if w < 20 {
                w = 20;
            }
            let mut lg = p2 as usize;
            if lg < 1 {
                lg = 1;
            }
            if lg > w / 2 {
                lg = w / 2;
            }
            if t >= w {
                let nf = w as f32;
                let mut mean = 0.0f32;
                for i in 0..w {
                    let r = ring_ret(c, t, w, i);
                    scr[i] = r;
                    mean = mean + r;
                }
                mean = mean / nf;
                let mut var = 0.0f32;
                for i in 0..w {
                    scr[i] = scr[i] - mean;
                    var = var + scr[i] * scr[i];
                }
                var = var / nf;
                if var <= 1.0e-12f32 {
                    held0 = 0.0f32;
                } else {
                    let mut q = 0.0f32;
                    for hh in 1..(lg + 1) {
                        let mut num = 0.0f32;
                        for k in hh..w {
                            num = num + scr[k] * scr[k - hh];
                        }
                        let rho = num / (nf * var);
                        q = q + rho * rho / (nf - (hh as f32));
                    }
                    held0 = q * nf * (nf + 2.0f32);
                }
            }
            v0 = held0;
        } else if formula == 1228u32 {
            // partial autocorrelation at `fast` lag by Durbin-Levinson on ring-ordered returns
            let mut w = period as usize;
            if w < 10 {
                w = 10;
            }
            let mut kl = p2 as usize;
            if kl < 1 {
                kl = 1;
            }
            if kl > w - 2 {
                kl = w - 2;
            }
            if t >= w {
                let nf = w as f32;
                let mut mean = 0.0f32;
                for i in 0..w {
                    let r = ring_ret(c, t, w, i);
                    scr[i] = r;
                    mean = mean + r;
                }
                mean = mean / nf;
                for i in 0..w {
                    scr[i] = scr[i] - mean;
                }
                // autoc at [w .. w + kl], phi at [w + kl + 1 ..], tmp after
                let ao = w;
                let po = w + kl + 1;
                let to = w + 2 * (kl + 1);
                for lag in 0..(kl + 1) {
                    let mut sm = 0.0f32;
                    for k in lag..w {
                        sm = sm + scr[k] * scr[k - lag];
                    }
                    scr[ao + lag] = sm / nf;
                }
                if scr[ao].abs() < 1.0e-12f32 {
                    held0 = 0.0f32;
                } else {
                    for j in 0..(kl + 1) {
                        scr[po + j] = 0.0f32;
                    }
                    let mut vv = scr[ao];
                    for m in 1..(kl + 1) {
                        let mut sum = 0.0f32;
                        for j in 1..m {
                            sum = sum + scr[po + j] * scr[ao + m - j];
                        }
                        let km = (scr[ao + m] - sum) / vv.max(1.0e-12f32);
                        for j in 1..m {
                            scr[to + j] = scr[po + j] - km * scr[po + m - j];
                        }
                        for j in 1..m {
                            scr[po + j] = scr[to + j];
                        }
                        scr[po + m] = km;
                        vv = vv * (1.0f32 - km * km);
                    }
                    held0 = scr[po + kl].max(-1.0f32).min(1.0f32);
                }
            }
            v0 = held0;
        } else if formula == 1229u32 {
            // half life of mean reversion from an AR(1) fit of consecutive returns
            let mut w = period as usize;
            if w < 20 {
                w = 20;
            }
            if t >= w {
                let mut sx = 0.0f32;
                let mut sy = 0.0f32;
                let mut sxx = 0.0f32;
                let mut sxy = 0.0f32;
                let mut cn = 0.0f32;
                for i in 1..w {
                    let j = t + 1 - w + i;
                    let y = (c[j] / c[j - 1]).ln();
                    let x = (c[j - 1] / c[j - 2]).ln();
                    sx = sx + x;
                    sy = sy + y;
                    sxx = sxx + x * x;
                    sxy = sxy + x * y;
                    cn = cn + 1.0f32;
                }
                let den = cn * sxx - sx * sx;
                let mut phi = 0.0f32;
                if den.abs() > 1.0e-12f32 {
                    phi = (cn * sxy - sx * sy) / den;
                }
                if phi > 0.0f32 && phi < 1.0f32 {
                    held0 = (-(0.6931472f32) / phi.ln()).max(0.0f32);
                } else {
                    // +inf on the CPU: patched on the host after read-back
                    held0 = 3.4e38f32;
                }
            }
            v0 = held0;
        } else if formula == 1230u32 {
            // residual stationarity: var(close - running SMA) / var(close) over ring-ordered closes
            let mut w = period as usize;
            if w < 20 {
                w = 20;
            }
            if t + 1 >= w {
                let nf = w as f32;
                let mut mean = 0.0f32;
                for i in 0..w {
                    let b = t - ((t + w - i) % w);
                    scr[i] = c[b];
                    mean = mean + c[b];
                }
                mean = mean / nf;
                let mut run = 0.0f32;
                let mut vr = 0.0f32;
                let mut vc = 0.0f32;
                for i in 0..w {
                    run = run + scr[i];
                    let sma = run / ((i + 1) as f32);
                    let r = scr[i] - sma;
                    vr = vr + r * r;
                    vc = vc + (scr[i] - mean) * (scr[i] - mean);
                }
                if vc > 0.0f32 {
                    held0 = (vr / nf) / (vc / nf);
                } else {
                    held0 = 0.0f32;
                }
            }
            v0 = held0;
        } else if formula == 1231u32 {
            // Hurst exponent by rescaled range over the last min(period, 512) closes
            let mut pr = period as usize;
            if pr > 512 {
                pr = 512;
            }
            let mut minp = pr / 2;
            if minp < 10usize {
                minp = 10usize;
            }
            if pr > 0usize && minp > pr - 1 {
                minp = pr - 1;
            }
            let mut n = t + 1;
            if n > pr {
                n = pr;
            }
            if n >= 2usize && n - 1 >= minp {
                let base = t + 1 - n;
                let rn = n - 1;
                let mut sm = 0.0f32;
                for j in 1..n {
                    sm = sm + (c[base + j] / c[base + j - 1]).ln();
                }
                let mean = sm / (rn as f32);
                let mut cs = 0.0f32;
                let mut mx = cs - 3.4e38f32;
                let mut mn = cs + 3.4e38f32;
                let mut var = 0.0f32;
                for j in 1..n {
                    let d = (c[base + j] / c[base + j - 1]).ln() - mean;
                    cs = cs + d;
                    if cs > mx {
                        mx = cs;
                    }
                    if cs < mn {
                        mn = cs;
                    }
                    var = var + d * d;
                }
                let sd = (var / (rn as f32)).sqrt();
                if sd > 1.0e-10f32 {
                    let rs = (mx - mn) / sd;
                    let nf = rn as f32;
                    if rs > 0.0f32 && nf > 2.0f32 {
                        held0 = (rs.ln() / (nf / 2.0f32).ln()).max(0.0f32).min(1.0f32);
                    }
                }
            }
            if hset == 0u32 {
                held0 = 0.5f32;
                hset = 1u32;
            }
            v0 = held0;
        } else if formula == 1232u32 {
            // Higuchi fractal dimension (period window, p2 max k)
            let mut pr = period as usize;
            if pr > 512 {
                pr = 512;
            }
            let mut mk = p2 as usize;
            if mk > pr / 4 {
                mk = pr / 4;
            }
            if mk < 2usize {
                mk = 2usize;
            }
            if hset == 0u32 {
                held0 = 1.5f32;
                hset = 1u32;
            }
            if t + 1 >= pr {
                let base = t + 1 - pr;
                let n = pr;
                let mut cntk = 0.0f32;
                let mut sx = 0.0f32;
                let mut sy = 0.0f32;
                let mut sxy = 0.0f32;
                let mut sxx = 0.0f32;
                for k in 1..(mk + 1) {
                    let mut lk = 0.0f32;
                    if k < n {
                        let m = (n - 1) / k;
                        let mut total = 0.0f32;
                        for i in 1..(k + 1) {
                            let mut length = 0.0f32;
                            let mut count = 0u32;
                            for j in 1..(m + 1) {
                                let i1 = i + (j - 1) * k - 1;
                                let i2 = i + j * k - 1;
                                if i1 < n && i2 < n {
                                    length = length + (c[base + i2] - c[base + i1]).abs();
                                    count = count + 1u32;
                                }
                            }
                            if count > 0u32 {
                                length = length * ((n as f32) - 1.0f32) / ((count as f32) * (k as f32));
                                total = total + length;
                            }
                        }
                        lk = total / (k as f32);
                    }
                    if lk > 0.0f32 {
                        let x = (k as f32).ln();
                        let y = lk.ln();
                        cntk = cntk + 1.0f32;
                        sx = sx + x;
                        sy = sy + y;
                        sxy = sxy + x * y;
                        sxx = sxx + x * x;
                    }
                }
                if cntk >= 3.0f32 {
                    let den = cntk * sxx - sx * sx;
                    let mut slope = 0.0f32;
                    if den.abs() >= 1.0e-10f32 {
                        slope = (cntk * sxy - sx * sy) / den;
                    }
                    held0 = (2.0f32 - slope).max(1.0f32).min(2.0f32);
                }
            }
            v0 = held0;
        } else if formula == 1233u32 {
            // detrended fluctuation analysis alpha over four scales (period, p2, p3, p4)
            let n0 = period as usize;
            let n1 = p2 as usize;
            let n2 = p3 as usize;
            let n3 = p4 as usize;
            if t == 0usize {
                f0 = c[0];
                f1 = c[0];
                f2 = c[0];
                f3 = c[0];
            }
            if t >= n0 {
                f0 = dfa_f(c, t, n0, f0, scr);
            }
            if t >= n1 {
                f1 = dfa_f(c, t, n1, f1, scr);
            }
            if t >= n2 {
                f2 = dfa_f(c, t, n2, f2, scr);
            }
            if t >= n3 {
                f3 = dfa_f(c, t, n3, f3, scr);
            }
            if t + 1 >= n0 && t + 1 >= n1 && t + 1 >= n2 && t + 1 >= n3 {
                let mut cn = 0.0f32;
                let mut sx = 0.0f32;
                let mut sy = 0.0f32;
                let mut sxy = 0.0f32;
                let mut sxx = 0.0f32;
                if f0 > 0.0f32 {
                    let x = (n0 as f32).ln();
                    let y = f0.ln();
                    cn = cn + 1.0f32;
                    sx = sx + x;
                    sy = sy + y;
                    sxy = sxy + x * y;
                    sxx = sxx + x * x;
                }
                if f1 > 0.0f32 {
                    let x = (n1 as f32).ln();
                    let y = f1.ln();
                    cn = cn + 1.0f32;
                    sx = sx + x;
                    sy = sy + y;
                    sxy = sxy + x * y;
                    sxx = sxx + x * x;
                }
                if f2 > 0.0f32 {
                    let x = (n2 as f32).ln();
                    let y = f2.ln();
                    cn = cn + 1.0f32;
                    sx = sx + x;
                    sy = sy + y;
                    sxy = sxy + x * y;
                    sxx = sxx + x * x;
                }
                if f3 > 0.0f32 {
                    let x = (n3 as f32).ln();
                    let y = f3.ln();
                    cn = cn + 1.0f32;
                    sx = sx + x;
                    sy = sy + y;
                    sxy = sxy + x * y;
                    sxx = sxx + x * x;
                }
                if cn >= 2.0f32 {
                    let den = cn * sxx - sx * sx;
                    if den.abs() > 1.0e-9f32 {
                        held0 = (cn * sxy - sx * sy) / den;
                    } else {
                        held0 = 0.5f32;
                    }
                }
            }
            v0 = held0;
        } else if formula == 1234u32 || formula == 1235u32 {
            // percentile rank of the inner series (`volume` slot) over the last `flag` values
            let w = flag as usize;
            if hset == 0u32 {
                held0 = 0.5f32;
                hset = 1u32;
            }
            if t + 1 >= w {
                let mut cnt = 0.0f32;
                for j in (t + 1 - w)..(t + 1) {
                    if v[j] <= v[t] {
                        cnt = cnt + 1.0f32;
                    }
                }
                held0 = cnt / (w as f32);
            }
            v0 = held0;
        } else if formula == 1236u32 {
            // KPSS level-stationarity proxy with a Newey-West long-run variance
            let mut n = period as usize;
            if n < 50 {
                n = 50;
            }
            if n > 1024 {
                n = 1024;
            }
            if t + 1 >= n {
                let base = t + 1 - n;
                let nf = n as f32;
                let mut sm = 0.0f32;
                for i in 0..n {
                    sm = sm + c[base + i];
                }
                let mean = sm / nf;
                let mut run = 0.0f32;
                let mut s2 = 0.0f32;
                let mut em = 0.0f32;
                for i in 0..n {
                    let e = c[base + i] - mean;
                    scr[i] = e;
                    run = run + e;
                    s2 = s2 + run * run;
                    em = em + e;
                }
                em = em / nf;
                // Newey-West: bandwidth floor(4 (n/100)^(2/9)), at least 1, at most n - 1
                let bwf = 4.0f32 * ((nf / 100.0f32).ln() * 0.22222222f32).exp();
                let mut l = bwf.floor() as usize;
                if l < 1usize {
                    l = 1usize;
                }
                if l > n - 1 {
                    l = n - 1;
                }
                let mut g0 = 0.0f32;
                for i in 0..n {
                    let d = scr[i] - em;
                    g0 = g0 + d * d;
                }
                let mut lrv = g0 / nf;
                for lag in 1..(l + 1) {
                    let mut g = 0.0f32;
                    for k in lag..n {
                        g = g + (scr[k] - em) * (scr[k - lag] - em);
                    }
                    g = g / nf;
                    let wgt = 1.0f32 - (lag as f32) / ((l as f32) + 1.0f32);
                    lrv = lrv + 2.0f32 * wgt * g;
                }
                lrv = lrv.max(0.0f32).max(1.0e-12f32);
                held0 = (s2 / (nf * nf * lrv)).max(0.0f32);
            }
            v0 = held0;
        } else if formula == 1237u32 {
            // KPSS trend-stationarity proxy: residuals of y ~ t, cheap long-run variance
            let mut n = period as usize;
            if n < 50 {
                n = 50;
            }
            if n > 2048 {
                n = 2048;
            }
            if t + 1 >= n {
                let base = t + 1 - n;
                let nf = n as f32;
                let mut sx = 0.0f32;
                let mut sy = 0.0f32;
                let mut sxx = 0.0f32;
                let mut sxy = 0.0f32;
                for i in 0..n {
                    let tt = (i + 1) as f32;
                    let y = c[base + i];
                    sx = sx + tt;
                    sy = sy + y;
                    sxx = sxx + tt * tt;
                    sxy = sxy + tt * y;
                }
                let den = nf * sxx - sx * sx;
                if den.abs() >= 1.0e-12f32 {
                    let ai = (sxx * sy - sx * sxy) / den;
                    let bi = (nf * sxy - sx * sy) / den;
                    let mut run = 0.0f32;
                    let mut s2 = 0.0f32;
                    let mut em = 0.0f32;
                    for i in 0..n {
                        let e = c[base + i] - (ai + bi * ((i + 1) as f32));
                        scr[i] = e;
                        run = run + e;
                        s2 = s2 + run * run;
                        em = em + e;
                    }
                    em = em / nf;
                    let mut var = 0.0f32;
                    let mut cov1 = 0.0f32;
                    for i in 0..n {
                        let d = scr[i] - em;
                        var = var + d * d;
                    }
                    for i in 1..n {
                        cov1 = cov1 + (scr[i] - em) * (scr[i - 1] - em);
                    }
                    var = var / nf;
                    cov1 = cov1 / ((n - 1) as f32);
                    let lrv = (var + 2.0f32 * cov1.max(0.0f32)).max(1.0e-12f32);
                    held0 = (s2 / (nf * lrv)).max(0.0f32);
                } else {
                    held0 = 0.0f32;
                }
            }
            v0 = held0;
        } else if formula == 1238u32 {
            // ADF proxy t-statistic (constant, Schwert lag rule) over the last `period` closes
            let mut n = period as usize;
            if n < 20 {
                n = 20;
            }
            if t + 1 >= n {
                let base = t + 1 - n;
                let m = n - 1;
                let pf = 12.0f32 * ((((n as f32) / 100.0f32).ln()) * 0.25f32).exp();
                let mut p = pf.floor() as usize;
                if p > m / 3 {
                    p = m / 3;
                }
                if m > p + 2 {
                    let k = 2 + p;
                    let rows = m - p;
                    if rows > k {
                        let xo = 0usize;
                        let yo = rows * k;
                        let wo = yo + rows;
                        for r in 0..rows {
                            let tt = p + r;
                            scr[xo + r * k] = 1.0f32;
                            scr[xo + r * k + 1] = c[base + tt];
                            for i in 1..(p + 1) {
                                scr[xo + r * k + 1 + i] = c[base + tt - i + 1] - c[base + tt - i];
                            }
                            scr[yo + r] = c[base + tt + 1] - c[base + tt];
                        }
                        let ok = ols_gamma(scr, xo, yo, rows, k, wo, 1usize);
                        if ok > 0.5f32 {
                            held0 = scr[wo + 1];
                            held1 = 1.0f32 + scr[wo];
                        }
                    }
                }
            }
            v0 = held0;
            v1 = held1;
        } else if formula == 1239u32 {
            // Phillips-Perron proxy z-statistic
            let mut n = period as usize;
            if n < 20 {
                n = 20;
            }
            if t + 1 >= n {
                let base = t + 1 - n;
                let rows = n - 1;
                let rf = rows as f32;
                let mut xb = 0.0f32;
                let mut yb = 0.0f32;
                for i in 0..rows {
                    xb = xb + c[base + i];
                    yb = yb + c[base + i + 1];
                }
                xb = xb / rf;
                yb = yb / rf;
                let mut sxx = 0.0f32;
                let mut sxy = 0.0f32;
                for i in 0..rows {
                    let dx = c[base + i] - xb;
                    sxx = sxx + dx * dx;
                    sxy = sxy + dx * (c[base + i + 1] - yb);
                }
                held0 = 0.0f32;
                if sxx > 0.0f32 {
                    let rho = sxy / sxx;
                    let b0 = yb - rho * xb;
                    let mut sse = 0.0f32;
                    for i in 0..rows {
                        let e = c[base + i + 1] - (b0 + rho * c[base + i]);
                        scr[i] = e;
                        sse = sse + e * e;
                    }
                    let dof = (rows - 2) as f32;
                    let sigma2 = sse / dof;
                    let se = (sigma2 / sxx).sqrt();
                    if se > 0.0f32 {
                        let trho = (rho - 1.0f32) / se;
                        let lam2 = nw_lrv(scr, 0usize, rows);
                        let s2h = sse / rf;
                        if s2h <= 0.0f32 || lam2 <= 0.0f32 {
                            held0 = trho;
                        } else {
                            let lam = lam2.sqrt();
                            let sg = s2h.sqrt();
                            let sh = sigma2.sqrt();
                            let z = (sg / lam) * trho - (lam2 - s2h) * (rf * se) / (2.0f32 * lam * sh);
                            if z == z {
                                held0 = z;
                            } else {
                                held0 = trho;
                            }
                        }
                    }
                }
            }
            v0 = held0;
        } else if formula == 1240u32 {
            // Zivot-Andrews proxy: minimum break-dummy ADF t-statistic (one lag)
            let mut n = period as usize;
            if n < 30 {
                n = 30;
            }
            if t + 1 >= n {
                let base = t + 1 - n;
                let m = n - 1;
                let pl = 1usize;
                let lo = ((0.15f32 * (n as f32)) as usize);
                let hi = ((0.85f32 * (n as f32)) as usize);
                let mut min_t = 3.4e38f32;
                let mut any = 0.0f32;
                if m > pl + 6 {
                    let k = 4 + pl;
                    let rows = m - pl;
                    if rows > k + 2 {
                        for tb in lo..(hi + 1) {
                            let xo = 0usize;
                            let yo = rows * k;
                            let wo = yo + rows;
                            for r in 0..rows {
                                let tt = pl + r;
                                scr[xo + r * k] = 1.0f32;
                                scr[xo + r * k + 1] = (tt + 1) as f32;
                                if tt + 1 > tb {
                                    scr[xo + r * k + 2] = 1.0f32;
                                } else {
                                    scr[xo + r * k + 2] = 0.0f32;
                                }
                                scr[xo + r * k + 3] = c[base + tt];
                                scr[xo + r * k + 4] = c[base + tt] - c[base + tt - 1];
                                scr[yo + r] = c[base + tt + 1] - c[base + tt];
                            }
                            let ok = ols_gamma(scr, xo, yo, rows, k, wo, 3usize);
                            if ok > 0.5f32 {
                                if scr[wo + 1] < min_t {
                                    min_t = scr[wo + 1];
                                }
                                any = 1.0f32;
                            }
                        }
                    }
                }
                if any > 0.5f32 {
                    held0 = min_t;
                } else {
                    held0 = 0.0f32;
                }
            }
            v0 = held0;
        } else if formula == 1241u32 {
            // Engle-Granger trend proxy: AR(1) t-statistic of the detrended closes
            let mut n = period as usize;
            if n < 32 {
                n = 32;
            }
            if t + 1 >= n {
                let base = t + 1 - n;
                let cnt = n as f32;
                let mut st = 0.0f32;
                let mut stt = 0.0f32;
                let mut sy = 0.0f32;
                let mut syt = 0.0f32;
                for i in 0..n {
                    let y = c[base + i];
                    let tt = i as f32;
                    st = st + tt;
                    stt = stt + tt * tt;
                    sy = sy + y;
                    syt = syt + y * tt;
                }
                let den = cnt * stt - st * st;
                let mut cc = 0.0f32;
                if den.abs() > 1.0e-12f32 {
                    cc = (cnt * syt - st * sy) / den;
                }
                let a0 = (sy - cc * st) / cnt;
                let mut rx = 0.0f32;
                let mut ry = 0.0f32;
                let mut rxx = 0.0f32;
                let mut rxy = 0.0f32;
                let rc = (n - 1) as f32;
                for i in 0..n {
                    scr[i] = c[base + i] - (a0 + cc * (i as f32));
                }
                for i in 1..n {
                    let y = scr[i];
                    let x = scr[i - 1];
                    rx = rx + x;
                    ry = ry + y;
                    rxx = rxx + x * x;
                    rxy = rxy + x * y;
                }
                let d = rc * rxx - rx * rx;
                let mut phi = 0.0f32;
                if d.abs() > 1.0e-12f32 {
                    phi = (rc * rxy - rx * ry) / d;
                }
                let mut se = 0.0f32;
                for i in 1..n {
                    let e = scr[i] - phi * scr[i - 1];
                    se = se + e * e;
                }
                let var = se.max(1.0e-12f32) / (rc - 1.0f32).max(1.0f32);
                let sphi = (var / (rxx - rx * rx / rc).max(1.0e-12f32)).sqrt();
                if sphi > 0.0f32 {
                    held0 = (phi - 1.0f32) / sphi;
                } else {
                    held0 = 0.0f32;
                }
            }
            v0 = held0;
        } else if formula >= 1242u32 && formula <= 1244u32 {
            // AR(1) fit of (close - SMA) over the last `w` residuals; `v` carries the SMA series
            let mut w = period as usize;
            if formula == 1242u32 {
                if w < 32 {
                    w = 32;
                }
            } else if w < 20 {
                w = 20;
            }
            if t + 1 >= w {
                let mut sx = 0.0f32;
                let mut sy = 0.0f32;
                let mut sxx = 0.0f32;
                let mut sxy = 0.0f32;
                let mut cn = 0.0f32;
                for i in 1..w {
                    let j = t + 1 - w + i;
                    let y = c[j] - v[j];
                    let x = c[j - 1] - v[j - 1];
                    sx = sx + x;
                    sy = sy + y;
                    sxx = sxx + x * x;
                    sxy = sxy + x * y;
                    cn = cn + 1.0f32;
                }
                let den = cn * sxx - sx * sx;
                let mut phi = 0.0f32;
                if den.abs() > 1.0e-12f32 {
                    phi = (cn * sxy - sx * sy) / den;
                }
                let mut ss = 0.0f32;
                for i in 1..w {
                    let j = t + 1 - w + i;
                    let y = c[j] - v[j];
                    let x = c[j - 1] - v[j - 1];
                    let e = y - phi * x;
                    ss = ss + e * e;
                }
                let var = ss.max(1.0e-12f32) / (cn - 1.0f32).max(1.0f32);
                let sadj = (sxx - sx * sx / cn).max(1.0e-12f32);
                let sphi = (var / sadj).sqrt();
                held0 = phi;
                if sphi > 0.0f32 {
                    held1 = (phi - 1.0f32) / sphi;
                } else {
                    held1 = 0.0f32;
                }
            }
            if formula == 1244u32 {
                v0 = held1;
            } else {
                v0 = held0;
                v1 = held1;
            }
        } else if formula == 1245u32 {
            // ADF/KPSS composite: o = EG-ADF t-stat, h = KPSS level, l = KPSS trend (0 when unused)
            let adf_score = 1.0f32 / (1.0f32 + o[t].abs().exp());
            let kscore = 1.0f32 / (1.0f32 + h[t].max(l[t]));
            v0 = 0.5f32 * (adf_score + kscore);
        } else if formula == 1246u32 {
            // z-score of the inner KPSS series (`v`) over the last `p2` (>= 20) values
            let mut w = p2 as usize;
            if w < 20 {
                w = 20;
            }
            if t + 1 >= w {
                let mut mean = 0.0f32;
                for j in (t + 1 - w)..(t + 1) {
                    mean = mean + v[j];
                }
                mean = mean / (w as f32);
                let mut var = 0.0f32;
                for j in (t + 1 - w)..(t + 1) {
                    let d = v[j] - mean;
                    var = var + d * d;
                }
                let sdv = (var / (w as f32)).sqrt().max(1.0e-9f32);
                held0 = (v[t] - mean) / sdv;
            }
            v0 = held0;
        } else if formula == 1247u32 || formula == 1248u32 {
            // ARCH-LM R^2 (1247) / chi-square p-value (1248) of squared returns on their lags
            let mut n = period as usize;
            if n < 50 {
                n = 50;
            }
            if n > 1024 {
                n = 1024;
            }
            let mut l = p2 as usize;
            if l < 1usize {
                l = 1usize;
            }
            if l > 10usize {
                l = 10usize;
            }
            if formula == 1248u32 && hset == 0u32 {
                held0 = 1.0f32;
                hset = 1u32;
            }
            if t >= n {
                let rows = n - l - 1;
                let k = l + 1;
                for i in 0..n {
                    let j = t + 1 - n + i;
                    let r = (c[j] / c[j - 1]).ln();
                    scr[i] = r * r;
                }
                let xo = n;
                let yo = xo + rows * k;
                let wo = yo + rows;
                let mut ym = 0.0f32;
                for r in 0..rows {
                    scr[xo + r * k] = 1.0f32;
                    for jj in 1..(l + 1) {
                        scr[xo + r * k + jj] = scr[r + l - jj];
                    }
                    scr[yo + r] = scr[r + l];
                    ym = ym + scr[r + l];
                }
                ym = ym / (rows as f32);
                let mut tss = 0.0f32;
                for r in 0..rows {
                    let d = scr[yo + r] - ym;
                    tss = tss + d * d;
                }
                let okk = ols_gamma(scr, xo, yo, rows, k, wo, 1usize);
                let sse = scr[wo + 2];
                let mut r2 = 0.0f32;
                if sse >= 0.0f32 && tss > 1.0e-12f32 {
                    r2 = (1.0f32 - sse / tss).max(0.0f32).min(1.0f32);
                }
                if okk < 0.0f32 {
                    r2 = 0.0f32;
                }
                if formula == 1247u32 {
                    held0 = r2;
                } else {
                    held0 = chi2_sf_k((rows as f32) * r2, l as f32).max(0.0f32).min(1.0f32);
                }
            }
            v0 = held0;
        } else if formula == 1249u32 {
            // inverse-fisher RSI: tanh((rsi - 0.5) * 2) with `v` = RSI series
            let x = (v[t] - 0.5f32) * 2.0f32;
            let ax = x.abs();
            let th = 1.0f32 - 2.0f32 / ((2.0f32 * ax).exp() + 1.0f32);
            if x < 0.0f32 {
                v0 = 0.0f32 - th;
            } else {
                v0 = th;
            }
        } else if formula == 1250u32 {
            // z-score of the RSI series (`v`) over min(t + 1, p2 >= 2) values
            let mut w = p2 as usize;
            if w < 2 {
                w = 2;
            }
            let mut n = t + 1;
            if n > w {
                n = w;
            }
            if n >= 2usize {
                let mut sm = 0.0f32;
                let mut sq = 0.0f32;
                for j in (t + 1 - n)..(t + 1) {
                    sm = sm + v[j];
                    sq = sq + v[j] * v[j];
                }
                let nf = n as f32;
                let mean = sm / nf;
                let var = sq / nf - mean * mean;
                if var > 0.0f32 {
                    let sd = var.sqrt();
                    if sd > 1.0e-12f32 {
                        v0 = (v[t] - mean) / sd;
                    }
                }
            }
        } else if formula == 1251u32 {
            // VHF with a smoothed |diff| (`v`) denominator
            let pr = period as usize;
            if pr >= 1usize && t >= pr {
                let mut mx = c[t + 1 - pr];
                let mut mn = c[t + 1 - pr];
                for j in (t + 1 - pr)..(t + 1) {
                    if c[j] > mx {
                        mx = c[j];
                    }
                    if c[j] < mn {
                        mn = c[j];
                    }
                }
                if v[t].abs() >= 1.0e-12f32 {
                    v0 = (mx - mn) / ((pr as f32) * v[t]);
                }
            }
        } else if formula == 1390u32 {
            // stage: |close - previous close|
            if t > 0usize {
                v0 = (c[t] - c[t - 1]).abs();
            }
        } else if formula == 1391u32 {
            // stage: raw stochastic of the RSI series (`v`), RSI ready from bar `period`,
            // stochastic window `p2`
            let pr = period as usize;
            let sp = p2 as usize;
            if sp >= 1usize && t + 1 >= pr + sp {
                let mut hi = v[t + 1 - sp];
                let mut lo = v[t + 1 - sp];
                for j in (t + 1 - sp)..(t + 1) {
                    if v[j] > hi {
                        hi = v[j];
                    }
                    if v[j] < lo {
                        lo = v[j];
                    }
                }
                if (hi - lo).abs() < 1.0e-12f32 {
                    v0 = 50.0f32;
                } else {
                    v0 = ((v[t] - lo) / (hi - lo)) * 100.0f32;
                }
            }
        } else if formula == 1392u32 {
            // stage: Bressert raw %K over min(t + 1, period) bars
            let mut w = period as usize;
            if w < 1 {
                w = 1;
            }
            let mut st = 0usize;
            if t + 1 > w {
                st = t + 1 - w;
            }
            let mut hi = h[st];
            let mut lo = l[st];
            for j in st..(t + 1) {
                if h[j] > hi {
                    hi = h[j];
                }
                if l[j] < lo {
                    lo = l[j];
                }
            }
            let range = (hi - lo).abs().max(1.0e-12f32);
            v0 = (c[t] - lo) / range * 100.0f32;
        } else if formula == 1393u32 {
            // stage: SMI numerator (close - mid) and range
            v0 = c[t] - 0.5f32 * (h[t] + l[t]);
            v1 = (h[t] - l[t]).max(1.0e-12f32);
        } else if formula == 1394u32 {
            // stage: SMI value from the double-smoothed numerator (o) and range (v)
            v0 = 100.0f32 * o[t] / (0.5f32 * v[t]).max(1.0e-12f32);
        } else if formula == 1395u32 {
            // stage: 50 + 50 tanh(v)
            let ax = v[t].abs();
            let th = 1.0f32 - 2.0f32 / ((2.0f32 * ax).exp() + 1.0f32);
            if v[t] < 0.0f32 {
                v0 = 50.0f32 - 50.0f32 * th;
            } else {
                v0 = 50.0f32 + 50.0f32 * th;
            }
        } else if formula == 1258u32 {
            // negative / positive volume index (close, volume lanes), both start at 1000
            if t == 0usize {
                held0 = 1000.0f32;
                held1 = 1000.0f32;
            } else {
                let mut pc = 0.0f32;
                if c[t - 1].abs() > 1.0e-12f32 {
                    pc = (c[t] - c[t - 1]) / c[t - 1];
                }
                if v[t] < v[t - 1] {
                    held0 = held0 * (1.0f32 + pc);
                }
                if v[t] > v[t - 1] {
                    held1 = held1 * (1.0f32 + pc);
                }
            }
            v0 = held0;
            v1 = held1;
        } else if formula == 1259u32 {
            // GMMA compression score over 12 smoothed series (scr[s * n + t], s = 0..6 fast, 6..12 slow)
            let nn = c.len();
            if t >= 59usize {
                let mut fmin = scr[t];
                let mut fmax = scr[t];
                let mut fsum = 0.0f32;
                for s in 0..6 {
                    let x = scr[s * nn + t];
                    if x < fmin {
                        fmin = x;
                    }
                    if x > fmax {
                        fmax = x;
                    }
                    fsum = fsum + x;
                }
                let mut smin = scr[6 * nn + t];
                let mut smax = scr[6 * nn + t];
                let mut ssum = 0.0f32;
                for s in 6..12 {
                    let x = scr[s * nn + t];
                    if x < smin {
                        smin = x;
                    }
                    if x > smax {
                        smax = x;
                    }
                    ssum = ssum + x;
                }
                let inter = (fsum / 6.0f32 - ssum / 6.0f32).abs();
                held0 = inter / (1.0e-9f32 + (fmax - fmin) + (smax - smin));
            }
            v0 = held0;
        } else if formula == 1260u32 {
            // robust EWMAC: tanh((fast - slow) / (1.4826 MAD)); o = slow, v = fast, window in p4
            let mut w = p4 as usize;
            if w < 15 {
                w = 15;
            }
            if t + 1 >= w {
                let med = kth_in(c, t + 1 - w, w, (w / 2) as u32);
                let scale = (1.4826f32 * kth_dev(c, t + 1 - w, w, (w / 2) as u32, med)).max(1.0e-9f32);
                let x = (v[t] - o[t]) / scale;
                let ax = x.abs();
                let th = 1.0f32 - 2.0f32 / ((2.0f32 * ax).exp() + 1.0f32);
                if x < 0.0f32 {
                    v0 = 0.0f32 - th;
                } else {
                    v0 = th;
                }
            }
        } else if formula == 1261u32 {
            // TDI basis: mean of the RSI series (`v`) over min(t + 1, period p2) bars
            let mut bp = p2 as usize;
            if bp < 1 {
                bp = 1;
            }
            let mut n = t + 1;
            if n > bp {
                n = bp;
            }
            let mut sm = 0.0f32;
            for j in (t + 1 - n)..(t + 1) {
                sm = sm + v[j];
            }
            v0 = sm / (n as f32);
        } else if formula == 1263u32 {
            // VIDYA: v = CMO series (percent), ready from bar `period`
            let pr = period as usize;
            let alpha = 2.0f32 / ((pr as f32) + 1.0f32);
            let pct = (v[t] / 100.0f32).abs();
            if held1 > 0.5f32 {
                held0 = (alpha * pct) * c[t] + (1.0f32 - alpha * pct) * held0;
            }
            if held1 < 0.5f32 && t >= pr {
                held1 = 1.0f32;
                held0 = c[t];
            }
            v0 = held0;
        } else if formula == 1396u32 {
            // stage: weighted blend a * o + (1 - a) * v
            v0 = a * o[t] + (1.0f32 - a) * v[t];
        } else if formula == 1397u32 {
            // stage: Ehlers rocket regular RSI of the smoothed price (`v`), Wilder seeded by the mean
            let pr = period as usize;
            v0 = 50.0f32;
            if t >= pr && pr >= 1usize {
                let gt = (v[t] - v[t - 1]).max(0.0f32);
                let lt = (v[t - 1] - v[t]).max(0.0f32);
                if held0 == 0.0f32 && held1 == 0.0f32 {
                    let mut sg = 0.0f32;
                    let mut sl = 0.0f32;
                    for j in (t + 1 - pr)..(t + 1) {
                        sg = sg + (v[j] - v[j - 1]).max(0.0f32);
                        sl = sl + (v[j - 1] - v[j]).max(0.0f32);
                    }
                    held0 = sg / (pr as f32);
                    held1 = sl / (pr as f32);
                } else {
                    let al = 1.0f32 / (pr as f32);
                    held0 = al * gt + (1.0f32 - al) * held0;
                    held1 = al * lt + (1.0f32 - al) * held1;
                }
                if held1 == 0.0f32 {
                    v0 = 100.0f32;
                } else {
                    v0 = 100.0f32 - 100.0f32 / (1.0f32 + held0 / held1);
                }
            }
        } else if formula == 1398u32 {
            // stage: first difference of `v`
            if t > 0usize {
                v0 = v[t] - v[t - 1];
            }
        } else if formula == 1399u32 {
            // stage: rocket RSI = clamp(rsi + a * smoothed momentum * 10, 0, 100) from bar 2
            v0 = v[t];
            if t >= 2usize {
                v0 = (v[t] + a * o[t] * 10.0f32).max(0.0f32).min(100.0f32);
            }
        } else if formula == 1265u32 {
            // Kaufman adaptive MA with its ten read-outs; period = ER window (2..=200), fast
            // 1..=50, slow fast+1..=200
            let mut erp = period as usize;
            if erp < 2 {
                erp = 2;
            }
            if erp > 200 {
                erp = 200;
            }
            let mut fp = p2 as usize;
            if fp < 1 {
                fp = 1;
            }
            if fp > 50 {
                fp = 50;
            }
            let mut sp = p3 as usize;
            if sp < fp + 1 {
                sp = fp + 1;
            }
            if sp > 200 {
                sp = 200;
            }
            let fast_sc = 2.0f32 / ((fp as f32) + 1.0f32);
            let slow_sc = 2.0f32 / ((sp as f32) + 1.0f32);
            let sc_diff = fast_sc - slow_sc;
            let mut len = t + 1;
            if len > 200usize {
                len = 200usize;
            }
            if len > erp {
                // ER of the current bar
                let dirc = (c[t] - c[t - erp]).abs();
                let mut volc = 0.0f32;
                for i in (t - erp + 1)..(t + 1) {
                    volc = volc + (c[i] - c[i - 1]).abs();
                }
                let mut er = 0.0f32;
                if volc > 0.0f32 {
                    er = (dirc / volc).max(0.0f32).min(1.0f32);
                }
                // statistics over the last (up to 100) ERs
                let nb = t - erp + 1;
                let mut cntr = nb;
                if cntr > 100usize {
                    cntr = 100usize;
                }
                let mut sm = 0.0f32;
                for k in 0..cntr {
                    let j = t - k;
                    let dj = (c[j] - c[j - erp]).abs();
                    let mut vj = 0.0f32;
                    for i in (j - erp + 1)..(j + 1) {
                        vj = vj + (c[i] - c[i - 1]).abs();
                    }
                    let mut ej = 0.0f32;
                    if vj > 0.0f32 {
                        ej = (dj / vj).max(0.0f32).min(1.0f32);
                    }
                    sm = sm + ej;
                }
                let avg = sm / (cntr as f32);
                let mut vs = 0.0f32;
                for k in 0..cntr {
                    let j = t - k;
                    let dj = (c[j] - c[j - erp]).abs();
                    let mut vj = 0.0f32;
                    for i in (j - erp + 1)..(j + 1) {
                        vj = vj + (c[i] - c[i - 1]).abs();
                    }
                    let mut ej = 0.0f32;
                    if vj > 0.0f32 {
                        ej = (dj / vj).max(0.0f32).min(1.0f32);
                    }
                    vs = vs + (ej - avg) * (ej - avg);
                }
                let evar = vs / (cntr as f32);
                // trend consistency from the last ten directions
                let mut tc = 0.0f32;
                if nb >= 10usize {
                    let mut up = 0.0f32;
                    let mut dn = 0.0f32;
                    for i in 1..10 {
                        let newer = (c[t + 1 - i] - c[t + 1 - i - erp]).abs();
                        let older = (c[t - i] - c[t - i - erp]).abs();
                        if newer < older {
                            up = up + 1.0f32;
                        } else if newer > older {
                            dn = dn + 1.0f32;
                        }
                    }
                    if up + dn > 0.0f32 {
                        tc = up.max(dn) / (up + dn);
                    }
                }
                let sc0 = er * sc_diff + slow_sc;
                let sc = sc0 * sc0;
                let mut ap = sp as f32;
                if sc > 0.0f32 {
                    ap = 2.0f32 / sc - 1.0f32;
                }
                if held0 == 0.0f32 {
                    held0 = c[t];
                } else {
                    held0 = held0 + sc * (c[t] - held0);
                }
                v0 = held0;
                v1 = er;
                v2 = ap;
                v3 = evar;
                v4 = tc;
                v5 = sc;
                v6 = avg;
                if held0 != 0.0f32 {
                    v7 = ((c[t] - held0) / held0) * 100.0f32;
                }
                v8 = volc;
                v9 = dirc;
            } else {
                held0 = c[t];
                v0 = held0;
            }
        } else if formula == 1389u32 {
            // stage: v[t] - v[t - 1] with a zero previous value on the first bar
            if t > 0usize {
                v0 = v[t] - v[t - 1];
            } else {
                v0 = v[t];
            }
        } else if formula == 1267u32 {
            // relative volume `[rvol, percentile]` over min(t + 1, period) bars of the volume lane
            let mut w = period as usize;
            if w < 1 {
                w = 1;
            }
            let mut n = t + 1;
            if n > w {
                n = w;
            }
            let mut sm = 0.0f32;
            for j in (t + 1 - n)..(t + 1) {
                sm = sm + v[j].max(0.0f32);
            }
            let cur = v[t].max(0.0f32);
            let mut mean = 0.0f32;
            if sm > 0.0f32 {
                mean = sm / (n as f32);
            }
            if mean > 0.0f32 {
                v0 = cur / mean;
            }
            let mut cnt = 0.0f32;
            for j in (t + 1 - n)..(t + 1) {
                if v[j].max(0.0f32) <= cur {
                    cnt = cnt + 1.0f32;
                }
            }
            v1 = cnt / (n as f32);
        } else if formula == 1268u32 {
            // cumulative (session) VWAP: held0 = sum pv, held1 = sum v
            if v[t] > 0.0f32 {
                held0 = held0 + (h[t] + l[t] + c[t]) / 3.0f32 * v[t];
                held1 = held1 + v[t];
            }
            if held1 > 1.0e-9f32 {
                v0 = held0 / held1;
            } else {
                v0 = c[t];
            }
        } else if formula == 1269u32 {
            // FRAMA: flag 0 standard, 1 improved, 2 dynamic, 3 robust fractal dimension
            let pr = period as usize;
            if t + 1 < pr {
                v0 = c[t];
            } else {
                let base = t + 1 - pr;
                let n = pr as f32;
                let mut mxh = h[base];
                let mut mnl = l[base];
                let mut tv = 0.0f32;
                let mut mean = 0.0f32;
                for j in 0..pr {
                    if h[base + j] > mxh {
                        mxh = h[base + j];
                    }
                    if l[base + j] < mnl {
                        mnl = l[base + j];
                    }
                    if j > 0usize {
                        tv = tv + (c[base + j] - c[base + j - 1]).abs();
                    }
                    mean = mean + c[base + j];
                }
                mean = mean / n;
                let mut vol = 0.5f32;
                if pr >= 2usize {
                    let mut var = 0.0f32;
                    for j in 0..pr {
                        var = var + (c[base + j] - mean) * (c[base + j] - mean);
                    }
                    var = var / n;
                    vol = (var.sqrt() / mean.abs().max(1.0e-12f32) * 100.0f32).max(0.0f32).min(1.0f32);
                }
                let mut dim = 1.0f32;
                if flag == 2u32 {
                    let mut ad = pr;
                    if vol > 0.7f32 {
                        ad = ((pr as f32) * 0.7f32) as usize;
                    } else if vol < 0.3f32 {
                        ad = ((pr as f32) * 1.3f32) as usize;
                    }
                    if ad > pr {
                        ad = pr;
                    }
                    if ad < 2usize {
                        ad = 2usize;
                    }
                    let st = base + pr - ad;
                    let mut mx2 = h[st];
                    let mut mn2 = l[st];
                    let mut tv2 = 0.0f32;
                    for j in st..(base + pr) {
                        if h[j] > mx2 {
                            mx2 = h[j];
                        }
                        if l[j] < mn2 {
                            mn2 = l[j];
                        }
                        if j > st {
                            tv2 = tv2 + (c[j] - c[j - 1]).abs();
                        }
                    }
                    let n1 = (mx2 - mn2) / (ad as f32);
                    let n2 = tv2 / ((ad - 1) as f32);
                    if n1 > 1.0e-12f32 && n2 > 1.0e-12f32 {
                        dim = ((n2 / n1).ln() / 0.6931472f32).max(1.0f32).min(2.0f32);
                    }
                } else {
                    let n1 = (mxh - mnl) / n;
                    let mut n2 = tv / (n - 1.0f32);
                    if flag == 3u32 {
                        // median absolute change
                        let m = pr - 1;
                        for j in 0..m {
                            scr[j] = (c[base + j + 1] - c[base + j]).abs();
                        }
                        let mid = m / 2;
                        let mut lo_v = scr[0];
                        let mut hi_v = scr[0];
                        for j in 0..m {
                            let x = scr[j];
                            let mut lt = 0usize;
                            let mut le = 0usize;
                            for q in 0..m {
                                if scr[q] < x {
                                    lt = lt + 1;
                                }
                                if scr[q] <= x {
                                    le = le + 1;
                                }
                            }
                            if lt <= mid && mid < le {
                                hi_v = x;
                            }
                            if mid >= 1usize && lt <= mid - 1 && mid - 1 < le {
                                lo_v = x;
                            }
                        }
                        n2 = hi_v;
                        if m % 2usize == 0usize && m > 0usize {
                            n2 = (lo_v + hi_v) / 2.0f32;
                        }
                    }
                    if n1 > 1.0e-12f32 && n2 > 1.0e-12f32 {
                        dim = ((n2 / n1).ln() / 0.6931472f32).max(1.0f32).min(2.0f32);
                    }
                    if flag == 1u32 {
                        dim = (dim * (1.0f32 + (vol - 0.5f32).max(0.0f32) * 0.2f32)).max(1.0f32).min(2.0f32);
                    }
                }
                // smoothed dimension state in held1 (EMA for improved / dynamic)
                if flag == 1u32 || flag == 2u32 {
                    held1 = 0.2f32 * dim + 0.8f32 * held1;
                } else {
                    held1 = dim;
                }
                let alpha = (0.0f32 - 4.6f32 * (held1 - 1.0f32)).exp().max(0.01f32).min(1.0f32);
                if held2 < 0.5f32 {
                    held0 = c[t];
                    held2 = 1.0f32;
                } else {
                    held0 = alpha * c[t] + (1.0f32 - alpha) * held0;
                }
                v0 = held0;
            }
        } else if formula == 1270u32 {
            // ROC percentile `[roc, percentile]`: period = ROC lag, p2 = window
            let pr = period as usize;
            let mut w = p2 as usize;
            if w < 1 {
                w = 1;
            }
            let mut roc = 0.0f32;
            if t >= pr && c[t - pr].abs() > 1.0e-12f32 {
                roc = (c[t] - c[t - pr]) / c[t - pr];
            }
            v0 = roc;
            let idx = t + 1;
            if idx > pr {
                let mut start = 0usize;
                if idx > w {
                    start = idx - w;
                }
                let mut cn = 0.0f32;
                let mut cl = 0.0f32;
                for j in start..idx {
                    if j > pr {
                        let mut rj = 0.0f32;
                        if c[j - pr].abs() > 1.0e-12f32 {
                            rj = (c[j] - c[j - pr]) / c[j - pr];
                        }
                        cn = cn + 1.0f32;
                        if rj <= roc {
                            cl = cl + 1.0f32;
                        }
                    }
                }
                if cn > 0.0f32 {
                    v1 = cl / cn;
                }
            }
        } else if formula == 1387u32 {
            // stage: rolling std of log returns over min(t, period >= 2) returns
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            let mut n = t;
            if n > w {
                n = w;
            }
            if t >= 1usize && n >= 2usize {
                let mut sm = 0.0f32;
                let mut sq = 0.0f32;
                for j in (t + 1 - n)..(t + 1) {
                    let r = (c[j] / c[j - 1]).ln();
                    sm = sm + r;
                    sq = sq + r * r;
                }
                let nf = n as f32;
                let mean = sm / nf;
                let var = sq / nf - mean * mean;
                if var > 0.0f32 {
                    v0 = var.sqrt();
                }
            }
        } else if formula == 1386u32 {
            // stage: z-score of `v` over min(t, p2 >= 2) values, first value at bar 1
            let mut w = p2 as usize;
            if w < 2 {
                w = 2;
            }
            let mut m = t;
            if m > w {
                m = w;
            }
            if m >= 2usize {
                let mut sm = 0.0f32;
                let mut sq = 0.0f32;
                for j in (t + 1 - m)..(t + 1) {
                    sm = sm + v[j];
                    sq = sq + v[j] * v[j];
                }
                let mf = m as f32;
                let mean = sm / mf;
                let var = sq / mf - mean * mean;
                if var > 0.0f32 {
                    let sd = var.sqrt();
                    if sd > 1.0e-12f32 {
                        v0 = (v[t] - mean) / sd;
                    }
                }
            }
        } else if formula == 1272u32 {
            // Ehlers sinewave: phase += tanh(diff) * alpha, value = sin(phase); alpha in `a`
            let al = a.max(0.0f32).min(1.0f32);
            let mut prev = 0.0f32;
            if t > 0usize {
                prev = c[t - 1];
            }
            let d = c[t] - prev;
            let ad = d.abs();
            let th = 1.0f32 - 2.0f32 / ((2.0f32 * ad).exp() + 1.0f32);
            if d < 0.0f32 {
                held0 = held0 - th * al;
            } else {
                held0 = held0 + th * al;
            }
            v0 = held0.sin();
        } else if formula == 1273u32 {
            // Ehlers super smoother (value); the cutoff period (float) is in `a`
            let pi = 3.1415927f32;
            let cw = (2.0f32 * pi / a).cos();
            let alpha = 1.0f32 - cw;
            let b2 = 0.0f32 - alpha * alpha / 4.0f32;
            let prev_price = if t >= 1usize { c[t - 1] } else { c[t] };
            let mut p1 = c[t];
            let mut p2v = c[t];
            if t >= 1usize {
                p1 = held0;
            }
            if t >= 2usize {
                p2v = held1;
            } else if t == 1usize {
                p2v = c[t];
            }
            let ss = 0.5f32 * c[t] + 0.5f32 * prev_price + cw * p1 + b2 * p2v;
            held1 = held0;
            held0 = ss;
            v0 = ss;
        } else if formula == 1274u32 {
            // Ehlers fractal adaptive MA (fama): period capped by the 512-sample history
            let pr = period as usize;
            let mut minp = pr / 4;
            if minp < 2usize {
                minp = 2usize;
            }
            let maxp = pr * 2;
            let mut len = t + 1;
            if len > 512usize {
                len = 512usize;
            }
            let mut alpha = 2.0f32 / ((pr as f32) + 1.0f32);
            if len >= pr {
                let st = t + 1 - pr;
                let mut tl = 0.0f32;
                for i in 1..pr {
                    tl = tl + (c[st + i] - c[st + i - 1]).abs();
                }
                let dd = (c[t] - c[st]).abs();
                let mut fd = 1.5f32;
                if dd != 0.0f32 && tl != 0.0f32 {
                    fd = ((tl / dd).ln() / (pr as f32).ln()).max(1.0f32).min(2.0f32);
                }
                let mut er = 0.0f32;
                if tl != 0.0f32 {
                    er = dd / tl;
                }
                let sp = er / fd;
                let ap = ((maxp as f32) - sp * ((maxp - minp) as f32)).max(minp as f32).min(maxp as f32);
                alpha = (2.0f32 / (ap + 1.0f32)).max(0.0f32).min(1.0f32);
            }
            if held2 < 0.5f32 {
                held0 = c[t];
                held2 = 1.0f32;
            } else {
                held0 = alpha * c[t] + (1.0f32 - alpha) * held0;
            }
            v0 = held0;
        } else if formula == 1275u32 {
            // Hilbert transform `[amplitude, phase, frequency]`: Hamming-windowed Hilbert kernel
            // at the newest sample; period = window (16..=256), a = sampling rate
            let mut ws = period as usize;
            if ws < 16 {
                ws = 16;
            }
            if ws > 256 {
                ws = 256;
            }
            let pi = 3.1415927f32;
            if t + 1 >= 2 * ws {
                let mut len = t + 1;
                if len > 512usize {
                    len = 512usize;
                }
                let i = len - 1;
                let mut wstart = 0usize;
                if i > ws / 2 {
                    wstart = i - ws / 2;
                }
                let nw = len - wstart;
                let base = t + 1 - len;
                let mut hv = 0.0f32;
                let mut wsum = 0.0f32;
                for j in wstart..len {
                    if i != j {
                        let tau = (i as f32) - (j as f32);
                        let wgt = 1.0f32 / (pi * tau);
                        let mut hc = 1.0f32;
                        if nw > 1usize {
                            hc = 0.54f32 - 0.46f32 * (2.0f32 * pi * ((j - wstart) as f32) / ((nw - 1) as f32)).cos();
                        }
                        hv = hv + wgt * c[base + j] * hc;
                        wsum = wsum + wgt.abs();
                    }
                }
                if wsum > 0.0f32 {
                    hv = hv / wsum;
                }
                let re = c[t];
                let amp = (re * re + hv * hv).sqrt();
                let ph = hv.atan2(re);
                let mut fq = 0.0f32;
                if cnt_a >= 2usize {
                    let mut pd = ph - held0;
                    for _ in 0..4 {
                        if pd > pi {
                            pd = pd - 2.0f32 * pi;
                        }
                        if pd < 0.0f32 - pi {
                            pd = pd + 2.0f32 * pi;
                        }
                    }
                    let raw = pd * a / (2.0f32 * pi);
                    if cnt_a < 5usize {
                        fq = raw;
                    } else {
                        let mut sm5 = 0.0f32;
                        for q in 0..5 {
                            sm5 = sm5 + scr[q];
                        }
                        fq = 0.7f32 * (sm5 / 5.0f32) + 0.3f32 * raw;
                    }
                }
                scr[cnt_a % 5usize] = fq;
                cnt_a = cnt_a + 1usize;
                held0 = ph;
                v0 = amp;
                v1 = ph;
                v2 = fq;
            }
        } else if formula == 1276u32 {
            // Hilbert dominant cycle period (smoothed); a = min period, b = max period
            let pi = 3.1415927f32;
            if t >= 7usize {
                let ic = (c[t - 3] + c[t - 2] + c[t - 1] + c[t]) / 4.0f32;
                let qc = (c[t - 6] + 2.0f32 * c[t - 4] + 3.0f32 * c[t - 2] + 3.0f32 * c[t]) / 9.0f32;
                let mut phase = 0.0f32;
                if ic != 0.0f32 {
                    phase = (qc / ic).atan();
                } else if qc > 0.0f32 {
                    phase = pi / 2.0f32;
                } else if qc < 0.0f32 {
                    phase = 0.0f32 - pi / 2.0f32;
                }
                let mut lastp = held1;
                if cnt_a == 0usize {
                    lastp = 0.0f32;
                }
                let mut dp = phase - lastp;
                if dp < 0.0f32 - pi {
                    dp = dp + 2.0f32 * pi;
                } else if dp > pi {
                    dp = dp - 2.0f32 * pi;
                }
                let mut ip = held0;
                if dp.abs() > 0.01f32 {
                    ip = (2.0f32 * pi / dp.abs()).max(a).min(b);
                }
                held0 = 0.2f32 * ip + 0.8f32 * held0;
                held1 = phase;
                cnt_a = cnt_a + 1usize;
            }
            v0 = held0;
        } else if formula == 1277u32 {
            // MESA adaptive MA (EMA 6 / 12 quadrature stage, EMA 10 period smoother):
            // a = min period, b = max period.
            // scr: 0 fast ema, 1 slow ema, 2 i(t-1), 3 i(t-2), 4 q(t-1), 5 period ema, 6 current period, 7 mama, 8 period-ema seeded
            let pi = 3.1415927f32;
            let minp = a;
            let maxp = b;
            if t >= 6usize {
                if t == 6usize {
                    scr[0] = c[t];
                    scr[1] = c[t];
                    scr[2] = 0.0f32;
                    scr[3] = 0.0f32;
                    scr[4] = 0.0f32;
                    scr[5] = 0.0f32;
                    scr[6] = 0.5f32 * (minp + maxp);
                    scr[7] = 0.0f32;
                    scr[8] = 0.0f32;
                } else {
                    scr[0] = (2.0f32 / 7.0f32) * c[t] + (1.0f32 - 2.0f32 / 7.0f32) * scr[0];
                    scr[1] = (2.0f32 / 13.0f32) * c[t] + (1.0f32 - 2.0f32 / 13.0f32) * scr[1];
                }
                let ic = scr[0] - scr[1];
                let kk = t - 6;
                let mut qc = ic;
                if kk >= 2usize {
                    qc = (ic + scr[3]) / 2.0f32;
                }
                if kk >= 1usize {
                    let mut phc = 0.0f32;
                    if ic != 0.0f32 {
                        phc = (qc / ic).atan();
                    }
                    let mut php = 0.0f32;
                    if scr[2] != 0.0f32 {
                        php = (scr[4] / scr[2]).atan();
                    }
                    let mut dp = phc - php;
                    if dp < 0.0f32 - pi {
                        dp = dp + 2.0f32 * pi;
                    } else if dp > pi {
                        dp = dp - 2.0f32 * pi;
                    }
                    let mut ip = scr[6];
                    if dp.abs() > 0.01f32 {
                        ip = (2.0f32 * pi / dp.abs()).max(minp).min(maxp);
                    }
                    if scr[8] < 0.5f32 {
                        scr[5] = ip;
                        scr[8] = 1.0f32;
                    } else {
                        scr[5] = (2.0f32 / 11.0f32) * ip + (1.0f32 - 2.0f32 / 11.0f32) * scr[5];
                    }
                    scr[6] = scr[5].max(minp).min(maxp);
                }
                scr[3] = scr[2];
                scr[2] = ic;
                scr[4] = qc;
                let al = (2.0f32 / (scr[6] + 1.0f32)).max(0.0f32).min(1.0f32);
                if t == 6usize {
                    scr[7] = c[t];
                } else {
                    scr[7] = al * c[t] + (1.0f32 - al) * scr[7];
                }
                v0 = scr[7];
            }
        } else if formula == 1278u32 {
            // Ehlers zero-lag EMA: ema + (ema - ema[lag]); lag = p2 or (period-1)/2 (min 1); the ring keeps 16 emas
            let pr = period as f32;
            let al = 2.0f32 / (pr + 1.0f32);
            let mut lag = p2 as usize;
            if lag == 0usize {
                lag = ((period as usize) - 1) / 2;
                if lag < 1usize {
                    lag = 1usize;
                }
            }
            if t == 0usize {
                held0 = c[t];
            } else {
                held0 = al * c[t] + (1.0f32 - al) * held0;
            }
            scr[t % 16usize] = held0;
            let mut lagged = held0;
            let mut have = t + 1;
            if have > 16usize {
                have = 16usize;
            }
            if have > lag {
                lagged = scr[(t - lag) % 16usize];
            }
            v0 = held0 + (held0 - lagged);
        } else if formula == 1279u32 {
            // Ultimate oscillator over rescanned buying-pressure / true-range windows (period, p2, p3)
            let p3u = p3 as usize;
            if t >= p3u && t >= 1usize {
                let mut sb1 = 0.0f32;
                let mut st1 = 0.0f32;
                let mut sb2 = 0.0f32;
                let mut st2 = 0.0f32;
                let mut sb3 = 0.0f32;
                let mut st3 = 0.0f32;
                for k in 0..p3u {
                    let q = t - k;
                    let pc = c[q - 1];
                    let lo = l[q].min(pc);
                    let bp = c[q] - lo;
                    let tr = (h[q] - l[q]).max((h[q] - pc).abs()).max((l[q] - pc).abs());
                    sb3 = sb3 + bp;
                    st3 = st3 + tr;
                    if k < p2 as usize {
                        sb2 = sb2 + bp;
                        st2 = st2 + tr;
                    }
                    if k < period as usize {
                        sb1 = sb1 + bp;
                        st1 = st1 + tr;
                    }
                }
                let mut a1 = 0.0f32;
                let mut a2 = 0.0f32;
                let mut a3 = 0.0f32;
                if st1.abs() >= 1.0e-12f32 {
                    a1 = sb1 / st1;
                }
                if st2.abs() >= 1.0e-12f32 {
                    a2 = sb2 / st2;
                }
                if st3.abs() >= 1.0e-12f32 {
                    a3 = sb3 / st3;
                }
                v0 = (100.0f32 * (4.0f32 * a1 + 2.0f32 * a2 + a3) / 7.0f32).max(0.0f32).min(100.0f32);
            }
        } else if formula == 1280u32 {
            // NR range: the exposed output is the clamped bar range
            v0 = (h[t] - l[t]).max(0.0f32);
        } else if formula == 1281u32 {
            // Ehlers instantaneous trendline (alpha in `a`), live from the 8th bar
            if t >= 7usize {
                let a2 = a * a;
                let mut prev = c[t];
                if cnt_a > 0usize {
                    prev = held0;
                }
                let tl = (a - a2 / 4.0f32) * c[t] + (a2 / 2.0f32) * c[t - 1] - (a - 3.0f32 * a2 / 4.0f32) * prev;
                held0 = tl;
                cnt_a = cnt_a + 1usize;
            }
            v0 = held0;
        } else if formula == 1282u32 {
            // Parkinson estimator: sqrt of the running (non-windowed) mean of ln(H/L)^2 / (4 ln 2)
            let mut hl = 1.0e-12f32;
            if l[t] > 0.0f32 {
                hl = (h[t] / l[t]).max(1.0e-12f32);
            }
            let lg = hl.ln();
            held0 = held0 + lg * lg / (4.0f32 * 0.6931472f32);
            v0 = (held0 / ((t + 1) as f32)).sqrt();
        } else if formula == 1283u32 {
            // range compression burst: 1.0 burst, 0.5 compressed, 0.0 otherwise; window = max(period, 2)
            let mut w = period as usize;
            if w < 2usize {
                w = 2usize;
            }
            if t + 1 >= w {
                let mut mn = 3.4e38f32;
                let mut mx = 0.0f32;
                for k in 0..w {
                    let r = (h[t - k] - l[t - k]).max(0.0f32);
                    mn = mn.min(r);
                    mx = mx.max(r);
                }
                let cur = (h[t] - l[t]).max(0.0f32);
                let prv = (h[t - 1] - l[t - 1]).max(0.0f32);
                let thr = mn + 0.1f32 * (mx - mn).max(1.0e-12f32);
                let was = held0 > 0.5f32;
                let comp = cur <= thr;
                let burst = was && cur > prv && cur > thr;
                if comp {
                    held0 = 1.0f32;
                } else {
                    held0 = 0.0f32;
                }
                if burst {
                    v0 = 1.0f32;
                } else if comp {
                    v0 = 0.5f32;
                }
            }
        } else if formula == 1284u32 {
            // price/volume coherence proxy: |corr| of log price / log volume changes over the window;
            // lanes: c = close, v = volume
            let mut w = period as usize;
            if w < 2usize {
                w = 2usize;
            }
            let mut n = t + 1;
            if n > w {
                n = w;
            }
            if n >= 2usize {
                let mut sx = 0.0f32;
                let mut sy = 0.0f32;
                let mut sxx = 0.0f32;
                let mut syy = 0.0f32;
                let mut sxy = 0.0f32;
                for k in 0..n {
                    let q = t - k;
                    let mut x = 0.0f32;
                    let mut y = 0.0f32;
                    if q >= 1usize {
                        x = (c[q] / c[q - 1].max(1.0e-12f32)).ln();
                        y = (v[q] / v[q - 1].max(1.0e-9f32)).ln();
                        if v[q - 1].max(1.0e-9f32) <= 0.0f32 {
                            y = 0.0f32;
                        }
                    }
                    sx = sx + x;
                    sy = sy + y;
                    sxx = sxx + x * x;
                    syy = syy + y * y;
                    sxy = sxy + x * y;
                }
                let nn = n as f32;
                let num = nn * sxy - sx * sy;
                let den = ((nn * sxx - sx * sx) * (nn * syy - sy * sy)).max(1.0e-24f32).sqrt();
                if den > 0.0f32 {
                    held0 = (num / den).abs().min(1.0f32);
                } else {
                    held0 = 0.0f32;
                }
            }
            v0 = held0;
        } else if formula == 1285u32 {
            // demand index (lanes o,h,l,c,v), volume mean over the last `period` bars
            let mut pr = period as usize;
            if pr < 1usize {
                pr = 1usize;
            }
            if t >= 1usize {
                let mut cntv = t + 1;
                if cntv > pr {
                    cntv = pr;
                }
                let mut vs = 0.0f32;
                for k in 0..cntv {
                    vs = vs + v[t - k];
                }
                let avg = vs / (cntv as f32);
                let mut vf = 1.0f32;
                if avg > 1.0e-10f32 {
                    vf = v[t] / avg;
                }
                let pc = c[t - 1];
                let tr = (h[t].max(pc) - l[t].min(pc)).max(1.0e-10f32);
                let bp = (h[t] - o[t]) + (c[t] - l[t]);
                let sp = (h[t] - c[t]) + (o[t] - l[t]);
                let range = h[t] - l[t];
                if range > 1.0e-10f32 {
                    let cp = (c[t] - l[t]) / range;
                    let tp = bp + sp;
                    let mut prr = 0.0f32;
                    if tp > 1.0e-10f32 {
                        prr = (bp - sp) / tp;
                    }
                    let pf = (c[t] - pc) / tr;
                    let pos = (cp - 0.5f32) * 2.0f32;
                    held0 = (pf * vf * (1.0f32 + pos) + prr * vf * 0.5f32) * 100.0f32;
                } else {
                    held0 = 0.0f32;
                }
            }
            v0 = held0;
        } else if formula == 1286u32 {
            // Kalman trend regime: z-score of the Kalman velocity (lane v) over `max(period, 20)`
            // (population sd, floor 1e-9); +1 above 1, -1 below -1
            let mut w = period as usize;
            if w < 20usize {
                w = 20usize;
            }
            if t + 1 >= w {
                let st = t + 1 - w;
                let mut sm = 0.0f32;
                for j in st..(t + 1) {
                    sm = sm + v[j];
                }
                let mean = sm / (w as f32);
                let mut ss = 0.0f32;
                for j in st..(t + 1) {
                    let dd = v[j] - mean;
                    ss = ss + dd * dd;
                }
                let sd = (ss / (w as f32)).sqrt().max(1.0e-9f32);
                let z = (v[t] - mean) / sd;
                if z > 1.0f32 {
                    v0 = 1.0f32;
                } else if z < 0.0f32 - 1.0f32 {
                    v0 = 0.0f32 - 1.0f32;
                }
            }
        } else if formula == 1287u32 {
            // AMAT: smoothed fast (lane v) / slow (lane o) series, ready once the slow smoother is
            // (index slow - 1); compares each with its value `signal` bars back (front of the deque)
            let mut rdy = 0usize;
            if p3 > 1u32 {
                rdy = (p3 as usize) - 1;
            }
            if t >= rdy {
                let mut fi = rdy;
                if t >= rdy + (p4 as usize) {
                    fi = t - (p4 as usize);
                }
                let d1 = v[t] - v[fi];
                let d2 = o[t] - o[fi];
                let up = (d1 > 0.0f32 && d2 < 0.0f32) || (d1 > 0.0f32 && d2 > 0.0f32);
                let dn = (d1 < 0.0f32 && d2 > 0.0f32) || (d1 < 0.0f32 && d2 < 0.0f32);
                if up {
                    v0 = 1.0f32;
                } else if dn {
                    v0 = 0.0f32 - 1.0f32;
                }
            }
        } else if formula == 1288u32 {
            // Elder impulse: trend smoother series in lane v; MACD 12/26/9 EMAs inline
            let mut prev = 0.0f32;
            if t > 0usize {
                prev = v[t - 1];
            }
            let slope_up = v[t] > prev + 1.0e-12f32;
            if t == 0usize {
                held0 = c[t];
                held1 = c[t];
                held2 = 0.0f32;
            } else {
                held0 = (2.0f32 / 13.0f32) * c[t] + (1.0f32 - 2.0f32 / 13.0f32) * held0;
                held1 = (2.0f32 / 27.0f32) * c[t] + (1.0f32 - 2.0f32 / 27.0f32) * held1;
                held2 = (2.0f32 / 10.0f32) * (held0 - held1) + (1.0f32 - 2.0f32 / 10.0f32) * held2;
            }
            let hist = (held0 - held1) - held2;
            let macd_up = hist > 0.0f32;
            if slope_up && macd_up {
                v0 = 1.0f32;
            } else if !slope_up && !macd_up {
                v0 = 0.0f32 - 1.0f32;
            }
        } else if formula == 1290u32 {
            // ATR-RSI: ATR in lane v, its baseline smoother in lane o; inline Wilder RSI over `period`
            let rp = period as usize;
            let mut raw = 50.0f32;
            if t >= 1usize && rp >= 1usize && t >= rp {
                let chg = c[t] - c[t - 1];
                let mut gain = 0.0f32;
                let mut loss = 0.0f32;
                if chg > 0.0f32 {
                    gain = chg;
                }
                if chg < 0.0f32 {
                    loss = 0.0f32 - chg;
                }
                if held0 == 0.0f32 && held1 == 0.0f32 {
                    let mut sg = 0.0f32;
                    let mut sl = 0.0f32;
                    for k2 in 0..rp {
                        let q = t - k2;
                        let cc2 = c[q] - c[q - 1];
                        if cc2 > 0.0f32 {
                            sg = sg + cc2;
                        }
                        if cc2 < 0.0f32 {
                            sl = sl - cc2;
                        }
                    }
                    held0 = sg / (rp as f32);
                    held1 = sl / (rp as f32);
                } else {
                    let al = 1.0f32 / (rp as f32);
                    held0 = al * gain + (1.0f32 - al) * held0;
                    held1 = al * loss + (1.0f32 - al) * held1;
                }
                if held1 == 0.0f32 {
                    raw = 100.0f32;
                } else {
                    raw = 100.0f32 - 100.0f32 / (1.0f32 + held0 / held1);
                }
            }
            let mut ratio = 1.0f32;
            if o[t] > 0.0f32 {
                ratio = v[t] / o[t];
            }
            v0 = (raw * ratio.sqrt()).max(0.0f32).min(100.0f32);
        } else if formula == 1291u32 {
            // volume-weighted RSI (lanes c = close, v = volume); period = rsi period, p2 = volume period
            let rp = period as usize;
            let vp = p2 as usize;
            v0 = 50.0f32;
            if t >= 1usize && rp >= 1usize && vp >= 1usize && t >= rp {
                let mut vcnt = t + 1;
                if vcnt > vp {
                    vcnt = vp;
                }
                let mut vsum = 0.0f32;
                for k2 in 0..vcnt {
                    vsum = vsum + v[t - k2];
                }
                // regular state: held0 / held1, volume-weighted state: f0 / f1
                if held0 == 0.0f32 && held1 == 0.0f32 {
                    let mut sg = 0.0f32;
                    let mut sl = 0.0f32;
                    let mut wg = 0.0f32;
                    let mut wl = 0.0f32;
                    for k2 in 0..rp {
                        let q = t - k2;
                        let cc2 = c[q] - c[q - 1];
                        let mut vc2 = 0usize;
                        if q + 1 > vp {
                            vc2 = q + 1 - vp;
                        }
                        let mut vs2 = 0.0f32;
                        for j in vc2..(q + 1) {
                            vs2 = vs2 + v[j];
                        }
                        let avg2 = vs2 / ((q + 1 - vc2) as f32);
                        let mut wt = 1.0f32;
                        if avg2 > 0.0f32 {
                            wt = v[q] / avg2;
                        }
                        if cc2 > 0.0f32 {
                            sg = sg + cc2;
                            wg = wg + cc2 * wt;
                        }
                        if cc2 < 0.0f32 {
                            sl = sl - cc2;
                            wl = wl - cc2 * wt;
                        }
                    }
                    held0 = sg / (rp as f32);
                    held1 = sl / (rp as f32);
                    f0 = wg / (rp as f32);
                    f1 = wl / (rp as f32);
                } else {
                    let chg = c[t] - c[t - 1];
                    let avg = vsum / (vcnt as f32);
                    let mut wt = 1.0f32;
                    if avg > 0.0f32 {
                        wt = v[t] / avg;
                    }
                    let mut gain = 0.0f32;
                    let mut loss = 0.0f32;
                    if chg > 0.0f32 {
                        gain = chg;
                    }
                    if chg < 0.0f32 {
                        loss = 0.0f32 - chg;
                    }
                    let al = 1.0f32 / (rp as f32);
                    held0 = al * gain + (1.0f32 - al) * held0;
                    held1 = al * loss + (1.0f32 - al) * held1;
                    f0 = al * gain * wt + (1.0f32 - al) * f0;
                    f1 = al * loss * wt + (1.0f32 - al) * f1;
                }
                if f1 == 0.0f32 {
                    v0 = 100.0f32;
                } else {
                    v0 = 100.0f32 - 100.0f32 / (1.0f32 + f0 / f1);
                }
                held2 = v0;
            } else if held2 > 0.0f32 {
                v0 = held2;
            }
        } else if formula == 1292u32 {
            // helper stage: |v[t] - v[t - 1]| (previous value 0 on the first bar)
            let mut pv = 0.0f32;
            if t > 0usize {
                pv = v[t - 1];
            }
            v0 = (v[t] - pv).abs();
        } else if formula == 1293u32 {
            // QQE: smoothed RSI in lane v, smoothed |delta| (ATR of RSI) in lane o; `a` = threshold
            // multiplier (<= 0 means 1.5). Trailing bands start at lower = 100, upper = 0, long.
            let mut tm = a;
            if tm <= 0.0f32 {
                tm = 1.5f32;
            }
            if t == 0usize {
                held0 = 100.0f32;
                held1 = 0.0f32;
                held2 = 1.0f32;
            }
            let bw = tm * o[t];
            let sr = v[t];
            let mut val = 0.0f32;
            if held2 > 0.5f32 {
                let nl = sr - bw;
                if nl > held0 {
                    held0 = nl;
                }
                if sr < held0 {
                    held2 = 0.0f32;
                    held1 = sr + bw;
                    val = held1;
                } else {
                    val = held0;
                }
            } else {
                let nu = sr + bw;
                if nu < held1 {
                    held1 = nu;
                }
                if sr > held1 {
                    held2 = 1.0f32;
                    held0 = sr - bw;
                    val = held0;
                } else {
                    val = held1;
                }
            }
            v0 = val;
            v1 = sr;
        } else if formula == 1294u32 {
            // squeeze momentum `[momentum, squeeze]`: lane o = BB middle, h = KC middle, l = ATR
            // (ATR fed from bar 1); period = bb period, p2 = kc period, p3 = momentum period
            let bbp = period as usize;
            let kcp = p2 as usize;
            let mp = p3 as usize;
            let mut sq = 0.0f32;
            if t + 1 >= bbp && bbp >= 1usize {
                let mut ss = 0.0f32;
                for k2 in 0..bbp {
                    let d2 = c[t - k2] - o[t];
                    ss = ss + d2 * d2;
                }
                let sd = (ss / (bbp as f32)).sqrt();
                let bbu = o[t] + 2.0f32 * sd;
                let bbl = o[t] - 2.0f32 * sd;
                if t >= kcp {
                    let kcu = h[t] + 1.5f32 * l[t];
                    let kcl = h[t] - 1.5f32 * l[t];
                    if bbu < kcu && bbl > kcl {
                        sq = 1.0f32;
                    }
                }
                // momentum: slope of (close - bb middle) over the last `mp` of those values
                if t + 2 >= bbp + mp && mp >= 2usize {
                    let nn = mp as f32;
                    let mut sy = 0.0f32;
                    let mut sxy = 0.0f32;
                    let mut sx = 0.0f32;
                    let mut sx2 = 0.0f32;
                    for i in 0..mp {
                        let q = t + 1 - mp + i;
                        let y = c[q] - o[q];
                        let x = i as f32;
                        sy = sy + y;
                        sxy = sxy + x * y;
                        sx = sx + x;
                        sx2 = sx2 + x * x;
                    }
                    let den = nn * sx2 - sx * sx;
                    if den.abs() >= 1.0e-12f32 {
                        held0 = (nn * sxy - sx * sy) / den;
                    } else {
                        held0 = 0.0f32;
                    }
                }
            }
            v0 = held0;
            v1 = sq;
        } else if formula >= 1295u32 && formula <= 1299u32 {
            // pivot levels family: every `period` bars (aligned to bar 0) the H/L/C (open for Woodie / Demark)
            // of the finished period give new levels; between completions the last levels are held.
            // The CPU `levels_grid` matrix (one price rung per row) is emitted as one column per rung.
            // scr[0..9] holds the rungs in grid order (lowest first). 1295 Pivot `[r1, s1, point, grid(7)]`,
            // 1296 Floor `[pivot, grid(7)]`, 1297 Camarilla `[pivot, grid(9)]`, 1298 Woodie `[pivot, grid(9)]`,
            // 1299 DeMark `[pivot, grid(3)]`.
            let mut pp = period as usize;
            if pp < 1usize {
                pp = 1usize;
            }
            if (t + 1) % pp == 0usize {
                let mut hh = h[t];
                let mut ll = l[t];
                for k2 in 0..pp {
                    hh = hh.max(h[t - k2]);
                    ll = ll.min(l[t - k2]);
                }
                let cl = c[t];
                let op = o[t + 1 - pp];
                let rg = hh - ll;
                if formula == 1295u32 || formula == 1296u32 {
                    let pv = (hh + ll + cl) / 3.0f32;
                    scr[0] = ll - 2.0f32 * (hh - pv);
                    scr[1] = pv - rg;
                    scr[2] = 2.0f32 * pv - hh;
                    scr[3] = pv;
                    scr[4] = 2.0f32 * pv - ll;
                    scr[5] = pv + rg;
                    scr[6] = hh + 2.0f32 * (pv - ll);
                } else if formula == 1297u32 {
                    let mm = 1.1f32;
                    scr[0] = cl - rg * mm / 2.0f32;
                    scr[1] = cl - rg * mm / 4.0f32;
                    scr[2] = cl - rg * mm / 6.0f32;
                    scr[3] = cl - rg * mm / 12.0f32;
                    scr[4] = (hh + ll + cl) / 3.0f32;
                    scr[5] = cl + rg * mm / 12.0f32;
                    scr[6] = cl + rg * mm / 6.0f32;
                    scr[7] = cl + rg * mm / 4.0f32;
                    scr[8] = cl + rg * mm / 2.0f32;
                } else if formula == 1298u32 {
                    let pv = (hh + ll + 2.0f32 * op) / 4.0f32;
                    let r3 = hh + 2.0f32 * (pv - ll);
                    let s3 = ll - 2.0f32 * (hh - pv);
                    scr[0] = s3 - rg;
                    scr[1] = s3;
                    scr[2] = pv - rg;
                    scr[3] = 2.0f32 * pv - hh;
                    scr[4] = pv;
                    scr[5] = 2.0f32 * pv - ll;
                    scr[6] = pv + rg;
                    scr[7] = r3;
                    scr[8] = r3 + rg;
                } else {
                    let mut xv = hh + ll + 2.0f32 * cl;
                    if cl < op {
                        xv = hh + 2.0f32 * ll + cl;
                    } else if cl > op {
                        xv = 2.0f32 * hh + ll + cl;
                    }
                    scr[0] = xv / 2.0f32 - hh;
                    scr[1] = xv / 4.0f32;
                    scr[2] = xv / 2.0f32 - ll;
                }
            }
            if formula == 1295u32 {
                v0 = scr[4];
                v1 = scr[2];
                v2 = scr[3];
                v3 = scr[0];
                v4 = scr[1];
                v5 = scr[2];
                v6 = scr[3];
                v7 = scr[4];
                v8 = scr[5];
                v9 = scr[6];
            } else if formula == 1296u32 {
                v0 = scr[3];
                v1 = scr[0];
                v2 = scr[1];
                v3 = scr[2];
                v4 = scr[3];
                v5 = scr[4];
                v6 = scr[5];
                v7 = scr[6];
            } else if formula == 1299u32 {
                v0 = scr[1];
                v1 = scr[0];
                v2 = scr[1];
                v3 = scr[2];
            } else {
                v0 = scr[4];
                v1 = scr[0];
                v2 = scr[1];
                v3 = scr[2];
                v4 = scr[3];
                v5 = scr[4];
                v6 = scr[5];
                v7 = scr[6];
                v8 = scr[7];
                v9 = scr[8];
            }
        } else if formula == 1300u32 {
            // Alligator: SMA 13 / 8 / 5 of the median price in lanes o / h / l, shifted back by 8 / 5 / 3 bars
            // (the shifted value appears once the offset buffer is full, before that the live smoother value)
            v0 = o[t];
            if t + 1 >= 8usize {
                v0 = o[t - 7];
            }
            v1 = h[t];
            if t + 1 >= 5usize {
                v1 = h[t - 4];
            }
            v2 = l[t];
            if t + 1 >= 3usize {
                v2 = l[t - 2];
            }
        } else if formula == 1301u32 {
            // classic daily pivot channels with adaptive width: levels from the first bar, then
            // recomputed every `period` bars (default 1440); lanes o,h,l,c; outputs [upper, pivot, lower]
            let mut pp = period as usize;
            if pp < 1usize {
                pp = 1usize;
            }
            let mut recompute = t == 0usize;
            let mut hh = h[t];
            let mut ll = l[t];
            if (t + 1) % pp == 0usize {
                recompute = true;
                for k2 in 0..pp {
                    hh = hh.max(h[t - k2]);
                    ll = ll.min(l[t - k2]);
                }
            }
            if recompute {
                let cl = c[t];
                let pv = (hh + ll + cl) / 3.0f32;
                scr[0] = hh + 2.0f32 * (pv - ll);
                scr[1] = pv + (hh - ll);
                scr[2] = 2.0f32 * pv - ll;
                scr[3] = pv;
                scr[4] = 2.0f32 * pv - hh;
                scr[5] = pv - (hh - ll);
                scr[6] = ll - 2.0f32 * (hh - pv);
            }
            // volatility ring (last 100 ranges) -> width multiplier
            let mut vc = t + 1;
            if vc > 100usize {
                vc = 100usize;
            }
            let mut vs = 0.0f32;
            for k2 in 0..vc {
                vs = vs + (h[t - k2] - l[t - k2]);
            }
            let avgv = vs / (vc as f32);
            if avgv > 0.0f32 {
                held0 = ((h[t] - l[t]) / avgv).max(0.5f32).min(2.0f32);
            } else if t == 0usize {
                held0 = 1.0f32;
            }
            let px = c[t];
            let mut res = 0.0f32;
            let mut sup = 0.0f32;
            let mut has_r = 0u32;
            let mut has_s = 0u32;
            for k2 in 0..7usize {
                let lv = scr[k2];
                if lv > px {
                    if has_r == 0u32 || lv < res {
                        res = lv;
                        has_r = 1u32;
                    }
                }
                if lv < px {
                    if has_s == 0u32 || lv > sup {
                        sup = lv;
                        has_s = 1u32;
                    }
                }
            }
            let mut rng = scr[0] - scr[6];
            if rng < 0.0f32 {
                rng = 0.0f32;
            }
            if has_r == 0u32 {
                if rng > 0.0f32 {
                    res = px + rng * 0.5f32;
                } else {
                    res = px;
                }
            }
            if has_s == 0u32 {
                if rng > 0.0f32 {
                    sup = px - rng * 0.5f32;
                } else {
                    sup = px;
                }
            }
            if held0 != 1.0f32 {
                let ctr = (res + sup) / 2.0f32;
                let hw = (res - sup) / 2.0f32 * held0;
                res = ctr + hw;
                sup = ctr - hw;
            }
            v0 = res;
            v1 = scr[3];
            v2 = sup;
        } else if formula == 1302u32 {
            // Theil-Sen channels `[upper, middle, lower]`: window = period (5..=10000), k = a (<= 0 means 2).
            // Median slope by bisection over pairwise slopes, intercept / deviation medians by rank selection
            // (scr[0..w] intercepts, scr[w..2w] deviations); ring positions like the CPU buffer.
            let mut w = period as usize;
            if w < 5usize {
                w = 5usize;
            }
            if w > 10000usize {
                w = 10000usize;
            }
            let mut kk = a;
            if kk <= 0.0f32 {
                kk = 2.0f32;
            }
            if t + 1 >= w {
                let nsl = w * (w - 1) / 2;
                let mid_rank = nsl / 2;
                let mut lo = 3.4e38f32;
                let mut hi = 0.0f32;
                let mut first = 1u32;
                for i in 0..w {
                    let yi = ts_val(c, t, w, i);
                    for j in (i + 1)..w {
                        let sl = (ts_val(c, t, w, j) - yi) / ((j - i) as f32);
                        if first == 1u32 {
                            lo = sl;
                            hi = sl;
                            first = 0u32;
                        } else {
                            lo = lo.min(sl);
                            hi = hi.max(sl);
                        }
                    }
                }
                for _ in 0..40 {
                    let mid = 0.5f32 * (lo + hi);
                    if ts_cnt_le(c, t, w, mid) >= mid_rank + 1usize {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                // snap to the exact slope: largest slope that is <= hi
                let mut slope = lo;
                let mut got = 0u32;
                for i in 0..w {
                    let yi = ts_val(c, t, w, i);
                    for j in (i + 1)..w {
                        let sl = (ts_val(c, t, w, j) - yi) / ((j - i) as f32);
                        if sl <= hi {
                            if got == 0u32 || sl > slope {
                                slope = sl;
                                got = 1u32;
                            }
                        }
                    }
                }
                for i in 0..w {
                    scr[i] = ts_val(c, t, w, i) - slope * (i as f32);
                }
                let icpt = ts_kth(scr, 0usize, w, w / 2);
                for i in 0..w {
                    scr[w + i] = (ts_val(c, t, w, i) - (icpt + slope * (i as f32))).abs();
                }
                let mad = ts_kth(scr, w, w, w / 2);
                let mid = icpt + slope * ((w as f32) - 1.0f32);
                let sg = 1.4826f32 * mad;
                v0 = mid + kk * sg;
                v1 = mid;
                v2 = mid - kk * sg;
                held0 = v0;
                held1 = v1;
                held2 = v2;
            }
            v0 = held0;
            v1 = held1;
            v2 = held2;
        } else if formula >= 1303u32 && formula <= 1306u32 {
            // anchored VWAP family on the monthly anchor key in lane o (`GpuTimes::mkey`), one effective
            // anchor (the CPU params are ignored: every anchor is monthly). Cumulative sums reset when the key changes.
            // 1303 value, 1304 (close - vwap) / vwap, 1305 reversion z-score (window = max(period, 20)),
            // 1306 touch probability over max(period, 20) bars (threshold in `a`).
            if t == 0usize || o[t] != o[t - 1] {
                held0 = 0.0f32;
                held1 = 0.0f32;
            }
            let tp = (h[t] + l[t] + c[t]) / 3.0f32;
            let vv = v[t].max(0.0f32);
            held0 = held0 + tp * vv;
            held1 = held1 + vv;
            if held1 > 0.0f32 {
                vw = held0 / held1;
            }
            if formula == 1303u32 {
                v0 = vw;
            } else if formula == 1304u32 {
                if vw != 0.0f32 {
                    v0 = (c[t] - vw) / vw;
                }
            } else {
                let mut w = period as usize;
                if w < 20usize {
                    w = 20usize;
                }
                if formula == 1305u32 {
                    scr[t % w] = (c[t] - vw).abs();
                } else {
                    let mut tch = 0.0f32;
                    if (c[t] - vw).abs() / vw.max(1.0e-9f32) <= a.max(0.0f32) {
                        tch = 1.0f32;
                    }
                    scr[t % w] = tch;
                }
                if t + 1 >= w {
                    let mut mean = 0.0f32;
                    for k2 in 0..w {
                        mean = mean + scr[k2];
                    }
                    mean = mean / (w as f32);
                    if formula == 1306u32 {
                        held2 = mean;
                    } else {
                        let mut var = 0.0f32;
                        for k2 in 0..w {
                            let dd = scr[k2] - mean;
                            var = var + dd * dd;
                        }
                        let sd = (var / (w as f32)).sqrt().max(1.0e-9f32);
                        held2 = 0.0f32 - (scr[t % w] - mean) / sd;
                    }
                }
                v0 = held2;
            }
        } else if formula == 1307u32 {
            // Fisher transform `[fisher, trigger]`: period = window, p2 = smoothing period; held0 = smoothed
            // normalised price, held1 = fisher, held2 = ema seeded flag
            let pr = period as usize;
            if pr >= 1usize && t + 1 >= pr {
                let mut hh = h[t];
                let mut ll = l[t];
                for k2 in 0..pr {
                    hh = hh.max(h[t - k2]);
                    ll = ll.min(l[t - k2]);
                }
                let mut nrm = 0.0f32;
                if (hh - ll).abs() >= 1.0e-12f32 {
                    nrm = 2.0f32 * ((c[t] - ll) / (hh - ll)) - 1.0f32;
                }
                let al = 2.0f32 / ((p2 as f32) + 1.0f32);
                if held2 < 0.5f32 {
                    held0 = nrm;
                    held2 = 1.0f32;
                } else {
                    held0 = al * nrm + (1.0f32 - al) * held0;
                }
                let cl = held0.max(0.0f32 - 0.999f32).min(0.999f32);
                v1 = held1;
                held1 = 0.5f32 * ((1.0f32 + cl) / (1.0f32 - cl)).ln();
                f0 = v1;
            }
            v0 = held1;
            v1 = f0;
        } else if formula == 1308u32 {
            // relative trend position `[sma_rel, avwap_rel]`: smoother of close in lane v, cumulative VWAP inline
            let tp = (h[t] + l[t] + c[t]) / 3.0f32;
            let vv = v[t].max(0.0f32);
            held0 = held0 + tp * vv;
            held1 = held1 + vv;
            if held1 > 0.0f32 {
                vw = held0 / held1;
            }
            let sm = o[t];
            if sm != 0.0f32 {
                v0 = (c[t] - sm) / sm.abs().max(1.0e-9f32);
            }
            if vw != 0.0f32 {
                v1 = (c[t] - vw) / vw.abs().max(1.0e-9f32);
            }
        } else if formula == 1309u32 {
            // sweep reversion: lookback = period, ATR (lane v) period = p2, quartile = a, weight k = b,
            // flag = confirm next bar. held0 = pending signal, held1 = its reference close.
            let lb = period as usize;
            let mut sig = 0.0f32;
            if lb >= 1usize && t + 1 >= lb && t + 1 >= (p2 as usize) {
                let mut sw_top = false;
                let mut sw_bot = false;
                if t >= lb {
                    let mut ph = h[t - 1];
                    let mut pl = l[t - 1];
                    for k2 in 0..lb {
                        ph = ph.max(h[t - 1 - k2]);
                        pl = pl.min(l[t - 1 - k2]);
                    }
                    sw_top = h[t] > ph;
                    sw_bot = l[t] < pl;
                }
                let rg = (h[t] - l[t]).max(1.0e-12f32);
                let pos = (c[t] - l[t]) / rg;
                let mut raw = 0.0f32;
                if sw_top && pos <= a {
                    raw = 1.0f32;
                } else if sw_bot && pos >= 1.0f32 - a {
                    raw = 0.0f32 - 1.0f32;
                }
                if flag == 1u32 {
                    if held0 != 0.0f32 {
                        let mut ok = false;
                        if held0 > 0.0f32 {
                            ok = c[t] < held1;
                        } else {
                            ok = c[t] > held1;
                        }
                        if ok {
                            sig = held0;
                        }
                        held0 = 0.0f32;
                    } else if raw != 0.0f32 {
                        held0 = raw;
                        held1 = c[t];
                    }
                } else {
                    sig = raw;
                }
                let at = v[t].abs().max(1.0e-12f32);
                let dv = (c[t] - 0.5f32 * (h[t] + l[t])).abs();
                let wgt = (dv / (b.max(1.0e-12f32) * at)).min(1.0f32);
                v0 = sig * wgt;
            }
        } else if formula == 1310u32 {
            // Bai-Perron CUSUM: the CUSUM break detector restarted every max(period, 50) bars.
            // threshold = a, kappa = b. held0 = pos, held1 = neg, held2 = value, cnt_a = bars since restart.
            let mut sw = period as usize;
            if sw < 50usize {
                sw = 50usize;
            }
            if cnt_a > 0usize {
                let r = (c[t] / c[t - 1]).ln();
                held0 = (b * (held0 + r)).max(0.0f32);
                held1 = (b * (held1 - r)).max(0.0f32);
                let hp = (held0 - a).max(0.0f32);
                let hn = (held1 - a).max(0.0f32);
                held2 = hp.max(hn);
                if hp > 0.0f32 {
                    held0 = 0.0f32;
                }
                if hn > 0.0f32 {
                    held1 = 0.0f32;
                }
            }
            v0 = held2;
            cnt_a = cnt_a + 1usize;
            if cnt_a >= sw {
                held0 = 0.0f32;
                held1 = 0.0f32;
                held2 = 0.0f32;
                cnt_a = 0usize;
            }
        } else if formula == 1311u32 || formula == 1312u32 {
            // variance-ratio aggregate: mean of the three VR series in lanes o / h / l;
            // 1312 additionally z-scores that mean over max(ext[4], 20) bars (0 until the window is full)
            let mean = (o[t] + h[t] + l[t]) / 3.0f32;
            if formula == 1311u32 {
                v0 = mean;
            } else {
                let mut w = p4 as usize;
                if w < 20usize {
                    w = 20usize;
                }
                scr[t % w] = mean;
                if t + 1 >= w {
                    let mut sm = 0.0f32;
                    for k2 in 0..w {
                        sm = sm + scr[k2];
                    }
                    let mu = sm / (w as f32);
                    let mut ss = 0.0f32;
                    for k2 in 0..w {
                        let dd = scr[k2] - mu;
                        ss = ss + dd * dd;
                    }
                    let sd = (ss / (w as f32)).sqrt().max(1.0e-9f32);
                    v0 = (mean - mu) / sd;
                }
            }
        } else if formula == 1313u32 {
            // FVG hit rate over max(period, 20) triplets (hit = bull or bear fair value gap), 0 until full
            let mut w = period as usize;
            if w < 20usize {
                w = 20usize;
            }
            if t >= w + 1usize {
                let mut s = 0.0f32;
                for k2 in 0..w {
                    let q = t - k2;
                    let bull = l[q - 1] > h[q - 2] && l[q - 1] > h[q];
                    let bear = h[q - 1] < l[q - 2] && h[q - 1] < l[q];
                    if bull || bear {
                        s = s + 1.0f32;
                    }
                }
                held0 = s / (w as f32);
            }
            v0 = held0;
        } else if formula == 1314u32 {
            // FVG intensity: EMA (alpha in `a`, clamped 0..1) of the gap size at every triplet
            if t >= 2usize {
                let al = a.max(0.0f32).min(1.0f32);
                let bull = l[t - 1] > h[t - 2] && l[t - 1] > h[t];
                let bear = h[t - 1] < l[t - 2] && h[t - 1] < l[t];
                let mut gap = 0.0f32;
                if bull {
                    gap = (l[t - 1] - h[t - 2]).max(l[t - 1] - h[t]).max(0.0f32);
                } else if bear {
                    gap = (l[t - 2] - h[t - 1]).min(l[t] - h[t - 1]).abs();
                }
                held0 = al * gap + (1.0f32 - al) * held0;
            }
            v0 = held0;
        } else if formula == 1315u32 {
            // FVG reversion probability: pending gaps (upper, lower, remaining bars) live in scr[3 * i];
            // held0 = total gaps, held1 = gaps revisited by the next close, cnt_a = pending count
            if t >= 3usize {
                let mut hz = period as usize;
                if hz < 1usize {
                    hz = 1usize;
                }
                if hz > 50usize {
                    hz = 50usize;
                }
                let b0 = t - 3;
                let b1 = t - 2;
                let b2 = t - 1;
                let bull = l[b1] > h[b0] && l[b1] > h[b2];
                let bear = h[b1] < l[b0] && h[b1] < l[b2];
                if bull {
                    scr[3 * cnt_a] = l[b1];
                    scr[3 * cnt_a + 1] = h[b0].max(h[b2]);
                    scr[3 * cnt_a + 2] = hz as f32;
                    cnt_a = cnt_a + 1usize;
                    held0 = held0 + 1.0f32;
                } else if bear {
                    scr[3 * cnt_a] = l[b0].min(h[b2]);
                    scr[3 * cnt_a + 1] = h[b1];
                    scr[3 * cnt_a + 2] = hz as f32;
                    cnt_a = cnt_a + 1usize;
                    held0 = held0 + 1.0f32;
                }
                let nc = c[t];
                let mut dst = 0usize;
                for i in 0..cnt_a {
                    let up = scr[3 * i];
                    let lo = scr[3 * i + 1];
                    let rem = scr[3 * i + 2];
                    if nc <= up && nc >= lo {
                        held1 = held1 + 1.0f32;
                    } else if rem > 1.0f32 {
                        scr[3 * dst] = up;
                        scr[3 * dst + 1] = lo;
                        scr[3 * dst + 2] = rem - 1.0f32;
                        dst = dst + 1usize;
                    }
                }
                cnt_a = dst;
                if held0 > 0.0f32 {
                    held2 = held1 / held0;
                }
            }
            v0 = held2;
        } else if formula == 1316u32 {
            // statistical wick detector `[upper_spike, lower_spike]`: percentile rank (>= 0.95) of the current
            // wick fractions within the last max(period, 1) bars
            let mut w = period as usize;
            if w < 1usize {
                w = 1usize;
            }
            let mut len = t + 1;
            if len > w {
                len = w;
            }
            let rg = (h[t] - l[t]).abs().max(1.0e-12f32);
            let up = (h[t] - o[t].max(c[t])).max(0.0f32) / rg;
            let lo = (o[t].min(c[t]) - l[t]).max(0.0f32) / rg;
            let mut cu = 0.0f32;
            let mut cl = 0.0f32;
            for k2 in 0..len {
                let q = t - k2;
                let rq = (h[q] - l[q]).abs().max(1.0e-12f32);
                let uq = (h[q] - o[q].max(c[q])).max(0.0f32) / rq;
                let lq = (o[q].min(c[q]) - l[q]).max(0.0f32) / rq;
                if uq <= up {
                    cu = cu + 1.0f32;
                }
                if lq <= lo {
                    cl = cl + 1.0f32;
                }
            }
            if cu / (len as f32) >= 0.95f32 {
                v0 = 1.0f32;
            }
            if cl / (len as f32) >= 0.95f32 {
                v1 = 1.0f32;
            }
        } else if formula == 1317u32 {
            // swing stop `[long_stop, short_stop]`: lookback = period, min swing size = a, offset = b,
            // flag = percentage offset. held0 = last swing high (0 = none), held1 = last swing low (f0 = seen),
            // held2 = long stop, f1 = short stop
            let lb = period as usize;
            if lb >= 1usize && t >= 2usize * lb {
                let ci = t - lb;
                let ch = h[ci];
                let mut is_h = true;
                for q in (t - 2usize * lb)..ci {
                    if h[q] >= ch {
                        is_h = false;
                    }
                }
                for q in (ci + 1)..(t + 1) {
                    if h[q] >= ch {
                        is_h = false;
                    }
                }
                if is_h && a > 0.0f32 && held0 > 0.0f32 {
                    if (ch - held0).abs() < a {
                        is_h = false;
                    }
                }
                if is_h {
                    held0 = ch;
                }
                let cw = l[ci];
                let mut is_l = true;
                for q in (t - 2usize * lb)..ci {
                    if l[q] <= cw {
                        is_l = false;
                    }
                }
                for q in (ci + 1)..(t + 1) {
                    if l[q] <= cw {
                        is_l = false;
                    }
                }
                if is_l && a > 0.0f32 && f0 > 0.5f32 {
                    if (held1 - cw).abs() < a {
                        is_l = false;
                    }
                }
                if is_l {
                    held1 = cw;
                    f0 = 1.0f32;
                }
            }
            if f0 > 0.5f32 {
                if flag == 1u32 {
                    held2 = held1 * (1.0f32 - b / 100.0f32);
                } else {
                    held2 = held1 - b;
                }
            }
            if held0 > 0.0f32 {
                if flag == 1u32 {
                    f1 = held0 * (1.0f32 + b / 100.0f32);
                } else {
                    f1 = held0 + b;
                }
            }
            v0 = held2;
            v1 = f1;
        } else if formula == 1318u32 {
            // Connors RSI: (RSI + streak RSI + ROC percentile rank) / 3. Lane o = RSI fed from bar 1 (host
            // pass), p2 = streak RSI period, p3 = ROC window. scr ring (p2 + 1) of streak lengths;
            // held0 = streak length, held1 = last direction, held2 = avg gain, f0 = avg loss, f1 = streak RSI
            v0 = 50.0f32;
            if t >= 1usize {
                let up = p2 as usize;
                let rp = p3 as usize;
                let wr = up + 1usize;
                let dir = c[t] - c[t - 1];
                let mut dv = 0.0f32;
                if dir > 0.0f32 {
                    dv = 1.0f32;
                } else if dir < 0.0f32 {
                    dv = 0.0f32 - 1.0f32;
                }
                if dv != 0.0f32 {
                    if dv == held1 {
                        if held0 > 0.0f32 && dv > 0.0f32 {
                            held0 = held0 + 1.0f32;
                        } else if held0 < 0.0f32 && dv < 0.0f32 {
                            held0 = held0 - 1.0f32;
                        } else {
                            held0 = dv;
                        }
                    } else {
                        held0 = dv;
                    }
                    held1 = dv;
                }
                scr[t % wr] = held0;
                let mut udv = 50.0f32;
                if t >= 2usize {
                    let chg = scr[t % wr] - scr[(t - 1) % wr];
                    if t >= up + 1usize {
                        if held2 == 0.0f32 && f0 == 0.0f32 {
                            let mut sg = 0.0f32;
                            let mut sl = 0.0f32;
                            for k2 in 0..up {
                                let q = t - k2;
                                let cq = scr[q % wr] - scr[(q - 1) % wr];
                                if cq > 0.0f32 {
                                    sg = sg + cq;
                                }
                                if cq < 0.0f32 {
                                    sl = sl - cq;
                                }
                            }
                            held2 = sg / (up as f32);
                            f0 = sl / (up as f32);
                        } else {
                            let al = 1.0f32 / (up as f32);
                            let mut g = 0.0f32;
                            let mut ls = 0.0f32;
                            if chg > 0.0f32 {
                                g = chg;
                            }
                            if chg < 0.0f32 {
                                ls = 0.0f32 - chg;
                            }
                            held2 = al * g + (1.0f32 - al) * held2;
                            f0 = al * ls + (1.0f32 - al) * f0;
                        }
                        if f0 == 0.0f32 {
                            udv = 100.0f32;
                        } else {
                            udv = 100.0f32 - 100.0f32 / (1.0f32 + held2 / f0);
                        }
                    }
                }
                let mut rocp = 50.0f32;
                if rp >= 1usize && t >= rp {
                    let mut pr = 0.0f32;
                    if c[t - 1] != 0.0f32 {
                        pr = (c[t] - c[t - 1]) / c[t - 1] * 100.0f32;
                    }
                    let mut below = 0.0f32;
                    let mut eq = 0.0f32;
                    for k2 in 0..rp {
                        let q = t - k2;
                        let mut rq = 0.0f32;
                        if c[q - 1] != 0.0f32 {
                            rq = (c[q] - c[q - 1]) / c[q - 1] * 100.0f32;
                        }
                        if rq < pr {
                            below = below + 1.0f32;
                        } else if (rq - pr).abs() < 1.0e-9f32 {
                            eq = eq + 1.0f32;
                        }
                    }
                    rocp = (below + 0.5f32 * eq) / (rp as f32) * 100.0f32;
                }
                v0 = (o[t] + udv + rocp) / 3.0f32;
            }
        } else if formula == 1319u32 {
            // adaptive stochastic %K (the D smoother runs afterwards on the host path): lane o = Wilder ATR,
            // period = base / min period, p2 = max period, a = volatility sensitivity.
            // held0 = current period, held1 = volatility EMA(10) (live from the 16th bar)
            if t == 0usize {
                held0 = period as f32;
            }
            if t >= 15usize {
                if t == 15usize {
                    held1 = o[t];
                } else {
                    held1 = (2.0f32 / 11.0f32) * o[t] + (1.0f32 - 2.0f32 / 11.0f32) * held1;
                }
                let mut nrm = 0.01f32;
                if c[t] > 0.0f32 {
                    nrm = held1 / c[t];
                }
                let vf = (nrm * 100.0f32 * a).max(0.1f32);
                let adj = 1.0f32 / (1.0f32 + vf);
                held0 = ((period as f32) * adj).max(period as f32).min(p2 as f32);
            }
            let mut pu = held0 as usize;
            if pu < 2usize {
                pu = 2usize;
            }
            let mut avail = t + 1;
            if avail > 64usize {
                avail = 64usize;
            }
            if avail > pu {
                avail = pu;
            }
            v0 = 50.0f32;
            if avail >= 2usize {
                let mut hh = h[t];
                let mut ll = l[t];
                for k2 in 0..avail {
                    hh = hh.max(h[t - k2]);
                    ll = ll.min(l[t - k2]);
                }
                if (hh - ll).abs() > 1.0e-12f32 {
                    v0 = ((c[t] - ll) / (hh - ll)) * 100.0f32;
                }
            }
        } else if formula == 1320u32 {
            // pivot anchored VWAP: cumulative close * volume restarts whenever the high is at least the previous
            // lookback - 1 highs (or the low at most their lows); lookback = max(period, 3), only once the ring is full
            let mut lb = period as usize;
            if lb < 3usize {
                lb = 3usize;
            }
            if t >= lb {
                let mut pmax = h[t - 1];
                let mut pmin = l[t - 1];
                for k2 in 1..lb {
                    pmax = pmax.max(h[t - k2]);
                    pmin = pmin.min(l[t - k2]);
                }
                if h[t] >= pmax || l[t] <= pmin {
                    held0 = 0.0f32;
                    held1 = 0.0f32;
                }
            }
            held0 = held0 + c[t] * v[t];
            held1 = held1 + v[t].max(1.0e-12f32);
            v0 = held0 / held1;
        } else if formula == 1321u32 {
            // swing strength score: left = period (1..=10), right = p2 (1..=10); once the 16+ slot ring is full
            // the pivot is the bar `right` back; tanh of (up strength - down strength) / mean range
            let mut lf = period as usize;
            let mut rt = p2 as usize;
            if lf < 1usize {
                lf = 1usize;
            }
            if lf > 10usize {
                lf = 10usize;
            }
            if rt < 1usize {
                rt = 1usize;
            }
            if rt > 10usize {
                rt = 10usize;
            }
            let mut cap = lf + rt + 2usize;
            if cap < 16usize {
                cap = 16usize;
            }
            if t + 1 >= cap {
                let pv = t - rt;
                let ph = h[pv];
                let pl = l[pv];
                let mut lmax = h[pv - 1];
                let mut lmin = l[pv - 1];
                for i in 1..(lf + 1) {
                    lmax = lmax.max(h[pv - i]);
                    lmin = lmin.min(l[pv - i]);
                }
                let mut rmax = h[pv + 1];
                let mut rmin = l[pv + 1];
                for i in 1..(rt + 1) {
                    rmax = rmax.max(h[pv + i]);
                    rmin = rmin.min(l[pv + i]);
                }
                let upk = (ph - lmax).max(0.0f32) + (ph - rmax).max(0.0f32);
                let dnk = (lmin - pl).max(0.0f32) + (rmin - pl).max(0.0f32);
                let raw = upk - dnk;
                let mut rs = 0.0f32;
                for i in 0..(lf + rt + 1) {
                    let q = pv - lf + i;
                    rs = rs + (h[q] - l[q]);
                }
                let den = (rs / ((lf + rt + 1) as f32)).max(1.0e-6f32);
                let z = raw / den;
                let e2 = (2.0f32 * z).exp();
                held0 = (e2 - 1.0f32) / (e2 + 1.0f32);
            }
            v0 = held0;
        } else if formula == 1322u32 {
            // liquidity gap density: mean over the last clamp(period, 20, 512) bars of the capped gap score
            // (gap / threshold in `a`, capped at 5, 0 when the gap does not exceed the threshold)
            let mut w = period as usize;
            if w < 20usize {
                w = 20usize;
            }
            if w > 512usize {
                w = 512usize;
            }
            let mut len = t + 1;
            if len > w {
                len = w;
            }
            let mut s = 0.0f32;
            for k2 in 0..len {
                let q = t - k2;
                if q >= 1usize {
                    let ug = (l[q] - h[q - 1]).max(0.0f32);
                    let dg = (l[q - 1] - h[q]).max(0.0f32);
                    let amp = ug.max(dg);
                    if amp > a {
                        s = s + (amp / a).min(5.0f32);
                    }
                }
            }
            v0 = s / (len as f32);
        } else if formula == 1323u32 {
            // FFT dominant period: window N = next power of two of `period` (max 256), Hamming window over the
            // last N closes, DFT of the first N / 2 bins; dominant bin = first strict maximum of the magnitude
            // over bins 1..N/2 (the CPU spectral smoothing never fires: it indexes the vector being built);
            // period = N / (bin * sampling_rate in `a`), 0 when no bin. Computed from bar 2N - 1 on, 0 before.
            let mut nn = 1usize;
            while nn < (period as usize) {
                nn = nn * 2usize;
            }
            if nn > 256usize {
                nn = 256usize;
            }
            if nn >= 2usize && t + 1 >= 2usize * nn {
                let half = nn / 2usize;
                let mut bestm = 0.0f32;
                let mut besti = 0usize;
                let tp = 6.2831855f32;
                for kb in 1..half {
                    let mut re = 0.0f32;
                    let mut im = 0.0f32;
                    for j in 0..nn {
                        let wj = 0.54f32 - 0.46f32 * (tp * (j as f32) / ((nn - 1usize) as f32)).cos();
                        let xv = c[t + 1usize - nn + j] * wj;
                        let ang = tp * (((kb * j) % nn) as f32) / (nn as f32);
                        re = re + xv * ang.cos();
                        im = im - xv * ang.sin();
                    }
                    let mg = re * re + im * im;
                    if mg > bestm {
                        bestm = mg;
                        besti = kb;
                    }
                }
                if besti > 0usize {
                    held0 = (nn as f32) / ((besti as f32) * a);
                } else {
                    held0 = 0.0f32;
                }
            }
            v0 = held0;
        } else if formula == 1324u32 {
            // wavelet entropy over the last min(t + 1, 512) closes (0 until 32 samples). `period` = max scales
            // (<= 32, >= 2), `flag` = wavelet (0 Haar, 1 Db4, 2 Db6, 3 Morlet, 4 Mexican, 5 Biorthogonal).
            // Morlet / Mexican: CWT energies over scales 64^(i / (ms - 1)); others: DWT detail energies for
            // min(ms, 8) levels with circular convolution. Entropy = -sum p ln p of the energies.
            // scratch: [0, 512) signal A, [512, 1024) signal B, [1024, 1030) low-pass, [1030, 1036) high-pass,
            // [1040, 1072) energies
            let mut nt = t + 1usize;
            if nt > 512usize {
                nt = 512usize;
            }
            let mut msc = period as usize;
            if msc > 32usize {
                msc = 32usize;
            }
            if nt >= 32usize && msc >= 2usize {
                let mut tot = 0.0f32;
                let mut nen = 0usize;
                if flag == 3u32 || flag == 4u32 {
                    for i in 0..msc {
                        let frac = (i as f32) / ((msc - 1usize) as f32);
                        let sc = (frac * 4.158883f32).exp();
                        let mut en = 0.0f32;
                        for pos in 0..nt {
                            let mut cf = 0.0f32;
                            for k in 0..nt {
                                let tt = ((k as f32) - (pos as f32)) / sc;
                                let g = (0.0f32 - tt * tt / 2.0f32).exp();
                                let mut psi = (1.0f32 - tt * tt) * g;
                                if flag == 3u32 {
                                    psi = (6.0f32 * tt).cos() * g;
                                }
                                cf = cf + c[t + 1usize - nt + k] * psi;
                            }
                            cf = cf / sc.sqrt();
                            en = en + cf * cf;
                        }
                        scr[1040 + i] = en;
                        tot = tot + en;
                    }
                    nen = msc;
                } else {
                    let mut nl = 2usize;
                    let mut ng = 2usize;
                    scr[1024] = 0.70710677f32;
                    scr[1025] = 0.70710677f32;
                    scr[1030] = 0.0f32 - 0.70710677f32;
                    scr[1031] = 0.70710677f32;
                    if flag == 1u32 {
                        nl = 4usize;
                        ng = 4usize;
                        scr[1024] = 0.6830127f32;
                        scr[1025] = 1.1830127f32;
                        scr[1026] = 0.3169873f32;
                        scr[1027] = 0.0f32 - 0.18301270f32;
                        // high-pass = alternating signs (even index -1) of h, reversed
                        scr[1030] = 0.0f32 - 0.18301270f32;
                        scr[1031] = 0.0f32 - 0.3169873f32;
                        scr[1032] = 1.1830127f32;
                        scr[1033] = 0.0f32 - 0.6830127f32;
                    } else if flag == 2u32 {
                        nl = 6usize;
                        ng = 6usize;
                        scr[1024] = 0.47046721f32;
                        scr[1025] = 1.14111692f32;
                        scr[1026] = 0.650365f32;
                        scr[1027] = 0.0f32 - 0.19093442f32;
                        scr[1028] = 0.0f32 - 0.12083221f32;
                        scr[1029] = 0.0498175f32;
                        scr[1030] = 0.0498175f32;
                        scr[1031] = 0.12083221f32;
                        scr[1032] = 0.0f32 - 0.19093442f32;
                        scr[1033] = 0.0f32 - 0.650365f32;
                        scr[1034] = 1.14111692f32;
                        scr[1035] = 0.0f32 - 0.47046721f32;
                    } else if flag == 5u32 {
                        nl = 5usize;
                        ng = 3usize;
                        scr[1024] = 0.0f32 - 0.125f32;
                        scr[1025] = 0.25f32;
                        scr[1026] = 0.75f32;
                        scr[1027] = 0.25f32;
                        scr[1028] = 0.0f32 - 0.125f32;
                        scr[1030] = 0.5f32;
                        scr[1031] = 1.0f32;
                        scr[1032] = 0.5f32;
                    }
                    for k in 0..nt {
                        scr[k] = c[t + 1usize - nt + k];
                    }
                    let mut m = nt;
                    let mut src_off = 0usize;
                    let mut lv = msc;
                    if lv > 8usize {
                        lv = 8usize;
                    }
                    let mut go = true;
                    for _l in 0..lv {
                        if go && m >= 4usize {
                            let dst_off = 512usize - src_off;
                            let mut en = 0.0f32;
                            let mut oi = 0usize;
                            let mut i = 0usize;
                            while i < m {
                                let mut asum = 0.0f32;
                                let mut dsum = 0.0f32;
                                for j in 0..nl {
                                    asum = asum + scr[1024 + j] * scr[src_off + (i + j) % m];
                                }
                                for j in 0..ng {
                                    dsum = dsum + scr[1030 + j] * scr[src_off + (i + j) % m];
                                }
                                scr[dst_off + oi] = asum;
                                en = en + dsum * dsum;
                                oi = oi + 1usize;
                                i = i + 2usize;
                            }
                            scr[1040 + nen] = en;
                            tot = tot + en;
                            nen = nen + 1usize;
                            m = oi;
                            src_off = dst_off;
                        } else {
                            go = false;
                        }
                    }
                }
                let mut ent = 0.0f32;
                if tot > 0.0f32 {
                    for i in 0..nen {
                        let p = scr[1040 + i] / tot;
                        if p > 0.0f32 {
                            ent = ent - p * p.ln();
                        }
                    }
                }
                held0 = ent;
            }
            v0 = held0;
        } else if formula == 1325u32 {
            // mutual information of the close return and its lag-return over the last (window - lag) pairs.
            // period = window (>= 2), p2 = lag (1..window-1), p3 = bins (2..32), a = clip. Equivalent to the CPU
            // incremental joint table: pairs of bars max(first full ring bar, t - (window - lag) + 1) ..= t.
            // scratch: joint [0, 1024), px [1024, 1056), py [1056, 1088)
            let mut w = period as usize;
            if w < 2usize {
                w = 2usize;
            }
            let mut lg = p2 as usize;
            if lg < 1usize {
                lg = 1usize;
            }
            if lg > w - 1usize {
                lg = w - 1usize;
            }
            let mut bn = p3 as usize;
            if bn < 2usize {
                bn = 2usize;
            }
            if bn > 32usize {
                bn = 32usize;
            }
            let clip = a.max(1.0e-6f32);
            let ring = w + 1usize + lg;
            if t + 1usize >= ring {
                let np = w - lg;
                let mut st = 0usize;
                if t + 1usize > np {
                    st = t + 1usize - np;
                }
                if st < ring - 1usize {
                    st = ring - 1usize;
                }
                for i in 0..(bn * bn) {
                    scr[i] = 0.0f32;
                }
                let mut cnt = 0.0f32;
                for s in st..(t + 1usize) {
                    let mut r = 0.0f32;
                    if c[s - 1usize] > 0.0f32 && c[s] > 0.0f32 {
                        r = (c[s] / c[s - 1usize]).ln();
                    }
                    let mut rl = 0.0f32;
                    if c[s - lg - 1usize] > 0.0f32 && c[s - lg] > 0.0f32 {
                        rl = (c[s - lg] / c[s - lg - 1usize]).ln();
                    }
                    let rr = r.max(0.0f32 - clip).min(clip);
                    let xr = (rr + clip) / (2.0f32 * clip) * (bn as f32);
                    let bx = (xr.floor().max(0.0f32).min((bn - 1usize) as f32)) as usize;
                    let rr2 = rl.max(0.0f32 - clip).min(clip);
                    let yr = (rr2 + clip) / (2.0f32 * clip) * (bn as f32);
                    let by = (yr.floor().max(0.0f32).min((bn - 1usize) as f32)) as usize;
                    scr[bx * bn + by] = scr[bx * bn + by] + 1.0f32;
                    cnt = cnt + 1.0f32;
                }
                let total = cnt.max(1.0f32);
                for i in 0..bn {
                    scr[1024 + i] = 0.0f32;
                    scr[1056 + i] = 0.0f32;
                }
                for bx in 0..bn {
                    for by in 0..bn {
                        scr[1024 + bx] = scr[1024 + bx] + scr[bx * bn + by];
                        scr[1056 + by] = scr[1056 + by] + scr[bx * bn + by];
                    }
                }
                let mut mi = 0.0f32;
                for bx in 0..bn {
                    for by in 0..bn {
                        let pxy = scr[bx * bn + by] / total;
                        let pxv = scr[1024 + bx] / total;
                        let pyv = scr[1056 + by] / total;
                        if pxy > 0.0f32 && pxv > 0.0f32 && pyv > 0.0f32 {
                            mi = mi + pxy * (pxy / (pxv * pyv)).ln();
                        }
                    }
                }
                held0 = mi.max(0.0f32);
            }
            v0 = held0;
        } else if formula == 1326u32 {
            // transfer entropy (volume return -> price return), lanes close / volume. period = window (>= 3),
            // p2 = lag (1..window-2), p3 = bins (2..16), a = clip. Joint triples (y, y1, x1) of bars
            // max(first full ring bar, t - (window - lag - 1) + 1) ..= t. scratch: joint [0, 4096),
            // p(y|y1,x1) marginal [4096, 4352), p(y,y1) marginal [4352, 4608)
            let mut w = period as usize;
            if w < 3usize {
                w = 3usize;
            }
            let mut lg = p2 as usize;
            if lg < 1usize {
                lg = 1usize;
            }
            if lg > w - 2usize {
                lg = w - 2usize;
            }
            let mut bn = p3 as usize;
            if bn < 2usize {
                bn = 2usize;
            }
            if bn > 16usize {
                bn = 16usize;
            }
            let clip = a.max(1.0e-6f32);
            let ring = w + 2usize;
            if t >= ring {
                let np = w - lg - 1usize;
                let mut st = 0usize;
                if t + 1usize > np {
                    st = t + 1usize - np;
                }
                if st < ring {
                    st = ring;
                }
                let b3 = bn * bn * bn;
                let b2 = bn * bn;
                for i in 0..b3 {
                    scr[i] = 0.0f32;
                }
                let mut cnt = 0.0f32;
                for s in st..(t + 1usize) {
                    let mut yv = 0.0f32;
                    let mut yv1 = 0.0f32;
                    let mut xv1 = 0.0f32;
                    if c[s - 1usize] > 0.0f32 {
                        yv = (c[s] / c[s - 1usize]).ln();
                    }
                    if c[s - 2usize] > 0.0f32 {
                        yv1 = (c[s - 1usize] / c[s - 2usize]).ln();
                    }
                    let xq = s - 1usize - lg;
                    let pvq = v[xq - 1usize].max(1.0f32);
                    if v[xq] > 0.0f32 {
                        xv1 = (v[xq] / pvq).ln();
                    }
                    let r0 = yv.max(0.0f32 - clip).min(clip);
                    let by = (((r0 + clip) / (2.0f32 * clip) * (bn as f32)).floor().max(0.0f32).min((bn - 1usize) as f32)) as usize;
                    let r1 = yv1.max(0.0f32 - clip).min(clip);
                    let by1 = (((r1 + clip) / (2.0f32 * clip) * (bn as f32)).floor().max(0.0f32).min((bn - 1usize) as f32)) as usize;
                    let r2 = xv1.max(0.0f32 - clip).min(clip);
                    let bx1 = (((r2 + clip) / (2.0f32 * clip) * (bn as f32)).floor().max(0.0f32).min((bn - 1usize) as f32)) as usize;
                    let ix = (by * bn + by1) * bn + bx1;
                    scr[ix] = scr[ix] + 1.0f32;
                    cnt = cnt + 1.0f32;
                }
                let total = cnt.max(1.0f32);
                for i in 0..b2 {
                    scr[4096 + i] = 0.0f32;
                    scr[4352 + i] = 0.0f32;
                }
                for y in 0..bn {
                    for y1 in 0..bn {
                        for x1 in 0..bn {
                            let jv = scr[(y * bn + y1) * bn + x1];
                            scr[4096 + y1 * bn + x1] = scr[4096 + y1 * bn + x1] + jv;
                            scr[4352 + y * bn + y1] = scr[4352 + y * bn + y1] + jv;
                        }
                    }
                }
                let mut te = 0.0f32;
                for y in 0..bn {
                    for y1 in 0..bn {
                        for x1 in 0..bn {
                            let pyyx = scr[(y * bn + y1) * bn + x1] / total;
                            if pyyx > 0.0f32 {
                                let a1 = pyyx / (scr[4096 + y1 * bn + x1] / total).max(1.0e-12f32);
                                let a2 = pyyx / (scr[4352 + y * bn + y1] / total).max(1.0e-12f32);
                                if a1 > 0.0f32 && a2 > 0.0f32 {
                                    te = te + pyyx * (a1 / a2).ln();
                                }
                            }
                        }
                    }
                }
                held0 = te.max(0.0f32);
            }
            v0 = held0;
        } else if formula == 1327u32 {
            // fuzzy candlesticks `[direction, size, body_size, upper_wick, lower_wick]` as the CPU i8 codes.
            // period = window, thresholds t1 = a, t2 = b, t3 = c, t4 = p4 / 1000. Population std over the last
            // `period` values of range, (open - low) / range (the CPU "body percent"), upper and lower wick shares;
            // all zero until the window is full. The Medium size test is `sd * t2` without the mean, as on the CPU.
            let pw = period as usize;
            if pw >= 1usize && t + 1usize >= pw {
                let t4 = (p4 as f32) / 1000.0f32;
                let mut m0 = 0.0f32;
                let mut m1 = 0.0f32;
                let mut m2 = 0.0f32;
                let mut m3 = 0.0f32;
                for k2 in 0..pw {
                    let q = t - k2;
                    let ln = (h[q] - l[q]).abs();
                    let mut bp = 0.0f32;
                    let mut up = 0.0f32;
                    let mut lw = 0.0f32;
                    if ln != 0.0f32 {
                        bp = (o[q] - l[q]) / ln;
                        up = (h[q] - o[q].max(c[q])) / ln;
                        lw = (o[q].max(c[q]) - l[q]) / ln;
                    }
                    m0 = m0 + ln;
                    m1 = m1 + bp;
                    m2 = m2 + up;
                    m3 = m3 + lw;
                }
                let pf = pw as f32;
                m0 = m0 / pf;
                m1 = m1 / pf;
                m2 = m2 / pf;
                m3 = m3 / pf;
                let mut s0 = 0.0f32;
                let mut s1 = 0.0f32;
                let mut s2 = 0.0f32;
                let mut s3 = 0.0f32;
                for k2 in 0..pw {
                    let q = t - k2;
                    let ln = (h[q] - l[q]).abs();
                    let mut bp = 0.0f32;
                    let mut up = 0.0f32;
                    let mut lw = 0.0f32;
                    if ln != 0.0f32 {
                        bp = (o[q] - l[q]) / ln;
                        up = (h[q] - o[q].max(c[q])) / ln;
                        lw = (o[q].max(c[q]) - l[q]) / ln;
                    }
                    s0 = s0 + (ln - m0) * (ln - m0);
                    s1 = s1 + (bp - m1) * (bp - m1);
                    s2 = s2 + (up - m2) * (up - m2);
                    s3 = s3 + (lw - m3) * (lw - m3);
                }
                s0 = (s0 / pf).sqrt();
                s1 = (s1 / pf).sqrt();
                s2 = (s2 / pf).sqrt();
                s3 = (s3 / pf).sqrt();
                if c[t] > o[t] {
                    v0 = 1.0f32;
                } else if c[t] < o[t] {
                    v0 = 0.0f32 - 1.0f32;
                }
                let ln = (h[t] - l[t]).abs();
                let mut bp = 0.0f32;
                let mut up = 0.0f32;
                let mut lw = 0.0f32;
                if ln != 0.0f32 {
                    bp = (o[t] - l[t]) / ln;
                    up = (h[t] - o[t].max(c[t])) / ln;
                    lw = (o[t].max(c[t]) - l[t]) / ln;
                }
                if ln != 0.0f32 {
                    if ln <= m0 - s0 * b {
                        v1 = 1.0f32;
                    } else if ln <= m0 + s0 * a {
                        v1 = 2.0f32;
                    } else if ln <= s0 * b {
                        v1 = 3.0f32;
                    } else if ln <= m0 + s0 * _cc {
                        v1 = 4.0f32;
                    } else if ln <= m0 + s0 * t4 {
                        v1 = 5.0f32;
                    } else {
                        v1 = 6.0f32;
                    }
                }
                if bp != 0.0f32 {
                    if bp <= m1 - s1 * a {
                        v2 = 1.0f32;
                    } else if bp <= m1 + s1 * b {
                        v2 = 2.0f32;
                    } else if bp <= m1 + s1 * _cc {
                        v2 = 3.0f32;
                    } else {
                        v2 = 4.0f32;
                    }
                }
                if up != 0.0f32 {
                    if up <= m2 - s2 * a {
                        v3 = 1.0f32;
                    } else if up <= m2 + s2 * b {
                        v3 = 2.0f32;
                    } else {
                        v3 = 3.0f32;
                    }
                }
                if lw != 0.0f32 {
                    if lw <= m3 - s3 * a {
                        v4 = 1.0f32;
                    } else if lw <= m3 + s3 * b {
                        v4 = 2.0f32;
                    } else {
                        v4 = 3.0f32;
                    }
                }
            }
        } else if formula == 1328u32 {
            // oscillator with volume weight `[line, signal, strength]`: lane o = inner oscillator (host-resolved
            // RSI / CMO / PSL / Bias), period = baseline window (>= 2), a = spike threshold (>= 1.01).
            // Signal is 0 until the volume window is full and one more bar has been seen
            let mut bp = period as usize;
            if bp < 2usize {
                bp = 2usize;
            }
            let th = a.max(1.01f32);
            v0 = o[t];
            if t + 1usize > bp {
                let mut bs = 0.0f32;
                for k2 in 1..bp {
                    bs = bs + v[t - k2];
                }
                let bc = (bp - 1usize) as f32;
                let bv = bs / bc;
                let mut vr = 1.0f32;
                if bv > 1.0e-9f32 {
                    vr = v[t] / bv;
                }
                let mut dir = 0.0f32;
                if c[t] > c[t - 1] + 1.0e-9f32 {
                    dir = 1.0f32;
                } else if c[t] < c[t - 1] - 1.0e-9f32 {
                    dir = 0.0f32 - 1.0f32;
                }
                if dir != 0.0f32 {
                    if vr >= th {
                        v1 = dir * 3.0f32;
                    } else if vr > 1.0f32 {
                        v1 = dir * 2.0f32;
                    } else {
                        v1 = dir;
                    }
                }
                v2 = ((vr - 1.0f32) / (th - 1.0f32)).max(0.0f32).min(1.0f32);
            }
        } else if formula == 1329u32 {
            // confluence of up to 5 host-resolved oscillators (lanes o, h, l, c, v): p2 = input count, p3 = first
            // bar at which every input is ready, p4 = Sum threshold, flag = mode (0 All, 1 Any, 2 Majority, 3 Sum)
            if t >= (p3 as usize) {
                let mut pos = 0.0f32;
                let mut neg = 0.0f32;
                let mut first = 0.0f32;
                let mut got = 0u32;
                for i in 0..5usize {
                    if i < (p2 as usize) {
                        let mut x = o[t];
                        if i == 1usize {
                            x = h[t];
                        } else if i == 2usize {
                            x = l[t];
                        } else if i == 3usize {
                            x = c[t];
                        } else if i == 4usize {
                            x = v[t];
                        }
                        let mut sg = 0.0f32;
                        if x > 0.0f32 {
                            sg = 1.0f32;
                            pos = pos + 1.0f32;
                        } else if x < 0.0f32 {
                            sg = 0.0f32 - 1.0f32;
                            neg = neg + 1.0f32;
                        }
                        if sg != 0.0f32 && got == 0u32 {
                            first = sg;
                            got = 1u32;
                        }
                    }
                }
                let nin = p2 as f32;
                if flag == 0u32 {
                    if pos == nin {
                        v0 = 1.0f32;
                    } else if neg == nin {
                        v0 = 0.0f32 - 1.0f32;
                    }
                } else if flag == 1u32 {
                    v0 = first;
                } else if flag == 2u32 {
                    if pos > neg {
                        v0 = 1.0f32;
                    } else if neg > pos {
                        v0 = 0.0f32 - 1.0f32;
                    }
                } else {
                    let sm = pos - neg;
                    let th = p4 as f32;
                    if sm >= th {
                        v0 = 1.0f32;
                    } else if sm <= 0.0f32 - th {
                        v0 = 0.0f32 - 1.0f32;
                    }
                }
            }
        } else if formula == 1330u32 {
            // cross mutual information over lags: host pre-stage runs 1325 once per lag [1, 2, 3, 5, 10]
            v0 = 0.0f32;
        } else if formula == 1331u32 {
            // close-to-close vol percentile `[abs log return, percentile]`: window = `period` (the CPU percentile
            // window, >= 1); bar 0 only records the close. percentile = share of the last min(t, window) abs
            // returns that are <= the current one
            let mut w = period as usize;
            if w < 1usize {
                w = 1usize;
            }
            if t >= 1usize {
                let mut pv = c[t - 1];
                if pv < 1.0e-12f32 {
                    pv = 1.0e-12f32;
                }
                let cur = (c[t] / pv).ln().abs();
                v0 = cur;
                let mut len = t;
                if len > w {
                    len = w;
                }
                let mut le = 0.0f32;
                for q in 0..len {
                    let s = t - q;
                    let mut pp = c[s - 1];
                    if pp < 1.0e-12f32 {
                        pp = 1.0e-12f32;
                    }
                    let vs = (c[s] / pp).ln().abs();
                    if vs <= cur {
                        le = le + 1.0f32;
                    }
                }
                v1 = le / (len as f32);
            }
        } else if formula == 1332u32 {
            // Kalman regime composite: lanes o = Kscr, h = ATR percentile, l = close-vol percentile (host-resolved);
            // weights a / b / c = w_regime / w_atr / w_vov
            let den = (a + b + _cc).max(1.0e-9f32);
            v0 = (a * o[t] + b * (1.0f32 - h[t]) + _cc * (1.0f32 - l[t])) / den;
        }
        out[t] = v0;
        out[n + t] = v1;
        out[2 * n + t] = v2;
        out[3 * n + t] = v3;
        out[4 * n + t] = v4;
        out[5 * n + t] = v5;
        out[6 * n + t] = v6;
        out[7 * n + t] = v7;
        out[8 * n + t] = v8;
        out[9 * n + t] = v9;
    }
}

#[cube(launch_unchecked)]
fn bar_map(
    o: &[f32],
    h: &[f32],
    l: &[f32],
    c: &[f32],
    v: &[f32],
    out: &mut [f32],
    scr: &mut [f32],
    formula: u32,
    period: u32,
    p2: u32,
    p3: u32,
    p4: u32,
    a: f32,
    b: f32,
    cc: f32,
    flag: u32,
) {
    bar_scan(o, h, l, c, v, out, scr, formula, period, p2, p3, p4, a, b, cc, flag);
}

/// Launch the bar scan for a raw code (also used by the internal stage codes 1390..=1399) over the
/// five lanes; returns the flat `10 * n` output (column `k` at `k * n`).
pub(crate) fn bar_run(code: u32, ins: [&Vec<f32>; 5], params: CubeParams, flag: u32) -> Vec<f32> {
    bar_run_x(code, ins, &[], params, flag)
}

/// Narrow order of a scalar-slot oscillator (`+oscillator` members): `0` Rsi, `1` Cmo, `2` Psl, `3` Bias.
fn osc_order(kind: u32, period: usize) -> crate::engine::contract_engine::OscillatorSlotOrder {
    use crate::engine::contract_engine::OscillatorSlotOrder as O;
    use crate::indicators::average::moving_average::PeriodConfig as P;
    let p = P { period: period.max(1) };
    match kind {
        1 => O::Cmo(p),
        2 => O::Psl(p),
        3 => O::Bias(p),
        _ => O::Rsi(p),
    }
}

/// Inner oscillator series from the GPU cube formula of the slot member.
fn osc_series(samples: &[GpuSample], params: CubeParams, kind: u32, period: u32) -> Vec<f32> {
    let f = match kind {
        1 => CubeFormula::Cmo,
        2 => CubeFormula::Psl,
        3 => CubeFormula::Bias,
        _ => CubeFormula::Rsi,
    };
    super::kernels::launch_cube(f, samples, CubeParams { period: period.max(1), ..params })
}

/// First bar index at which the CPU slot reports `is_ready` (readiness depends on the bar count only).
fn osc_ready_index(kind: u32, period: usize) -> u32 {
    let mut s = osc_order(kind, period).into_slot();
    for i in 0..100_000usize {
        s.feed(100.0 + ((i * 7) % 13) as f64);
        if s.is_ready() {
            return i as u32;
        }
    }
    100_000
}

/// [`bar_run`] with extra input series copied to the start of the scratch buffer (read-only for the
/// formula): series `s` of length `n` sits at `s * n`.
pub(crate) fn bar_run_x(code: u32, ins: [&Vec<f32>; 5], extra: &[f32], params: CubeParams, flag: u32) -> Vec<f32> {
    let n = ins[3].len();
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let up = |s: &Vec<f32>| client.create_from_slice(f32::as_bytes(s));
    let out = client.empty(10 * n * core::mem::size_of::<f32>());
    // scratch (histograms / rings / design matrices) sized from the largest window parameter
    let scr_len = ((params.period.max(params.fast).max(params.slow).max(params.signal)) as usize * 28 + (params.fast as usize + 2) * (params.fast as usize + 2) + 64).max(256);
    let scr_len = scr_len.max(extra.len() + 64);
    let mut scr_init = vec![0.0f32; scr_len];
    scr_init[..extra.len()].copy_from_slice(extra);
    let scr_buf = client.create_from_slice(f32::as_bytes(&scr_init));
    unsafe {
        bar_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(up(ins[0]), n),
            BufferArg::from_raw_parts(up(ins[1]), n),
            BufferArg::from_raw_parts(up(ins[2]), n),
            BufferArg::from_raw_parts(up(ins[3]), n),
            BufferArg::from_raw_parts(up(ins[4]), n),
            BufferArg::from_raw_parts(out.clone(), 10 * n),
            BufferArg::from_raw_parts(scr_buf, scr_len),
            code,
            params.period,
            params.fast,
            params.slow,
            params.signal,
            params.a,
            params.b,
            params.c,
            flag,
        );
    }
    let bytes = client.read_one_unchecked(out);
    f32::from_bytes(&bytes).to_vec()
}

/// One internal stage: column 0 of a stage code over the given lanes.
pub(crate) fn bar_stage(code: u32, ins: [&Vec<f32>; 5], params: CubeParams, flag: u32) -> Vec<f32> {
    let n = ins[3].len();
    bar_run(code, ins, params, flag)[0..n].to_vec()
}

/// Run a bar formula of codes 1200..=1399. One `Vec` per output column.
pub fn launch_cube_bar(
    formula: CubeFormula,
    samples: &[GpuSample],
    params: CubeParams,
) -> Vec<Vec<f32>> {
    launch_cube_bar_x(formula, samples, params, None)
}

/// Bar formulas that read the calendar adapter: the open lane carries `GpuTimes::mkey` (the monthly
/// anchor key). Codes 1303..=1306. UNTESTED on GPU.
pub fn launch_cube_bar_timed(
    formula: CubeFormula,
    samples: &[GpuSample],
    times: &super::gpu_sample::GpuTimes,
    params: CubeParams,
) -> Vec<Vec<f32>> {
    assert_eq!(times.mkey.len(), samples.len(), "one GpuTimes row per sample");
    launch_cube_bar_x(formula, samples, params, Some(&times.mkey))
}

fn launch_cube_bar_x(
    formula: CubeFormula,
    samples: &[GpuSample],
    params: CubeParams,
    okey: Option<&Vec<f32>>,
) -> Vec<Vec<f32>> {
    let n = samples.len();
    if n == 0 {
        return Vec::new();
    }
    let o = match okey {
        Some(k) => k.clone(),
        None => lane_series(samples, params, OhlcvField::Open),
    };
    let mut h = lane_series(samples, params, OhlcvField::High);
    let mut l = lane_series(samples, params, OhlcvField::Low);
    let c = lane_series(samples, params, params.lane);
    let mut v = lane_series(samples, params, OhlcvField::Volume);
    let mut o = o;
    if formula == CubeFormula::DpoBandsBar {
        let mut dp = params;
        dp.period = params.period.max(2);
        dp.smoother = super::CubeSmoother::Sma;
        v = super::kernels::launch_cube(CubeFormula::DpoCols, samples, dp);
    }
    if formula == CubeFormula::HurstPctBar || formula == CubeFormula::DfaPctBar {
        // inner series on the device, percentile rank in the scan below
        let mut ip = params;
        let inner = if formula == CubeFormula::HurstPctBar {
            ip.period = params.period.max(50);
            CubeFormula::HurstBar
        } else {
            CubeFormula::DfaBar
        };
        v = launch_cube_bar(inner, samples, ip).swap_remove(0);
    }
    if matches!(formula, CubeFormula::EgAdfBar | CubeFormula::CointBar | CubeFormula::EgCointBar) {
        let w = match formula {
            CubeFormula::EgAdfBar => params.period.max(32),
            _ => params.period.max(20),
        };
        let ma = match formula {
            CubeFormula::EgAdfBar => params.fast.max(5),
            _ => w,
        };
        v = smooth_series(&c, super::CubeSmoother::Sma, ma, 0, params.a, params.b);
    }
    if formula == CubeFormula::KpssZBar {
        let mut ip = params;
        ip.period = params.period;
        v = launch_cube_bar(CubeFormula::KpssBar, samples, ip).swap_remove(0);
    }
    if formula == CubeFormula::AdfKpssBar {
        let mut ap = params;
        ap.period = params.period;
        o = launch_cube_bar(CubeFormula::EgAdfBar, samples, ap).swap_remove(1);
        h = launch_cube_bar(CubeFormula::KpssBar, samples, params).swap_remove(0);
        l = if params.flag == 1 {
            launch_cube_bar(CubeFormula::KpssTrendBar, samples, params).swap_remove(0)
        } else {
            vec![0.0; n]
        };
    }
    if formula == CubeFormula::IftRsiBar || formula == CubeFormula::RsiZscoreBar {
        v = super::kernels::launch_cube(CubeFormula::Rsi, samples, params);
    }
    if formula == CubeFormula::VhfMaBar {
        let ad = bar_stage(1390, [&o, &h, &l, &c, &v], params, 0);
        v = smooth_series(&ad, params.smoother, params.period.max(1), 1, params.a, params.b);
    }
    if formula == CubeFormula::StochRsiBar {
        let rsi = super::kernels::launch_cube(CubeFormula::Rsi, samples, params);
        let raw = bar_stage(1391, [&o, &h, &l, &c, &rsi], params, 0);
        let rp = params.period;
        let sp = params.fast.max(1);
        let kp = params.slow.max(1);
        let dp = params.signal.max(1);
        let k0 = rp + sp + kp - 2;
        let k = smooth_series(&raw, params.smoother, kp, k0, params.a, params.b);
        let d = smooth_series(&k, params.smoother2, dp, k0 + kp - 1, params.a, params.b);
        return vec![k, d];
    }
    if formula == CubeFormula::DssBar {
        let k = bar_stage(1392, [&o, &h, &l, &c, &v], params, 0);
        let sp = params.fast.max(1);
        let s1 = smooth_series(&k, params.smoother, sp, 0, params.a, params.b);
        let s2 = smooth_series(&s1, params.smoother, sp, 0, params.a, params.b);
        return vec![s2];
    }
    if formula == CubeFormula::SmiBar {
        let dr = bar_run(1393, [&o, &h, &l, &c, &v], params, 0);
        let diff = dr[0..n].to_vec();
        let range = dr[n..2 * n].to_vec();
        let p = params.period.max(1);
        let d1 = smooth_series(&diff, params.smoother, p, 0, params.a, params.b);
        let d2 = smooth_series(&d1, params.smoother, p, 0, params.a, params.b);
        let r1 = smooth_series(&range, params.smoother, p, 0, params.a, params.b);
        let r2 = smooth_series(&r1, params.smoother, p, 0, params.a, params.b);
        let line = bar_stage(1394, [&d2, &h, &l, &c, &r2], params, 0);
        let sig = smooth_series(&line, params.smoother2, params.signal.max(1), p - 1, params.a, params.b);
        return vec![line, sig];
    }
    if formula == CubeFormula::StcBar {
        let fast = super::kernels_comp::sm(&c, params.smoother, params.fast, 0, params);
        let slow = super::kernels_comp::sm(&c, params.smoother2, params.slow, 0, params);
        let line = super::kernels_comp::ew2(2, &fast, &slow, 0.0);
        let rdy = params.fast.max(1).max(params.slow.max(1)) - 1;
        let sig = super::kernels_comp::sm(&line, params.smoother3, 9, rdy, params);
        let raw = super::kernels_comp::ew2(2, &line, &sig, 0.0);
        let ks = smooth_series(&raw, params.smoother, params.period.max(1), 0, params.a, params.b);
        let ds = smooth_series(&ks, params.smoother2, params.smooth_period.max(1), 0, params.a, params.b);
        let k = bar_stage(1395, [&o, &h, &l, &c, &ks], params, 0);
        let d = bar_stage(1395, [&o, &h, &l, &c, &ds], params, 0);
        return vec![k, d];
    }
    if formula == CubeFormula::SuptsBar {
        return vec![super::kernels::launch_cube(CubeFormula::Supertrend, samples, params)];
    }
    if formula == CubeFormula::KeltsBar {
        let tp = lane_series(samples, params, OhlcvField::HLC3);
        let mid = smooth_series(&tp, params.smoother, params.period.max(1), 0, params.a, params.b);
        let atr = super::kernels_comp::atr_series(samples, params, params.smoother2, params.period.max(1), 0);
        return vec![super::kernels_comp::ewc(14, &mid, &atr, &mid, params.a, 0.0)];
    }
    if formula == CubeFormula::GmmaBar {
        let mut extra: Vec<f32> = Vec::with_capacity(12 * n);
        for per in [3u32, 5, 8, 10, 12, 15, 30, 35, 40, 45, 50, 60] {
            extra.extend(smooth_series(&c, params.smoother, per, 0, params.a, params.b));
        }
        let flat = bar_run_x(formula.code(), [&o, &h, &l, &c, &v], &extra, params, 0);
        return vec![flat[0..n].to_vec()];
    }
    if formula == CubeFormula::KregimeBar {
        let k = super::kernels_comp::kalman_all(&c, params.a, params.b, params.c, params.flag != 0);
        let flat = bar_run(formula.code(), [&o, &h, &l, &c, &k[1]], params, 0);
        return vec![flat[0..n].to_vec()];
    }
    if formula == CubeFormula::AmatBar {
        let fast = smooth_series(&c, params.smoother, params.fast.max(1), 0, params.a, params.b);
        let slow = smooth_series(&c, params.smoother2, params.slow.max(1), 0, params.a, params.b);
        let flat = bar_run(formula.code(), [&slow, &h, &l, &c, &fast], params, 0);
        return vec![flat[0..n].to_vec()];
    }
    if formula == CubeFormula::ElderImpulseBar {
        let ema = smooth_series(&c, params.smoother, params.period.max(2), 0, params.a, params.b);
        let flat = bar_run(formula.code(), [&o, &h, &l, &c, &ema], params, 0);
        return vec![flat[0..n].to_vec()];
    }
    if formula == CubeFormula::UoSmoothBar {
        let mut up = params;
        up.fast = params.fast;
        let u = bar_stage(1279, [&o, &h, &l, &c, &v], up, 0);
        return vec![smooth_series(&u, params.smoother, params.signal.max(1), 0, params.a, params.b)];
    }
    if formula == CubeFormula::AtrRsiBar {
        let atr = super::kernels_comp::atr_series(samples, params, super::CubeSmoother::Rma, params.fast.max(1), 0);
        let base = smooth_series(&atr, params.smoother, params.slow.max(1), 0, params.a, params.b);
        let flat = bar_run(formula.code(), [&base, &h, &l, &c, &atr], params, 0);
        return vec![flat[0..n].to_vec()];
    }
    if formula == CubeFormula::OscVolWeightBar {
        let inner = osc_series(samples, params, params.flag, params.fast);
        let flat = bar_run(formula.code(), [&inner, &h, &l, &c, &v], params, 0);
        return vec![flat[0..n].to_vec(), flat[n..2 * n].to_vec(), flat[2 * n..3 * n].to_vec()];
    }
    if formula == CubeFormula::ConfluenceBar {
        // `ext[i]` = kind * 1_000_000 + period + 1 per input (0 = absent), at most 5
        let mut series: Vec<Vec<f32>> = Vec::new();
        let mut ready = 0u32;
        for e in params.ext.iter().take(5) {
            if *e == 0 {
                continue;
            }
            let (kind, period) = (*e / 1_000_000, (*e % 1_000_000) - 1);
            series.push(osc_series(samples, params, kind, period));
            ready = ready.max(osc_ready_index(kind, period as usize));
        }
        let cnt = series.len();
        while series.len() < 5 {
            series.push(vec![0.0; n]);
        }
        let p = CubeParams { fast: cnt as u32, slow: ready, ..params };
        let flat = bar_run(formula.code(), [&series[0], &series[1], &series[2], &series[3], &series[4]], p, 0);
        return vec![flat[0..n].to_vec()];
    }
    if formula == CubeFormula::KcompBar {
        // period = Kscr window, a / b / c = Kalman dt / q / r, fast = ATR period (SMA), slow = ATR percentile
        // window, signal = close-vol percentile window, ext[0..3] = w_regime / w_atr / w_vov in 1e-6
        let k = super::kernels::launch_cube(CubeFormula::KscrComp, samples, params);
        let ap = CubeParams {
            period: params.fast,
            smoother: super::CubeSmoother::Sma,
            smooth_period: params.fast,
            ..params
        };
        let atrp = super::kernels::launch_cube(CubeFormula::AtrPct, samples, ap);
        let cv = bar_run(1331, [&o, &h, &l, &c, &v], CubeParams { period: params.signal.max(1), ..params }, 0);
        let cvp = cv[n..2 * n].to_vec();
        let w = |i: usize| params.ext[i] as f32 / 1.0e6;
        let wp = CubeParams { a: w(0), b: w(1), c: w(2), ..params };
        let flat = bar_run(formula.code(), [&k, &atrp, &cvp, &c, &v], wp, 0);
        return vec![flat[0..n].to_vec()];
    }
    if formula == CubeFormula::XmilBar {
        // window = period, bins = slow, clip = a; lags are the CPU factory's fixed [1, 2, 3, 5, 10]
        let mut cols = Vec::new();
        for lag in [1u32, 2, 3, 5, 10] {
            let w = params.period.max(2);
            let p = CubeParams { fast: lag.min(w - 1), signal: params.signal.max(200), ..params };
            let flat = bar_run(1325, [&o, &h, &l, &c, &v], p, 0);
            cols.push(flat[0..n].to_vec());
        }
        return cols;
    }
    if formula == CubeFormula::MinfoBar || formula == CubeFormula::TeBar {
        let flat = bar_run(formula.code(), [&o, &h, &l, &c, &v], CubeParams { signal: params.signal.max(200), ..params }, 0);
        return vec![flat[0..n].to_vec()];
    }
    if formula == CubeFormula::WaveBar {
        let flat = bar_run(formula.code(), [&o, &h, &l, &c, &v], CubeParams { slow: params.slow.max(64), ..params }, 0);
        return vec![flat[0..n].to_vec()];
    }
    if formula == CubeFormula::QqeBar {
        let rp = params.period.max(1);
        let sp = params.fast.max(1);
        let rsi = super::kernels::launch_cube(CubeFormula::Rsi, samples, CubeParams { period: rp, ..params });
        let sm = smooth_series(&rsi, params.smoother, sp, 0, params.a, params.b);
        let delta = bar_stage(1292, [&o, &h, &l, &c, &sm], params, 0);
        let atr_p = (((sp as f32) * 4.236f32).round() as u32).max(2);
        let atr = smooth_series(&delta, params.smoother2, atr_p, 0, params.a, params.b);
        let flat = bar_run(formula.code(), [&atr, &h, &l, &c, &sm], params, 0);
        return vec![flat[0..n].to_vec(), flat[n..2 * n].to_vec()];
    }
    if formula == CubeFormula::SqmomBar {
        let bb = smooth_series(&c, params.smoother, params.period.max(1), 0, params.a, params.b);
        let kc = smooth_series(&c, params.smoother2, params.fast.max(1), 0, params.a, params.b);
        let atr = super::kernels_comp::atr_series(samples, params, super::CubeSmoother::Sma, params.fast.max(1), 1);
        let flat = bar_run(formula.code(), [&bb, &kc, &atr, &c, &c], CubeParams { fast: params.fast.max(1), slow: params.slow.max(1), ..params }, 0);
        return vec![flat[0..n].to_vec(), flat[n..2 * n].to_vec()];
    }
    if formula == CubeFormula::AlligatorBar {
        let mid = lane_series(samples, params, OhlcvField::HL2);
        let jaw = smooth_series(&mid, super::CubeSmoother::Sma, 13, 0, params.a, params.b);
        let teeth = smooth_series(&mid, super::CubeSmoother::Sma, 8, 0, params.a, params.b);
        let lips = smooth_series(&mid, super::CubeSmoother::Sma, 5, 0, params.a, params.b);
        let flat = bar_run(formula.code(), [&jaw, &teeth, &lips, &c, &c], params, 0);
        return vec![flat[0..n].to_vec(), flat[n..2 * n].to_vec(), flat[2 * n..3 * n].to_vec()];
    }
    if formula == CubeFormula::RelTrendPosBar {
        let sm = smooth_series(&c, params.smoother, params.period.max(1), 0, params.a, params.b);
        let flat = bar_run(formula.code(), [&sm, &h, &l, &c, &v], params, 0);
        return vec![flat[0..n].to_vec(), flat[n..2 * n].to_vec()];
    }
    if formula == CubeFormula::SweepRevBar {
        let atr = super::kernels_comp::atr_series(samples, params, params.smoother, params.fast.max(1), 0);
        let flat = bar_run(formula.code(), [&o, &h, &l, &c, &atr], params, params.flag);
        return vec![flat[0..n].to_vec()];
    }
    if formula == CubeFormula::VrAggBar || formula == CubeFormula::VrZAggBar {
        let mut vp = [params; 3];
        let (ws, ms): ([u32; 3], [u32; 3]) = if formula == CubeFormula::VrAggBar {
            ([params.period; 3], [params.fast, params.slow, params.signal])
        } else {
            ([params.period, params.ext[0], params.ext[2]], [params.fast, params.ext[1], params.ext[3]])
        };
        let mut sers: Vec<Vec<f32>> = Vec::new();
        for i in 0..3 {
            vp[i].period = ws[i];
            vp[i].fast = ms[i];
            sers.push(super::kernels::launch_cube(CubeFormula::VarianceRatio, samples, vp[i]));
        }
        let kp = CubeParams { signal: params.ext[4], ..params };
        let flat = bar_run(formula.code(), [&sers[0], &sers[1], &sers[2], &c, &c], kp, 0);
        return vec![flat[0..n].to_vec()];
    }
    if formula == CubeFormula::ConnorsRsiBar {
        // the CPU RSI is first fed on bar 1, so run it on the shifted series
        let mut rsi = vec![0.0f32];
        if n > 1 {
            rsi.extend(super::kernels::launch_cube(CubeFormula::Rsi, &samples[1..], params));
        }
        let flat = bar_run(formula.code(), [&rsi, &h, &l, &c, &v], params, 0);
        return vec![flat[0..n].to_vec()];
    }
    if formula == CubeFormula::AdaptiveStochBar {
        let atr = super::kernels_comp::atr_series(samples, params, super::CubeSmoother::Rma, params.slow.max(1), 0);
        let flat = bar_run(formula.code(), [&atr, &h, &l, &c, &v], params, 0);
        let k = flat[0..n].to_vec();
        let d = smooth_series(&k, params.smoother, params.signal.max(1), 0, params.a, params.b);
        return vec![k, d];
    }
    if formula == CubeFormula::EwmacRobustBar {
        let fast = smooth_series(&c, params.smoother, params.fast.max(1), 0, params.a, params.b);
        let slow = smooth_series(&c, params.smoother2, params.slow.max(1), 0, params.a, params.b);
        let flat = bar_run(formula.code(), [&slow, &h, &l, &c, &fast], params, 0);
        return vec![flat[0..n].to_vec()];
    }
    if formula == CubeFormula::TdiBar {
        let rsi = super::kernels::launch_cube(CubeFormula::Rsi, samples, params);
        let sig = smooth_series(&rsi, params.smoother, params.signal.max(1), 0, params.a, params.b);
        let mut bp = params;
        bp.fast = params.fast.max(1);
        let basis = bar_stage(1261, [&o, &h, &l, &c, &rsi], CubeParams { slow: params.fast, ..params }, 0);
        return vec![rsi, sig, basis];
    }
    if formula == CubeFormula::JmaBar {
        let pp = params.period.max(1);
        let fast = smooth_series(&c, super::CubeSmoother::Ema, pp, 0, params.a, params.b);
        let slow = smooth_series(&c, super::CubeSmoother::Ema, (pp * 2).max(2), 0, params.a, params.b);
        let mut sp = params;
        sp.a = (params.c.max(-100.0).min(100.0) + 100.0) / 200.0;
        return vec![bar_stage(1396, [&fast, &h, &l, &c, &slow], sp, 0)];
    }
    if formula == CubeFormula::VidyaBar {
        let mut cp = params;
        cp.smoother = params.smoother;
        v = super::kernels::launch_cube(CubeFormula::Cmo, samples, cp);
    }
    if formula == CubeFormula::EhlersRocketBar {
        let sp = smooth_series(&c, super::CubeSmoother::Ema, 3, 0, params.a, params.b);
        let rsi = bar_stage(1397, [&o, &h, &l, &c, &sp], params, 0);
        let mom = bar_stage(1398, [&o, &h, &l, &c, &rsi], params, 0);
        let sm = smooth_series(&mom, params.smoother, params.fast.max(1), 2, params.a, params.b);
        let mut fp = params;
        fp.a = params.a.max(1.0e-6).min(1.0);
        return vec![bar_stage(1399, [&sm, &h, &l, &c, &rsi], fp, 0)];
    }
    if formula == CubeFormula::KamaSlopeBar {
        let mut kp = params;
        kp.period = params.period.max(2);
        kp.fast = 2;
        kp.slow = 30;
        let k = launch_cube_bar(CubeFormula::KamaBar, samples, kp).swap_remove(0);
        return vec![bar_stage(1389, [&o, &h, &l, &c, &k], params, 0)];
    }
    if formula == CubeFormula::RvzBar {
        let mut vp = params;
        vp.period = params.period.max(2);
        let vol = bar_stage(1387, [&o, &h, &l, &c, &v], vp, 0);
        let z = bar_stage(1386, [&o, &h, &l, &c, &vol], CubeParams { fast: params.fast.max(2), ..params }, 0);
        return vec![vol, z];
    }
    if formula == CubeFormula::DistLevelsBar {
        let mut mp = params;
        mp.period = params.period.max(1);
        o = super::kernels::launch_cube(CubeFormula::Rmid, samples, mp);
        let mut pp = params;
        pp.lane = OhlcvField::Close;
        pp.slow = params.slow;
        v = super::kernels::launch_cube_columns(CubeFormula::PctChannels, samples, pp).swap_remove(1);
    }
    if formula == CubeFormula::VoltsBar {
        let tp = lane_series(samples, params, OhlcvField::HLC3);
        v = smooth_series(&tp, params.smoother, params.period.max(1), 0, params.a, params.b);
        if params.flag == 1 {
            let mut ap = params;
            ap.period = params.period.max(1);
            ap.smoother = super::CubeSmoother::Rma;
            o = super::kernels::launch_cube(CubeFormula::Atr, samples, ap);
        }
    }
    if formula == CubeFormula::VprbBar {
        // ATR (EMA smoothed) runs on the device first and rides in the volume slot
        let mut ap = params;
        ap.smoother = super::CubeSmoother::Ema;
        v = super::kernels::launch_cube(CubeFormula::Atr, samples, ap)
            .into_iter()
            .map(|x| x.max(1e-12))
            .collect();
    }
    let flag_arg = if formula == CubeFormula::HurstPctBar {
        params.period.max(50)
    } else if formula == CubeFormula::DfaPctBar {
        params.flag.max(50)
    } else {
        params.flag
    };
    let flat = bar_run(formula.code(), [&o, &h, &l, &c, &v], params, flag_arg);
    let cols = formula.output_count() as usize;
    let mut res: Vec<Vec<f32>> = (0..cols).map(|k| flat[k * n..(k + 1) * n].to_vec()).collect();
    if formula == CubeFormula::PriceChanBar && params.flag == 1 {
        // smoothed mode: device smoother passes over the rolling extremes; middle is their mean
        let sp = params.period.max(1);
        let up = smooth_series(&res[0], params.smoother, sp, 0, params.a, params.b);
        let lo = smooth_series(&res[2], params.smoother, sp, 0, params.a, params.b);
        res[1] = up.iter().zip(lo.iter()).map(|(u, l)| (u + l) / 2.0).collect();
        res[0] = up;
        res[2] = lo;
    }
    if formula == CubeFormula::HalfLifeBar {
        // +inf on the CPU is carried as a huge sentinel through the kernel
        for x in res[0].iter_mut() {
            if *x >= 3.0e38 {
                *x = f32::INFINITY;
            }
        }
    }
    res
}
