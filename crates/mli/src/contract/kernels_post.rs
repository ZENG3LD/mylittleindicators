//! Composite GPU formulas: codes 500..=699.
//!
//! A composite is `post(inner(bars))`. The inner series comes from an existing launch
//! ([`super::kernels::launch_cube`] on another [`CubeFormula`]), is read back, and a
//! post kernel runs over it. Each stage is its own launch, so no shader holds more than one
//! stage's arms. Everything here is UNTESTED on GPU (no GPU on the authoring box).
//!
//! Parameter convention for composites (all in [`CubeParams`]):
//! - the inner formula reads `period` / `smoother` / `smooth_period` / lanes as it does alone;
//! - `slow` is the rolling window of the post stage (`w`);
//! - `a` is the post coefficient (EMA alpha for the trend ops).
//!
//! Post ops (`op`):
//! - 0 share of the last `w` values (partial window while filling) that are `<=` the current one;
//! - 1 population z-score over the last `w` values (0 when fewer than 2 or std <= 1e-12);
//! - 2 op 0, then `p - ema(p)` with `alpha = a` seeded by the first value;
//! - 3 identity.

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::gpu::CubeParams;
use super::gpu_sample::GpuSample;
use super::kernels::launch_cube;
use super::CubeFormula;

#[cube]
fn sqrt_p(x: f32) -> f32 {
    let mut y = 0.0f32;
    if x > 0.0f32 {
        y = x;
        for _step in 0..12 {
            y = 0.5f32 * (y + x / y);
        }
    }
    y
}

/// Post stage over one series. See the module doc for `op`.
#[cube]
fn post_scan(src: &[f32], out: &mut [f32], op: u32, w: u32, a: f32) {
    let n = src.len();
    let mut wi = w as usize;
    if wi < 1 {
        wi = 1;
    }
    let mut ema = 0.0f32;
    for i in 0..n {
        let mut len = wi;
        if len > i + 1 {
            len = i + 1;
        }
        let start = i + 1 - len;
        let cur = src[i];
        let mut v = cur;
        if op == 0u32 || op == 2u32 {
            let mut cnt = 0u32;
            for j in start..(i + 1) {
                if src[j] <= cur {
                    cnt = cnt + 1u32;
                }
            }
            v = (cnt as f32) / (len as f32);
            if op == 2u32 {
                if i == 0 {
                    ema = v;
                }
                ema = a * v + (1.0f32 - a) * ema;
                v = v - ema;
            }
        } else if op == 1u32 {
            v = 0.0f32;
            if len >= 2 {
                let mut sum = 0.0f32;
                for j in start..(i + 1) {
                    sum = sum + src[j];
                }
                let lf = len as f32;
                let mean = sum / lf;
                let mut acc = 0.0f32;
                for j in start..(i + 1) {
                    let d = src[j] - mean;
                    acc = acc + d * d;
                }
                let sd = sqrt_p(acc / lf);
                if sd > 1.0e-12f32 {
                    v = (cur - mean) / sd;
                }
            }
        }
        out[i] = v;
    }
}

#[cube(launch_unchecked)]
fn post_map(src: &[f32], out: &mut [f32], op: u32, w: u32, a: f32) {
    post_scan(src, out, op, w, a);
}

/// Run one post op over `series` on the device.
pub(crate) fn run_post(series: &[f32], op: u32, w: u32, a: f32) -> Vec<f32> {
    let n = series.len();
    if n == 0 {
        return Vec::new();
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let src = client.create_from_slice(f32::as_bytes(series));
    let out = client.empty(n * core::mem::size_of::<f32>());
    unsafe {
        post_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(src, n),
            BufferArg::from_raw_parts(out.clone(), n),
            op,
            w,
            a,
        );
    }
    let bytes = client.read_one_unchecked(out);
    f32::from_bytes(&bytes).to_vec()
}


/// k-th smallest (0-based, clamped to the last) of `src[start..start + len]`.
/// O(len^2) rank count: the value `x` with `count(< x) <= k < count(<= x)`.
#[cube]
fn kth_in(src: &[f32], start: usize, len: usize, k: u32) -> f32 {
    let mut kk = k;
    if kk >= len as u32 {
        kk = (len - 1) as u32;
    }
    let mut res = src[start];
    for j in 0..len {
        let x = src[start + j];
        let mut lt = 0u32;
        let mut le = 0u32;
        for m in 0..len {
            let y = src[start + m];
            if y < x {
                lt = lt + 1u32;
            }
            if y <= x {
                le = le + 1u32;
            }
        }
        if lt <= kk && kk < le {
            res = x;
        }
    }
    res
}

