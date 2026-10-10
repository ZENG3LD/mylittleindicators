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

/// 1200 Stochastik-D `[k, d]` (`period` %K window, `p2` %D window), 1201 Donchian stop
/// (lower stop: `period` lower window, `p2` upper window unused in the output, `a` offset, `flag`
/// percentage), 1202 projection bands `[upper, middle, lower]` (`period` window clamped 2..=512,
/// `a` k, ring-storage order like the CPU).
#[cube]
fn bar_scan(
    _o: &[f32],
    h: &[f32],
    l: &[f32],
    c: &[f32],
    v: &[f32],
    out: &mut [f32],
    formula: u32,
    period: u32,
    p2: u32,
    p3: u32,
    p4: u32,
    a: f32,
    _b: f32,
    _cc: f32,
    flag: u32,
) {
    let n = c.len();
    let mut k = 0.0f32;
    let mut d = 0.0f32;
    let mut dh = 0.0f32;
    let mut dl = 0.0f32;
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
                vol = _o[t];
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
        }
        out[t] = v0;
        out[n + t] = v1;
        out[2 * n + t] = v2;
        out[3 * n + t] = v3;
        out[4 * n + t] = v4;
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
    bar_scan(o, h, l, c, v, out, formula, period, p2, p3, p4, a, b, cc, flag);
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
    let h = lane_series(samples, params, OhlcvField::High);
    let l = lane_series(samples, params, OhlcvField::Low);
    let c = lane_series(samples, params, params.lane);
    let mut v = lane_series(samples, params, OhlcvField::Volume);
    let mut o = o;
    if formula == CubeFormula::DpoBandsBar {
        let mut dp = params;
        dp.period = params.period.max(2);
        dp.smoother = super::CubeSmoother::Sma;
        v = super::kernels::launch_cube(CubeFormula::DpoCols, samples, dp);
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
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let up = |s: &Vec<f32>| client.create_from_slice(f32::as_bytes(s));
    let out = client.empty(5 * n * core::mem::size_of::<f32>());
    unsafe {
        bar_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(up(&o), n),
            BufferArg::from_raw_parts(up(&h), n),
            BufferArg::from_raw_parts(up(&l), n),
            BufferArg::from_raw_parts(up(&c), n),
            BufferArg::from_raw_parts(up(&v), n),
            BufferArg::from_raw_parts(out.clone(), 5 * n),
            formula.code(),
            params.period,
            params.fast,
            params.slow,
            params.signal,
            params.a,
            params.b,
            params.c,
            params.flag,
        );
    }
    let bytes = client.read_one_unchecked(out);
    let flat = f32::from_bytes(&bytes).to_vec();
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
    res
}
