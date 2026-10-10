//! Stateful bar-series formulas, codes 1200..=1399: one sequential scan over the five OHLCV
//! lanes (`open`, `high`, `low`, `close`, `volume`), up to five output columns
//! (`out[col * n + i]`). Used by indicators whose CPU state does not fit the smoother-chain
//! composites (ring buffers read in storage order, held values, state machines).
//! Parameters: `period` / `p2` integers, `a` / `b` / `c` floats, `flag`.
//! UNTESTED on GPU (no GPU on the authoring box).

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::gpu::CubeParams;
use super::gpu_sample::GpuSample;
use super::kernels_comp::lane_series;
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
    _v: &[f32],
    out: &mut [f32],
    formula: u32,
    period: u32,
    p2: u32,
    a: f32,
    _b: f32,
    _cc: f32,
    flag: u32,
) {
    let n = c.len();
    let mut k = 0.0f32;
    let mut d = 0.0f32;
    for t in 0..n {
        let mut v0 = 0.0f32;
        let mut v1 = 0.0f32;
        let mut v2 = 0.0f32;
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
        }
        out[t] = v0;
        out[n + t] = v1;
        out[2 * n + t] = v2;
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
    a: f32,
    b: f32,
    cc: f32,
    flag: u32,
) {
    bar_scan(o, h, l, c, v, out, formula, period, p2, a, b, cc, flag);
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
    let v = lane_series(samples, params, OhlcvField::Volume);
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
            params.a,
            params.b,
            params.c,
            params.flag,
        );
    }
    let bytes = client.read_one_unchecked(out);
    let flat = f32::from_bytes(&bytes).to_vec();
    let cols = formula.output_count() as usize;
    (0..cols).map(|k| flat[k * n..(k + 1) * n].to_vec()).collect()
}