/// Second post stage with `cols` output columns (`out[c * n + i]`).
/// Ops: 4 percentile rank 0..100 (`(below + 0.5 equal) / n * 100`, held at 50 until
/// `i >= rf` and the window is full), 10 quartiles `[q1, q2, q3]` (`len/4, len/2, 3len/4`),
/// 11 percentile channel `[upper, middle, lower]` with quantiles `b` (upper) and `a` (lower),
/// 12 RSI percentile bands `[upper, middle, lower]` (20/80 percent order statistics once
/// the window is full and `i >= rf`, else 80 / value / 20).
#[cube]
fn post_scan2(src: &[f32], out: &mut [f32], op: u32, w: u32, a: f32, b: f32, rf: u32) {
    let n = src.len();
    let mut wi = w as usize;
    if wi < 1 {
        wi = 1;
    }
    for i in 0..n {
        let mut len = wi;
        if len > i + 1 {
            len = i + 1;
        }
        let start = i + 1 - len;
        let cur = src[i];
        let full = i + 1 >= wi;
        let ready = full && (i as u32) >= rf;
        if op == 4u32 {
            let mut v = 50.0f32;
            if ready {
                let mut below = 0.0f32;
                let mut equal = 0.0f32;
                for j in start..(i + 1) {
                    if src[j] < cur {
                        below = below + 1.0f32;
                    } else if src[j] == cur {
                        equal = equal + 1.0f32;
                    }
                }
                v = ((below + 0.5f32 * equal) / (len as f32)) * 100.0f32;
            }
            out[i] = v;
        } else if op == 10u32 {
            out[i] = kth_in(src, start, len, (len / 4) as u32);
            out[n + i] = kth_in(src, start, len, (len / 2) as u32);
            out[2 * n + i] = kth_in(src, start, len, ((3 * len) / 4) as u32);
        } else if op == 11u32 {
            let lf = (len - 1) as f32;
            let lo_pos = (a * lf + 0.5f32) as u32;
            let hi_pos = (b * lf + 0.5f32) as u32;
            let lo = kth_in(src, start, len, lo_pos);
            let hi = kth_in(src, start, len, hi_pos);
            out[i] = hi;
            out[n + i] = 0.5f32 * (lo + hi);
            out[2 * n + i] = lo;
        } else if op == 12u32 {
            let mut up = 80.0f32;
            let mut lo = 20.0f32;
            if ready {
                lo = kth_in(src, start, len, ((len * 20) / 100) as u32);
                up = kth_in(src, start, len, ((len * 80) / 100) as u32);
            }
            out[i] = up;
            out[n + i] = cur;
            out[2 * n + i] = lo;
        }
    }
}

#[cube(launch_unchecked)]
fn post_map2(src: &[f32], out: &mut [f32], op: u32, w: u32, a: f32, b: f32, rf: u32) {
    post_scan2(src, out, op, w, a, b, rf);
}

/// Run `post_scan2` over `series`; returns `cols` columns.
pub(crate) fn run_post2(
    series: &[f32],
    op: u32,
    w: u32,
    a: f32,
    b: f32,
    rf: u32,
    cols: usize,
) -> Vec<Vec<f32>> {
    let n = series.len();
    if n == 0 {
        return Vec::new();
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let src = client.create_from_slice(f32::as_bytes(series));
    let out = client.empty(n * cols * core::mem::size_of::<f32>());
    unsafe {
        post_map2::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(src, n),
            BufferArg::from_raw_parts(out.clone(), n * cols),
            op,
            w,
            a,
            b,
            rf,
        );
    }
    let bytes = client.read_one_unchecked(out);
    let flat = f32::from_bytes(&bytes).to_vec();
    (0..cols).map(|c| flat[c * n..(c + 1) * n].to_vec()).collect()
}

fn abs_all(v: &mut [f32]) {
    for x in v.iter_mut() {
        if *x < 0.0 {
            *x = -*x;
        }
    }
}

