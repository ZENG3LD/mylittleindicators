//! Smoother-chain composites, codes 1000..=1099.
//!
//! Each formula is a short program over series: device smoother passes
//! ([`super::kernels::smooth_series`], with the CPU feed's start bar as `skip`) joined by the
//! elementwise device kernel [`ew_scan`]. Every stage is its own launch. UNTESTED on GPU.
//!
//! Parameter map (`CubeParams`): `smoother` / `smoother2` / `smoother3` are the first / second /
//! third smoother slots of the CPU indicator, `smooth_period*` their periods, `fast` / `slow` /
//! `signal` the named periods, `ext` the extra integer array, `lane` the source.

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::gpu::{CubeParams, CubeSmoother};
use super::gpu_sample::GpuSample;
use super::kernels::{launch_cube, smooth_series};
use super::CubeFormula;
use crate::engine::ohlcv_field::OhlcvField;

/// Elementwise / lag ops over up to four series (`x`, `y`, `z`, `w`), parameter `a`.
/// 1 `100 * (x - y) / den(y)` with `den = 1e-12` when `|y| < 1e-12`;
/// 2 `x - y`; 3 ROC percent over lag `a` (`0` before the lag, `0` when `|past| < 1e-12`);
/// 4 `x[t] - x[t-1]` (0 at t = 0); 5 `|x[t] - x[t-1]|`;
/// 6 `100 * x / y` (0 when `|y| < 1e-12`);
/// 7 KST `(x + 2y + 3z + 4w) / 10`;
/// 8 TRIX `(x[t] - x[t-1]) / x[t-1] * 10000`, held when `|x[t-1]| <= 1e-12`, 0 at t = 0;
/// 9 KVO volume force: `x` = typical price, `y` = volume: `+y` up, `-y` down, 0 flat or first;
/// 10 detrended price: `x` = close, lag `a` = period: `close[t - off] - centred SMA`, 0 until
///    `t + 1 >= period + off` (`off = period / 2 + 1`);
/// 11 `x + y`;
/// 12 PMO/1-bar ROC percent: `(x[t] - x[t-1]) / x[t-1] * 100` with the 1e-12 guard, 0 at t = 0.
#[cube]
fn ew_scan(
    x: &[f32],
    y: &[f32],
    z: &[f32],
    w: &[f32],
    out: &mut [f32],
    op: u32,
    a: f32,
) {
    let n = x.len();
    let mut held = 0.0f32;
    for t in 0..n {
        let mut v = 0.0f32;
        if op == 1u32 {
            let mut den = y[t];
            let mut ad = den;
            if ad < 0.0f32 {
                ad = -ad;
            }
            if ad < 1.0e-12f32 {
                den = 1.0e-12f32;
            }
            v = 100.0f32 * (x[t] - y[t]) / den;
        } else if op == 2u32 {
            v = x[t] - y[t];
        } else if op == 3u32 {
            let lag = a as usize;
            if t >= lag {
                let past = x[t - lag];
                let mut ap = past;
                if ap < 0.0f32 {
                    ap = -ap;
                }
                if ap >= 1.0e-12f32 {
                    v = (x[t] - past) / past * 100.0f32;
                }
            }
        } else if op == 4u32 {
            if t > 0 {
                v = x[t] - x[t - 1];
            }
        } else if op == 5u32 {
            if t > 0 {
                v = x[t] - x[t - 1];
                if v < 0.0f32 {
                    v = -v;
                }
            }
        } else if op == 6u32 {
            let mut ay = y[t];
            if ay < 0.0f32 {
                ay = -ay;
            }
            if ay >= 1.0e-12f32 {
                v = 100.0f32 * x[t] / y[t];
            }
        } else if op == 7u32 {
            v = (x[t] + 2.0f32 * y[t] + 3.0f32 * z[t] + 4.0f32 * w[t]) / 10.0f32;
        } else if op == 8u32 {
            if t > 0 {
                let p = x[t - 1];
                let mut ap = p;
                if ap < 0.0f32 {
                    ap = -ap;
                }
                if ap > 1.0e-12f32 {
                    held = (x[t] - p) / p * 10000.0f32;
                }
            }
            v = held;
        } else if op == 9u32 {
            if t > 0 {
                if x[t] > x[t - 1] {
                    v = y[t];
                } else if x[t] < x[t - 1] {
                    v = -y[t];
                }
            }
        } else if op == 10u32 {
            let p = a as usize;
            let off = p / 2 + 1;
            if t + 1 >= p + off {
                let c = t - off;
                let half = p / 2;
                let mut hist = 0.0f32;
                if c >= half && c + half < t + 1 {
                    let start = c - half;
                    let mut sum = 0.0f32;
                    for j in start..(start + p) {
                        sum = sum + x[j];
                    }
                    hist = sum / (p as f32);
                }
                held = x[c] - hist;
            }
            v = held;
        } else if op == 11u32 {
            v = x[t] + y[t];
        } else if op == 12u32 {
            if t > 0 {
                let p = x[t - 1];
                let mut ap = p;
                if ap < 0.0f32 {
                    ap = -ap;
                }
                if ap > 1.0e-12f32 {
                    v = (x[t] - p) / p * 100.0f32;
                }
            }
        }
        out[t] = v;
    }
}

