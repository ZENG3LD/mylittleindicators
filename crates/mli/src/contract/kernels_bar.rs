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
    let n = samples.len();
    if n == 0 {
        return Vec::new();
    }
    let o = lane_series(samples, params, OhlcvField::Open);
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
