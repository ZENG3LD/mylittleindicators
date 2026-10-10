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
    b: f32,
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
        } else if op == 13u32 {
            v = x[t] + a * y[t];
        } else if op == 14u32 {
            v = x[t] - a * y[t];
        } else if op == 15u32 {
            v = b;
            if (t as f32) + 1.0f32 >= a {
                v = x[t];
            }
        } else if op == 16u32 {
            // population std of the window (length min(a, t + 1)) about y[t]
            let p = a as usize;
            let mut lo = 0usize;
            if t + 1 > p {
                lo = t + 1 - p;
            }
            let mut ss = 0.0f32;
            for j in lo..(t + 1) {
                let d = x[j] - y[t];
                ss = ss + d * d;
            }
            v = (ss / ((t + 1 - lo) as f32)).sqrt();
        } else if op == 23u32 {
            // percent position: x = close, y = lower, z = upper; 0.5 when width <= 0
            let wd = z[t] - y[t];
            v = 0.5f32;
            if wd > 0.0f32 {
                v = (x[t] - y[t]) / wd;
            }
        } else if op == 24u32 {
            // bandwidth: x = upper, y = lower, z = middle
            let mut wd = x[t] - y[t];
            if wd < 0.0f32 {
                wd = 0.0f32;
            }
            let mut am = z[t];
            if am < 0.0f32 {
                am = -am;
            }
            if am > 1.0e-12f32 {
                v = wd / am;
            }
        } else if op == 25u32 {
            // stochastic raw %K over window a: x = close, y = high, z = low
            let p = a as usize;
            if t + 1 >= p {
                let mut hh = y[t + 1 - p];
                let mut ll = z[t + 1 - p];
                for j in (t + 1 - p)..(t + 1) {
                    if y[j] > hh {
                        hh = y[j];
                    }
                    if z[j] < ll {
                        ll = z[j];
                    }
                }
                let mut r = hh - ll;
                if r < 0.0f32 {
                    r = -r;
                }
                if r >= 1.0e-12f32 {
                    v = 100.0f32 * (x[t] - ll) / (hh - ll);
                }
            }
        } else if op == 30u32 {
            // true range, first fed bar `a`: x = high, y = low, z = close; 0 before it
            let st = a as usize;
            if t == st {
                v = x[t] - y[t];
            } else if t > st {
                let hl = x[t] - y[t];
                let mut d1 = x[t] - z[t - 1];
                if d1 < 0.0f32 {
                    d1 = -d1;
                }
                let mut d2 = y[t] - z[t - 1];
                if d2 < 0.0f32 {
                    d2 = -d2;
                }
                let mut hl2 = hl;
                if hl2 < 0.0f32 {
                    hl2 = -hl2;
                }
                v = hl;
                if d1 > v {
                    v = d1;
                }
                if d2 > v {
                    v = d2;
                }
            }
        } else if op == 29u32 {
            if z[t] != 0.0f32 {
                v = (x[t] - y[t]) / z[t];
            }
        } else if op == 26u32 {
            v = 3.0f32 * x[t] - 2.0f32 * y[t];
        } else if op == 27u32 {
            if (t as f32) + 1.0f32 >= b {
                v = x[t] * (1.0f32 + a / 100.0f32);
            }
        } else if op == 28u32 {
            if (t as f32) + 1.0f32 >= b {
                v = x[t] * (1.0f32 - a / 100.0f32);
            }
        }
        out[t] = v;
    }
}

#[cube(launch_unchecked)]
fn ew_map(x: &[f32], y: &[f32], z: &[f32], w: &[f32], out: &mut [f32], op: u32, a: f32, b: f32) {
    ew_scan(x, y, z, w, out, op, a, b);
}

/// One `ew_scan` launch. Unused inputs can repeat `x`.
pub(crate) fn ew(op: u32, x: &[f32], y: &[f32], z: &[f32], w: &[f32], a: f32) -> Vec<f32> {
    ewb(op, x, y, z, w, a, 0.0)
}

/// `ew` with the second scalar `b`.
pub(crate) fn ewb(
    op: u32,
    x: &[f32],
    y: &[f32],
    z: &[f32],
    w: &[f32],
    a: f32,
    b: f32,
) -> Vec<f32> {
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
            b,
        );
    }
    let bytes = client.read_one_unchecked(out);
    f32::from_bytes(&bytes).to_vec()
}

fn ew2(op: u32, x: &[f32], y: &[f32], a: f32) -> Vec<f32> {
    ew(op, x, y, x, x, a)
}

fn ewc(op: u32, x: &[f32], y: &[f32], z: &[f32], a: f32, b: f32) -> Vec<f32> {
    ewb(op, x, y, z, x, a, b)
}