#[cube(launch_unchecked)]
fn ew_map(x: &[f32], y: &[f32], z: &[f32], w: &[f32], out: &mut [f32], op: u32, a: f32) {
    ew_scan(x, y, z, w, out, op, a);
}

/// One `ew_scan` launch. Unused inputs can repeat `x`.
pub(crate) fn ew(op: u32, x: &[f32], y: &[f32], z: &[f32], w: &[f32], a: f32) -> Vec<f32> {
    let n = x.len();
    if n == 0 {
        return Vec::new();
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let xb = client.create_from_slice(f32::as_bytes(x));
    let yb = client.create_from_slice(f32::as_bytes(y));
    let zb = client.create_from_slice(f32::as_bytes(z));
    let wb = client.create_from_slice(f32::as_bytes(w));
    let out = client.empty(n * core::mem::size_of::<f32>());
    unsafe {
        ew_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(xb, n),
            BufferArg::from_raw_parts(yb, n),
            BufferArg::from_raw_parts(zb, n),
            BufferArg::from_raw_parts(wb, n),
            BufferArg::from_raw_parts(out.clone(), n),
            op,
            a,
        );
    }
    let bytes = client.read_one_unchecked(out);
    f32::from_bytes(&bytes).to_vec()
}

fn ew2(op: u32, x: &[f32], y: &[f32], a: f32) -> Vec<f32> {
    ew(op, x, y, x, x, a)
}

fn lane_series(samples: &[GpuSample], params: CubeParams, lane: OhlcvField) -> Vec<f32> {
    let mut p = params;
    p.lane = lane;
    launch_cube(CubeFormula::Identity, samples, p)
}

fn sm(s: &[f32], which: CubeSmoother, period: u32, skip: u32, p: CubeParams) -> Vec<f32> {
    smooth_series(s, which, period.max(1), skip, p.a, p.b)
}