/// Inner series, post op, window and coefficient of one composite formula.
fn plan(
    formula: CubeFormula,
    samples: &[GpuSample],
    params: CubeParams,
) -> (Vec<f32>, u32, u32, f32) {
    let alpha = params.a.clamp(0.01, 1.0);
    match formula {
        // ATR percentile: |ATR| (smoother slot), share of the window <= now.
        CubeFormula::AtrPct => {
            let mut s = launch_cube(CubeFormula::AtrSm, samples, params);
            abs_all(&mut s);
            (s, 0, params.slow, 0.0)
        }
        CubeFormula::AtrPctTrend => {
            let mut s = launch_cube(CubeFormula::AtrSm, samples, params);
            abs_all(&mut s);
            (s, 2, params.slow, alpha)
        }
        // ATR z-score: window at least 2, ATR not abs'd.
        CubeFormula::AtrZ => {
            let s = launch_cube(CubeFormula::AtrSm, samples, params);
            (s, 1, params.slow.max(2), 0.0)
        }
        CubeFormula::VovPct => {
            let mut s = launch_cube(CubeFormula::VolOfVol, samples, params);
            abs_all(&mut s);
            (s, 0, params.slow, 0.0)
        }
        CubeFormula::VovPctTrend => {
            let mut s = launch_cube(CubeFormula::VolOfVol, samples, params);
            abs_all(&mut s);
            (s, 2, params.slow, alpha)
        }
        _ => (Vec::new(), 3, 1, 0.0),
    }
}

/// Launch a single-output composite (codes 500..=599).
pub fn launch_cube_post(
    formula: CubeFormula,
    samples: &[GpuSample],
    params: CubeParams,
) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    if formula == CubeFormula::RsiPctRank {
        return launch_cube_post_columns(formula, samples, params).swap_remove(0);
    }
    let (series, op, w, a) = plan(formula, samples, params);
    run_post(&series, op, w, a)
}

/// Inner series and post-2 settings of the formulas that go through [`run_post2`].
fn plan2(
    formula: CubeFormula,
    samples: &[GpuSample],
    params: CubeParams,
) -> (Vec<f32>, u32, u32, f32, f32, u32, usize) {
    match formula {
        // RSI percentile rank: RSI(`period`), window `slow` clamped to 5..=1024.
        CubeFormula::RsiPctRank => {
            let s = launch_cube(CubeFormula::Rsi, samples, params);
            (s, 4, params.slow.clamp(5, 1024), 0.0, 0.0, params.period, 1)
        }
        // Rolling quartiles of the lane (`lane`), window `slow`.
        CubeFormula::RollQuart => {
            let s = launch_cube(CubeFormula::Identity, samples, params);
            (s, 10, params.slow.max(1), 0.0, 0.0, 0, 3)
        }
        // Percentile channels: quantiles `a` (lower) and `b` (upper), clamped to 0..1.
        CubeFormula::PctChannels => {
            let s = launch_cube(CubeFormula::Identity, samples, params);
            (s, 11, params.slow.max(1), params.a.clamp(0.0, 1.0), params.b.clamp(0.0, 1.0), 0, 3)
        }
        // RSI percentile bands: RSI(`period`), window `slow` clamped to 10..=1024.
        CubeFormula::RsiPctBands => {
            let s = launch_cube(CubeFormula::Rsi, samples, params);
            (s, 12, params.slow.clamp(10, 1024), 0.0, 0.0, params.period, 3)
        }
        _ => (Vec::new(), 3, 1, 0.0, 0.0, 0, 1),
    }
}

/// Launch a multi-column composite (codes 600..=699) or a second-stage single column.
pub fn launch_cube_post_columns(
    formula: CubeFormula,
    samples: &[GpuSample],
    params: CubeParams,
) -> Vec<Vec<f32>> {
    if samples.is_empty() {
        return Vec::new();
    }
    let (series, op, w, a, b, rf, cols) = plan2(formula, samples, params);
    run_post2(&series, op, w, a, b, rf, cols)
}

