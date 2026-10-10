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
    let (series, op, w, a) = plan(formula, samples, params);
    run_post(&series, op, w, a)
}