/// ATR series: true range from bar `start` smoothed by `which` over `period` (first TR is
/// `high - low`, like `Atr::feed`).
fn atr_series(
    samples: &[GpuSample],
    p: CubeParams,
    which: CubeSmoother,
    period: u32,
    start: u32,
) -> Vec<f32> {
    let h = lane_series(samples, p, OhlcvField::High);
    let l = lane_series(samples, p, OhlcvField::Low);
    let c = lane_series(samples, p, OhlcvField::Close);
    let tr = ewc(30, &h, &l, &c, start as f32, 0.0);
    sm(&tr, which, period, start, p)
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
        // Bollinger bands: `[upper, middle, lower, std_dev, bandwidth, percent_b]`, centre line
        // `smoother` over `period`, `a` = std multiplier. Bands need the full window.
        CubeFormula::BbCols => {
            let src = lane_series(samples, p, p.lane);
            let mid = sm(&src, p.smoother, p.period, 0, p);
            let sd = ewc(16, &src, &mid, &src, p.period as f32, 0.0);
            let up = ewc(13, &mid, &sd, &mid, p.a, 0.0);
            let lo = ewc(14, &mid, &sd, &mid, p.a, 0.0);
            let g = p.period.max(1) as f32;
            let up = ewc(15, &up, &up, &up, g, 0.0);
            let lo = ewc(15, &lo, &lo, &lo, g, 0.0);
            let sd = ewc(15, &sd, &sd, &sd, g, 0.0);
            // CPU bandwidth divides by the signed middle when it is non-zero.
            let bw = ewc(29, &up, &lo, &mid, 0.0, 0.0);
            let pb = ewc(23, &src, &lo, &up, 0.0, 0.0);
            vec![up, mid, lo, sd, bw, pb]
        }
        // Bollinger on the typical price: `[middle, upper, lower]`, all 0 until a full window.
        CubeFormula::BbPeriodCols => {
            let tp = lane_series(samples, p, OhlcvField::HLC3);
            let mid = sm(&tp, p.smoother, p.period, 0, p);
            let sd = ewc(16, &tp, &mid, &tp, p.period as f32, 0.0);
            let up = ewc(13, &mid, &sd, &mid, p.a, 0.0);
            let lo = ewc(14, &mid, &sd, &mid, p.a, 0.0);
            let g = p.period.max(1) as f32;
            vec![
                ewc(15, &mid, &mid, &mid, g, 0.0),
                ewc(15, &up, &up, &up, g, 0.0),
                ewc(15, &lo, &lo, &lo, g, 0.0),
            ]
        }
        // `[percent_b, bandwidth]` of SMA Bollinger bands (`a` = multiplier).
        CubeFormula::BbMetricsCols => {
            let src = lane_series(samples, p, p.lane);
            let mid = sm(&src, CubeSmoother::Sma, p.period, 0, p);
            let sd = ewc(16, &src, &mid, &src, p.period as f32, 0.0);
            let g = p.period.max(1) as f32;
            let up = ewc(15, &ewc(13, &mid, &sd, &mid, p.a, 0.0), &mid, &mid, g, 0.0);
            let lo = ewc(15, &ewc(14, &mid, &sd, &mid, p.a, 0.0), &mid, &mid, g, 0.0);
            vec![ewc(23, &src, &lo, &up, 0.0, 0.0), ewc(24, &up, &lo, &mid, 0.0, 0.0)]
        }
        // Envelope `[upper, middle, lower]`; `a` = percent. Fixed, adaptive and multiple modes
        // all give the base percentage on the main channel (the adaptive factor stays 1).
        CubeFormula::EnvelopeCols => {
            let src = lane_series(samples, p, p.lane);
            let mid = sm(&src, p.smoother, p.period, 0, p);
            let g = p.period.max(1) as f32;
            vec![
                ewc(27, &mid, &mid, &mid, p.a, g),
                mid.clone(),
                ewc(28, &mid, &mid, &mid, p.a, g),
            ]
        }
        // Stochastics `[k, d]`: %K over `period`, %D = `smoother` over `smooth_period`, fed
        // from the first full %K window.
        CubeFormula::StochCols => {
            let c = lane_series(samples, p, OhlcvField::Close);
            let h = lane_series(samples, p, OhlcvField::High);
            let l = lane_series(samples, p, OhlcvField::Low);
            let k = ewc(25, &c, &h, &l, p.period as f32, 0.0);
            let d = sm(&k, p.smoother, p.smooth_period, p.period.max(1) - 1, p);
            vec![k, d]
        }
        // KDJ `[k, d, j]`: %K as Stochastics, %D smoother fed on every bar, J = 3D - 2K.
        CubeFormula::KdjCols => {
            let c = lane_series(samples, p, OhlcvField::Close);
            let h = lane_series(samples, p, OhlcvField::High);
            let l = lane_series(samples, p, OhlcvField::Low);
            let k = ewc(25, &c, &h, &l, p.period as f32, 0.0);
            let d = sm(&k, p.smoother, p.smooth_period, 0, p);
            let j = ew2(26, &d, &k, 0.0);
            vec![k, d, j]
        }
        // Keltner channel `[upper, middle, lower]`: centre `smoother` of lane `lane`, ATR with
        // `smoother2`, both over `period`, `a` = multiplier. Middle always shows the centre;
        // the bands are 0 until `period` bars (mode is unused by the CPU feed).
        CubeFormula::KcCols => {
            let src = lane_series(samples, p, p.lane);
            let mid = sm(&src, p.smoother, p.period, 0, p);
            let atr = atr_series(samples, p, p.smoother2, p.period, 0);
            let g = p.period.max(1) as f32;
            let up = ewc(15, &ewc(13, &mid, &atr, &mid, p.a, 0.0), &mid, &mid, g, 0.0);
            let lo = ewc(15, &ewc(14, &mid, &atr, &mid, p.a, 0.0), &mid, &mid, g, 0.0);
            vec![up, mid, lo]
        }
        // `[width, position]` of the Keltner channel.
        CubeFormula::KcMetricsCols => {
            let c = lane_series(samples, p, OhlcvField::Close);
            let src = lane_series(samples, p, p.lane);
            let mid = sm(&src, p.smoother, p.period, 0, p);
            let atr = atr_series(samples, p, p.smoother2, p.period, 0);
            let g = p.period.max(1) as f32;
            let up = ewc(15, &ewc(13, &mid, &atr, &mid, p.a, 0.0), &mid, &mid, g, 0.0);
            let lo = ewc(15, &ewc(14, &mid, &atr, &mid, p.a, 0.0), &mid, &mid, g, 0.0);
            vec![ew2(2, &up, &lo, 0.0), ewc(23, &c, &lo, &up, 0.0, 0.0)]
        }
        // ATR channels (volatility/atr_channels): MA of close over `period` with `smoother`,
        // ATR over `smooth_period` with `smoother2`, `a` = k. Never gated: `[upper, middle, lower]`.
        CubeFormula::AtrcCols | CubeFormula::StarcCols => {
            let src = lane_series(samples, p, if formula == CubeFormula::StarcCols { p.lane } else { OhlcvField::Close });
            let mid = sm(&src, p.smoother, p.period, 0, p);
            let atr = atr_series(samples, p, p.smoother2, p.smooth_period, 0);
            vec![ewc(13, &mid, &atr, &mid, p.a, 0.0), mid.clone(), ewc(14, &mid, &atr, &mid, p.a, 0.0)]
        }
        // ATR channels (channels/atr_channels): as `AtrcCols` but one `period`, bands 0 until
        // `period` bars.
        CubeFormula::AtrChanCols => {
            let src = lane_series(samples, p, OhlcvField::Close);
            let mid = sm(&src, p.smoother, p.period, 0, p);
            let atr = atr_series(samples, p, p.smoother2, p.period, 0);
            let g = p.period.max(1) as f32;
            let up = ewc(15, &ewc(13, &mid, &atr, &mid, p.a, 0.0), &mid, &mid, g, 0.0);
            let lo = ewc(15, &ewc(14, &mid, &atr, &mid, p.a, 0.0), &mid, &mid, g, 0.0);
            vec![up, mid, lo]
        }
        // Keltner (volatility/kc): SMA of the typical price, ATR (`smoother`) only from the
        // bar the SMA window is full; everything 0 before. `[upper, middle, lower]`.
        CubeFormula::VoKcCols => {
            let tp = lane_series(samples, p, OhlcvField::HLC3);
            let pp = p.period.max(1);
            let mid = sm(&tp, CubeSmoother::Sma, pp, 0, p);
            let atr = atr_series(samples, p, p.smoother, pp, pp - 1);
            let g = pp as f32;
            let up = ewc(15, &ewc(13, &mid, &atr, &mid, p.a, 0.0), &mid, &mid, g, 0.0);
            let lo = ewc(15, &ewc(14, &mid, &atr, &mid, p.a, 0.0), &mid, &mid, g, 0.0);
            let mi = ewc(15, &mid, &mid, &mid, g, 0.0);
            vec![up, mi, lo]
        }
        _ => Vec::new(),
    }
}