/// Signal stage over one series: outputs -1 / 0 / 1 (the i8 signals packed as f32).
/// Ops: 20 direction of change (0 on bar 0 and on ties); 21 regime gate (`flag` 0 above / 1 below
/// the threshold `a`; +1 on entry, -1 on exit, 0 on bar 0 and otherwise); 22 threshold edge
/// (`flag` 0 above `b`... see below, `a` upper, `b` lower); 23 RSI threshold gate (`a` upper,
/// `b` lower, ready from bar `rf`, sticky between bars); 24 hysteresis gate (same
/// thresholds, sticky state); 25 volume spike (`w` window, `a` multiplier); 26 slope direction
/// (compares with the previous value, previous starts at 0).
/// Threshold kinds for op 22 in `flag`: 0 above `a`, 1 below `b`, 2 in `[b, a]`, 3 outside.
#[cube]
fn post_scan3(src: &[f32], out: &mut [f32], op: u32, w: u32, a: f32, b: f32, rf: u32, flag: u32) {
    let n = src.len();
    let mut wi = w as usize;
    if wi < 1 {
        wi = 1;
    }
    let mut in_regime = false;
    let mut last_state = false;
    let mut sticky = 0.0f32;
    let mut prev_v = 0.0f32;
    for i in 0..n {
        let v = src[i];
        let mut res = 0.0f32;
        if op == 20u32 {
            if i > 0 {
                let p = src[i - 1];
                if v > p {
                    res = 1.0f32;
                } else if v < p {
                    res = -1.0f32;
                }
            }
        } else if op == 21u32 {
            let mut now_in = v > a;
            if flag == 1u32 {
                now_in = v < a;
            }
            if i > 0 {
                if !in_regime && now_in {
                    res = 1.0f32;
                } else if in_regime && !now_in {
                    res = -1.0f32;
                }
            }
            in_regime = now_in;
        } else if op == 22u32 {
            let mut cur = v > a;
            if flag == 1u32 {
                cur = v < b;
            } else if flag == 2u32 {
                cur = v >= b && v <= a;
            } else if flag == 3u32 {
                cur = v < b || v > a;
            }
            if i > 0 {
                if !last_state && cur {
                    res = -1.0f32;
                    if v >= b {
                        res = 1.0f32;
                    }
                } else if last_state && !cur {
                    res = 1.0f32;
                    if v < b {
                        res = -1.0f32;
                    }
                }
            }
            last_state = cur;
        } else if op == 23u32 {
            if (i as u32) >= rf {
                if v >= a {
                    sticky = 1.0f32;
                } else if v <= b {
                    sticky = -1.0f32;
                } else {
                    sticky = 0.0f32;
                }
            }
            res = sticky;
        } else if op == 24u32 {
            if (i as u32) >= rf {
                if sticky <= 0.0f32 && v >= a {
                    sticky = 1.0f32;
                } else if sticky >= 0.0f32 && v <= b {
                    sticky = -1.0f32;
                }
            }
            res = sticky;
        } else if op == 25u32 {
            if i + 1 >= wi {
                let start = i + 1 - wi;
                let mut sum = 0.0f32;
                for j in start..(i + 1) {
                    sum = sum + src[j];
                }
                let mean = sum / (wi as f32);
                if mean > 0.0f32 && v > a * mean {
                    res = 1.0f32;
                }
            }
        } else if op == 26u32 {
            if v > prev_v {
                res = 1.0f32;
            } else if v < prev_v {
                res = -1.0f32;
            }
            prev_v = v;
        }
        out[i] = res;
    }
}

#[cube(launch_unchecked)]
fn post_map3(src: &[f32], out: &mut [f32], op: u32, w: u32, a: f32, b: f32, rf: u32, flag: u32) {
    post_scan3(src, out, op, w, a, b, rf, flag);
}

pub(crate) fn run_post3(
    series: &[f32],
    op: u32,
    w: u32,
    a: f32,
    b: f32,
    rf: u32,
    flag: u32,
) -> Vec<f32> {
    let n = series.len();
    if n == 0 {
        return Vec::new();
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let src = client.create_from_slice(f32::as_bytes(series));
    let out = client.empty(n * core::mem::size_of::<f32>());
    unsafe {
        post_map3::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(src, n),
            BufferArg::from_raw_parts(out.clone(), n),
            op,
            w,
            a,
            b,
            rf,
            flag,
        );
    }
    let bytes = client.read_one_unchecked(out);
    f32::from_bytes(&bytes).to_vec()
}