/// Launch a composite; one `Vec` per output column.
pub fn launch_cube_comp(
    formula: CubeFormula,
    samples: &[GpuSample],
    params: CubeParams,
) -> Vec<Vec<f32>> {
    if samples.is_empty() {
        return Vec::new();
    }
    let p = params;
    match formula {
        // Ppo / Pvo: line = 100 (fast - slow) / slow, signal = smoother3 of the line from the
        // bar both legs are ready (`max(fast, slow) - 1`), histogram = line - signal.
        CubeFormula::PpoCols | CubeFormula::PvoCols => {
            let lane = if formula == CubeFormula::PvoCols { OhlcvField::Volume } else { p.lane };
            let src = lane_series(samples, p, lane);
            let fast = sm(&src, p.smoother, p.fast, 0, p);
            let slow = sm(&src, p.smoother2, p.slow, 0, p);
            let line = ew2(1, &fast, &slow, 0.0);
            let rdy = p.fast.max(1).max(p.slow.max(1)) - 1;
            let sig = sm(&line, p.smoother3, p.signal, rdy, p);
            let hist = ew2(2, &line, &sig, 0.0);
            vec![line, sig, hist]
        }
        CubeFormula::TrixCols => {
            let src = lane_series(samples, p, p.lane);
            let e1 = sm(&src, p.smoother, p.period, 0, p);
            let e2 = sm(&e1, p.smoother, p.period, 0, p);
            let e3 = sm(&e2, p.smoother, p.period, 0, p);
            let line = ew2(8, &e3, &e3, 0.0);
            let sig = sm(&line, p.smoother, p.signal, 0, p);
            vec![line, sig]
        }
        // True strength index: momentum and |momentum| double smoothed (first = `fast`,
        // second = `slow`), signal = `signal`; all one smoother. Starts at bar 1.
        CubeFormula::TsiCols => {
            let src = lane_series(samples, p, p.lane);
            let pc = ew2(4, &src, &src, 0.0);
            let apc = ew2(5, &src, &src, 0.0);
            let f_pc = sm(&pc, p.smoother, p.fast, 1, p);
            let f_apc = sm(&apc, p.smoother, p.fast, 1, p);
            let s_pc = sm(&f_pc, p.smoother, p.slow, 1, p);
            let s_apc = sm(&f_apc, p.smoother, p.slow, 1, p);
            let line = ew2(6, &s_pc, &s_apc, 0.0);
            let sig = sm(&line, p.smoother, p.signal, 1, p);
            let hist = ew2(2, &line, &sig, 0.0);
            vec![line, sig, hist]
        }
        // Know sure thing: ROC periods `ext[0..4]`, smoother periods `ext[4..8]` (smoother
        // `smoother`), signal period `signal` with `smoother2`. Starts at bar `max(roc)`.
        CubeFormula::KstCols => {
            let src = lane_series(samples, p, p.lane);
            let max_roc = p.ext[0].max(p.ext[1]).max(p.ext[2]).max(p.ext[3]);
            let mut sm_roc: Vec<Vec<f32>> = Vec::new();
            for k in 0..4 {
                let roc = ew2(3, &src, &src, p.ext[k] as f32);
                sm_roc.push(sm(&roc, p.smoother, p.ext[4 + k], max_roc, p));
            }
            let line = ew(7, &sm_roc[0], &sm_roc[1], &sm_roc[2], &sm_roc[3], 0.0);
            let sig = sm(&line, p.smoother2, p.signal, max_roc, p);
            vec![line, sig]
        }
        // Price momentum oscillator: 1-bar ROC percent, two smoothers (`smooth_period`,
        // `smooth_period2`) and the signal (`smooth_period3`), all from bar 1, one smoother.
        CubeFormula::PmoCols => {
            let src = lane_series(samples, p, p.lane);
            let roc = ew2(12, &src, &src, 0.0);
            let s1 = sm(&roc, p.smoother, p.smooth_period, 1, p);
            let line = sm(&s1, p.smoother, p.smooth_period2, 1, p);
            let sig = sm(&line, p.smoother, p.smooth_period3, 1, p);
            vec![line, sig]
        }
        // Klinger volume oscillator: volume force, fast and slow smoothers (slow period at
        // least 2), signal from the bar the slow smoother is ready.
        CubeFormula::KvoCols => {
            let tp = lane_series(samples, p, OhlcvField::HLC3);
            let vol = lane_series(samples, p, OhlcvField::Volume);
            let force = ew2(9, &tp, &vol, 0.0);
            let slow_p = p.slow.max(2);
            let fast = sm(&force, p.smoother, p.fast, 0, p);
            let slow = sm(&force, p.smoother, slow_p, 0, p);
            let line = ew2(2, &fast, &slow, 0.0);
            let sig = sm(&line, p.smoother, p.signal, slow_p - 1, p);
            vec![line, sig]
        }
        // RSI(`period`) smoothed by `smoother` over `smooth_period` (fed from bar 0).
        CubeFormula::RsiOmaCols => {
            let rsi = launch_cube(CubeFormula::Rsi, samples, p);
            vec![sm(&rsi, p.smoother, p.smooth_period, 0, p)]
        }
        // Detrended price oscillator over `period`.
        CubeFormula::DpoCols => {
            let src = lane_series(samples, p, p.lane);
            vec![ew2(10, &src, &src, p.period as f32)]
        }
        // Elder ray: `[high - smoother(close), low - smoother(close)]`.
        CubeFormula::ElderRayCols => {
            let close = lane_series(samples, p, OhlcvField::Close);
            let hi = lane_series(samples, p, OhlcvField::High);
            let lo = lane_series(samples, p, OhlcvField::Low);
            let e = sm(&close, p.smoother, p.smooth_period, 0, p);
            vec![ew2(2, &hi, &e, 0.0), ew2(2, &lo, &e, 0.0)]
        }
        _ => Vec::new(),
    }
}