/// Launch a signal formula (codes 800..=899): inner series, then [`post_scan3`].
/// Inner series: the lane (`Identity`), RSI(`period`), or a smoother of the lane (`SmoothLane`).
pub fn launch_cube_signal(
    formula: CubeFormula,
    samples: &[GpuSample],
    params: CubeParams,
) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    let ident = || launch_cube(CubeFormula::Identity, samples, params);
    let rsi = || launch_cube(CubeFormula::Rsi, samples, params);
    // RSI thresholds are clamped as the CPU gates clamp them: upper 50..=100, lower 0..=50.
    let (up, lo) = (params.a.clamp(50.0, 100.0), params.b.clamp(0.0, 50.0));
    match formula {
        CubeFormula::DirDetect => run_post3(&ident(), 20, 1, 0.0, 0.0, 0, 0),
        CubeFormula::RegimeGateSig => run_post3(&ident(), 21, 1, params.a, 0.0, 0, params.flag),
        CubeFormula::ThresholdEdge => {
            run_post3(&ident(), 22, 1, params.a, params.b, 0, params.flag)
        }
        CubeFormula::ThresholdGateSig => run_post3(&rsi(), 23, 1, up, lo, params.period, 0),
        CubeFormula::HysteresisGateSig => run_post3(&rsi(), 24, 1, up, lo, params.period, 0),
        CubeFormula::VolEventSig => run_post3(&ident(), 25, params.period.max(1), params.a, 0.0, 0, 0),
        CubeFormula::SlopeDirLine => {
            let s = launch_cube(CubeFormula::SmoothLane, samples, params);
            run_post3(&s, 26, 1, 0.0, 0.0, 0, 0)
        }
        _ => Vec::new(),
    }
}

/// Two-series signal stage (`a_ser`, `b_ser`), outputs -1 / 0 / 1 as f32.
/// Ops: 30 AND gate / 31 OR gate / 32 XOR gate on two RSIs (extreme = outside 30..=70;
/// AND needs both on the same side), 1 or 0, sticky, updated once `i >= rf`;
/// 33 sign combiner (`(sig_short + sig_long)` clamped to -1..=1 with `<30 -> 1`, `>70 -> -1`);
/// 34 volatility regime detector on `a_ser` (`a` low, `b` high; +1 into High or out of Low,
/// -1 out of High or into Low, 0 otherwise and on bar 0);
/// 35 relative position: sign of `a_ser - b_ser`, sticky on non-zero changes, from `i >= rf`;
/// 36 CUSUM filter on `a_ser` closes (`a` threshold, abs and at least 1e-12).
#[cube]
fn post_scan4(
    a_ser: &[f32],
    b_ser: &[f32],
    out: &mut [f32],
    op: u32,
    a: f32,
    b: f32,
    rf: u32,
) {
    let n = a_ser.len();
    let mut sticky = 0.0f32;
    let mut prev_level = 9u32;
    let mut pos_sum = 0.0f32;
    let mut neg_sum = 0.0f32;
    for i in 0..n {
        let sv = a_ser[i];
        let lv = b_ser[i];
        let mut res = 0.0f32;
        if op >= 30u32 && op <= 33u32 {
            if (i as u32) >= rf {
                let s_ext = sv < 30.0f32 || sv > 70.0f32;
                let l_ext = lv < 30.0f32 || lv > 70.0f32;
                if op == 30u32 {
                    let both_hi = sv > 70.0f32 && lv > 70.0f32;
                    let both_lo = sv < 30.0f32 && lv < 30.0f32;
                    sticky = 0.0f32;
                    if both_hi || both_lo {
                        sticky = 1.0f32;
                    }
                } else if op == 31u32 {
                    sticky = 0.0f32;
                    if s_ext || l_ext {
                        sticky = 1.0f32;
                    }
                } else if op == 32u32 {
                    sticky = 0.0f32;
                    if s_ext != l_ext {
                        sticky = 1.0f32;
                    }
                } else {
                    let mut ss = 0.0f32;
                    if sv < 30.0f32 {
                        ss = 1.0f32;
                    } else if sv > 70.0f32 {
                        ss = -1.0f32;
                    }
                    let mut ls = 0.0f32;
                    if lv < 30.0f32 {
                        ls = 1.0f32;
                    } else if lv > 70.0f32 {
                        ls = -1.0f32;
                    }
                    let mut t = ss + ls;
                    if t > 1.0f32 {
                        t = 1.0f32;
                    } else if t < -1.0f32 {
                        t = -1.0f32;
                    }
                    sticky = t;
                }
            }
            res = sticky;
        } else if op == 34u32 {
            // levels: 0 low, 1 normal, 2 high
            let mut level = 1u32;
            if sv < a {
                level = 0u32;
            } else if sv > b {
                level = 2u32;
            }
            if prev_level != 9u32 && prev_level != level {
                if level == 2u32 {
                    res = 1.0f32;
                } else if prev_level == 2u32 {
                    res = -1.0f32;
                } else if level == 0u32 {
                    res = -1.0f32;
                } else if prev_level == 0u32 {
                    res = 1.0f32;
                }
            }
            prev_level = level;
        } else if op == 35u32 {
            if (i as u32) >= rf {
                let mut t = 0.0f32;
                if sv > lv {
                    t = 1.0f32;
                } else if sv < lv {
                    t = -1.0f32;
                }
                if t != 0.0f32 && t != sticky {
                    sticky = t;
                }
            }
            res = sticky;
        } else if op == 36u32 {
            if i > 0 {
                let mut th = a;
                if th < 0.0f32 {
                    th = -th;
                }
                if th < 1.0e-12f32 {
                    th = 1.0e-12f32;
                }
                let r = sv / a_ser[i - 1] - 1.0f32;
                pos_sum = pos_sum + r;
                if pos_sum < 0.0f32 {
                    pos_sum = 0.0f32;
                }
                neg_sum = neg_sum + r;
                if neg_sum > 0.0f32 {
                    neg_sum = 0.0f32;
                }
                if pos_sum > th {
                    res = 1.0f32;
                    pos_sum = 0.0f32;
                    neg_sum = 0.0f32;
                }
                if neg_sum < -th {
                    res = -1.0f32;
                    pos_sum = 0.0f32;
                    neg_sum = 0.0f32;
                }
            }
        }
        out[i] = res;
    }
}

#[cube(launch_unchecked)]
fn post_map4(a_ser: &[f32], b_ser: &[f32], out: &mut [f32], op: u32, a: f32, b: f32, rf: u32) {
    post_scan4(a_ser, b_ser, out, op, a, b, rf);
}

pub(crate) fn run_post4(a_ser: &[f32], b_ser: &[f32], op: u32, a: f32, b: f32, rf: u32) -> Vec<f32> {
    let n = a_ser.len();
    if n == 0 {
        return Vec::new();
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let sa = client.create_from_slice(f32::as_bytes(a_ser));
    let sb = client.create_from_slice(f32::as_bytes(b_ser));
    let out = client.empty(n * core::mem::size_of::<f32>());
    unsafe {
        post_map4::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(sa, n),
            BufferArg::from_raw_parts(sb, n),
            BufferArg::from_raw_parts(out.clone(), n),
            op,
            a,
            b,
            rf,
        );
    }
    let bytes = client.read_one_unchecked(out);
    f32::from_bytes(&bytes).to_vec()
}

/// Launch a two-series signal formula (codes 810..=829).
pub fn launch_cube_signal2(
    formula: CubeFormula,
    samples: &[GpuSample],
    params: CubeParams,
) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    let rsi_pair = || {
        let mut ps = params;
        ps.period = params.fast.max(1);
        let mut pl = params;
        pl.period = params.slow.max(1);
        (
            launch_cube(CubeFormula::Rsi, samples, ps),
            launch_cube(CubeFormula::Rsi, samples, pl),
            ps.period.max(pl.period),
        )
    };
    match formula {
        CubeFormula::LogicAnd | CubeFormula::LogicOr | CubeFormula::LogicXor | CubeFormula::LogicSign => {
            let op = match formula {
                CubeFormula::LogicAnd => 30,
                CubeFormula::LogicOr => 31,
                CubeFormula::LogicXor => 32,
                _ => 33,
            };
            let (s, l, rf) = rsi_pair();
            run_post4(&s, &l, op, 0.0, 0.0, rf)
        }
        CubeFormula::VolRegimeSig => {
            let s = launch_cube(CubeFormula::Identity, samples, params);
            run_post4(&s, &s, 34, params.a, params.b, 0)
        }
        CubeFormula::RelPositionSig => {
            // Subject: first smoother / `smooth_period`; reference: second / `smooth_period2`.
            // Every smoother is ready once `period` bars are in (SMA/EMA/RMA/WMA/DEMA/TEMA/HMA/
            // ALMA count to `period`; TMA/TRIMA's two SMAs both reach `period` on the same bar).
            let subj = launch_cube(CubeFormula::SmoothLane, samples, params);
            let mut pr = params;
            pr.smoother = params.smoother2;
            pr.smooth_period = params.smooth_period2;
            let refr = launch_cube(CubeFormula::SmoothLane, samples, pr);
            let rf = params.smooth_period.max(1).max(params.smooth_period2.max(1)) - 1;
            run_post4(&subj, &refr, 35, 0.0, 0.0, rf)
        }
        CubeFormula::CusumFilter => {
            let s = launch_cube(CubeFormula::Identity, samples, params);
            run_post4(&s, &s, 36, params.a, 0.0, 0)
        }
        _ => Vec::new(),
    }
}
