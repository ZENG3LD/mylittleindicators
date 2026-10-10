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
    let mut held2 = 1.0f32;
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
        } else if op == 31u32 {
            // window std about y[t] with the Bessel factor sqrt(n / (n - 1)) for n > 1
            let p = a as usize;
            let mut lo = 0usize;
            if t + 1 > p {
                lo = t + 1 - p;
            }
            let nn = (t + 1 - lo) as f32;
            let mut ss = 0.0f32;
            for j in lo..(t + 1) {
                let d = x[j] - y[t];
                ss = ss + d * d;
            }
            v = (ss / nn).sqrt();
            if nn > 1.0f32 {
                v = v * (nn / (nn - 1.0f32)).sqrt();
            }
        } else if op == 32u32 {
            if t > 0 && x[t] > x[t - 1] {
                v = y[t];
            }
        } else if op == 33u32 {
            if t > 0 && x[t] < x[t - 1] {
                v = y[t];
            }
        } else if op == 34u32 {
            // CCI: x = typical price, y = mean, window a, scalar b; 0 until full window
            let p = a as usize;
            if t + 1 >= p {
                let mut sum = 0.0f32;
                for j in (t + 1 - p)..(t + 1) {
                    let mut d = x[j] - y[t];
                    if d < 0.0f32 {
                        d = -d;
                    }
                    sum = sum + d;
                }
                let mad = sum / (p as f32);
                let mut am = mad;
                if am < 0.0f32 {
                    am = -am;
                }
                if am >= 1.0e-12f32 {
                    v = (x[t] - y[t]) / (b * mad);
                }
            }
        } else if op == 35u32 {
            v = x[t] - y[t];
            if v < 0.0f32 {
                v = 0.0f32;
            }
        } else if op == 36u32 {
            // Chaikin volatility: x = smoothed range, lag a, first bar b
            let lag = a as usize;
            if (t as f32) >= b {
                let past = x[t - lag];
                let mut ap = past;
                if ap < 0.0f32 {
                    ap = -ap;
                }
                if ap >= 1.0e-12f32 {
                    v = 100.0f32 * (x[t] - past) / past;
                }
            }
        } else if op == 37u32 {
            v = 1.0f32;
            let mut ay = y[t];
            if ay < 0.0f32 {
                ay = -ay;
            }
            if ay > 1.0e-12f32 {
                v = x[t] / y[t];
            }
        } else if op == 38u32 {
            // window sum of x over a bars, 0 until full
            let p = a as usize;
            if t + 1 >= p {
                let mut sum = 0.0f32;
                for j in (t + 1 - p)..(t + 1) {
                    sum = sum + x[j];
                }
                v = sum;
            }
        } else if op == 39u32 {
            let m = a as usize;
            if t >= m {
                v = x[t] - x[t - m];
                if v < 0.0f32 {
                    v = 0.0f32;
                }
            }
        } else if op == 40u32 {
            let m = a as usize;
            if t >= m {
                v = x[t - m] - x[t];
                if v < 0.0f32 {
                    v = 0.0f32;
                }
            }
        } else if op == 41u32 {
            // RMI: x = up average, y = down average, 0 before bar a
            let m = a as usize;
            if t >= m {
                let dn = x[t] + y[t];
                v = 50.0f32;
                if dn > 0.0f32 {
                    v = 100.0f32 * x[t] / dn;
                }
            }
        } else if op == 42u32 {
            // Didi ratios held while |mid| <= 1e-12: x = short, y = mid, z = long; a = 0 short, 1 long
            let mut am = y[t];
            if am < 0.0f32 {
                am = -am;
            }
            if am > 1.0e-12f32 {
                if a < 0.5f32 {
                    held = x[t] / y[t];
                } else {
                    held = z[t] / y[t];
                }
            }
            v = held;
        } else if op == 43u32 {
            // SSL: x = close, y = ma(high), z = ma(low); a = 0 up line, 1 down line
            if t == 0 {
                if a < 0.5f32 {
                    v = y[t];
                } else {
                    v = z[t];
                }
            } else if a < 0.5f32 {
                v = z[t];
                if x[t] > y[t] {
                    v = y[t];
                }
            } else {
                v = y[t];
                if x[t] < z[t] {
                    v = z[t];
                }
            }
        } else if op == 44u32 {
            v = 0.0f32;
            let mut ad = y[t];
            if ad < 0.0f32 {
                ad = -ad;
            }
            if ad >= 1.0e-12f32 {
                v = x[t] / y[t];
            }
        } else if op == 45u32 {
            v = x[t] - y[t];
            if v < 1.0e-12f32 {
                v = 1.0e-12f32;
            }
        } else if op == 46u32 {
            // EMA slope: x = smoothed, lookback a, first ready bar b
            let lb = a as usize;
            if (t as f32) >= b {
                v = (x[t] - x[t + 1 - lb]) / a;
            }
        } else if op == 47u32 {
            // trend intensity: x = close, y = ma, window a; 50 until full
            let p = a as usize;
            v = 50.0f32;
            if t + 1 >= p {
                let mut cnt = 0.0f32;
                for j in (t + 1 - p)..(t + 1) {
                    if x[j] > y[t] {
                        cnt = cnt + 1.0f32;
                    }
                }
                v = 100.0f32 * cnt / a;
            }
        } else if op == 48u32 {
            v = x[t] * x[t];
        } else if op == 49u32 {
            let mut sv = y[t];
            if sv < 0.0f32 {
                sv = 0.0f32;
            }
            let sd = sv.sqrt();
            if sd > 0.0f32 {
                v = x[t] / sd;
            }
        } else if op == 50u32 {
            // volume price trend: x = close, y = volume
            if t > 0 {
                let mut pc = x[t - 1];
                if pc < 0.0f32 {
                    pc = -pc;
                }
                if pc > 1.0e-12f32 {
                    held = held + y[t] * (x[t] - x[t - 1]) / x[t - 1];
                }
            }
            v = held;
        } else if op == 51u32 {
            // std-dev channel deviation: x = price, y = regression line, window a, b = 1 for the
            // population (n - 1) denominator; 0 until the window is full
            let p = a as usize;
            if t + 1 >= p {
                let mut ss = 0.0f32;
                for j in (t + 1 - p)..(t + 1) {
                    let d = x[j] - y[t];
                    ss = ss + d * d;
                }
                let mut den = a;
                if b > 0.5f32 {
                    den = a - 1.0f32;
                }
                v = (ss / den).sqrt();
            }
        } else if op == 52u32 {
            // R2-weighted deviation: x = residual std, y = r2; times (0.5 + (r2 > 0 ? 1 - r2 : 1))
            let mut wgt = 1.0f32;
            if y[t] > 0.0f32 {
                wgt = 1.0f32 - y[t];
            }
            v = x[t] * (0.5f32 + wgt);
        } else if op == 53u32 {
            v = x[t];
            if v < 0.0f32 {
                v = -v;
            }
        } else if op == 54u32 {
            // Kalman slope z-score over a rolling window a (partial until full)
            let w = a as usize;
            let mut lo = 0usize;
            if t + 1 > w {
                lo = t + 1 - w;
            }
            let nn = (t + 1 - lo) as f32;
            let mut sum = 0.0f32;
            let mut sq = 0.0f32;
            for j in lo..(t + 1) {
                sum = sum + x[j];
                sq = sq + x[j] * x[j];
            }
            let mean = sum / nn;
            let mut var = sq / nn - mean * mean;
            if var < 1.0e-12f32 {
                var = 1.0e-12f32;
            }
            v = (x[t] - mean) / var.sqrt();
        } else if op == 55u32 {
            // Kalman slope z-score, window a, 0 until full, sd floor 1e-9
            let w = a as usize;
            if t + 1 >= w {
                let st = t + 1 - w;
                let mut sum = 0.0f32;
                for j in st..(t + 1) {
                    sum = sum + x[j];
                }
                let mean = sum / a;
                let mut ss = 0.0f32;
                for j in st..(t + 1) {
                    let d = x[j] - mean;
                    ss = ss + d * d;
                }
                let mut sd = (ss / a).sqrt();
                if sd < 1.0e-9f32 {
                    sd = 1.0e-9f32;
                }
                v = (x[t] - mean) / sd;
            }
        } else if op == 56u32 {
            // 0.5 * (tanh(x) + 1) = 1 / (1 + exp(-2x))
            v = 1.0f32 / (1.0f32 + (-2.0f32 * x[t]).exp());
        } else if op == 57u32 {
            // population std of the last a values about their own mean (partial until full)
            let p = a as usize;
            let mut lo = 0usize;
            if t + 1 > p {
                lo = t + 1 - p;
            }
            let nn = (t + 1 - lo) as f32;
            let mut sum = 0.0f32;
            for j in lo..(t + 1) {
                sum = sum + x[j];
            }
            let mean = sum / nn;
            let mut ss = 0.0f32;
            for j in lo..(t + 1) {
                let d = x[j] - mean;
                ss = ss + d * d;
            }
            v = (ss / nn).sqrt();
        } else if op == 58u32 {
            // Keltner position: x = close, y = upper, z = lower, w = middle
            let kw = (y[t] - z[t]) / 2.0f32;
            if kw > 0.0f32 {
                v = (x[t] - w[t]) / kw;
            }
        } else if op == 59u32 {
            // ATR trailing stop long level: x = new long level, y = close
            if t == 0 {
                held = x[0];
            } else if x[t] > held || y[t] < held {
                held = x[t];
            }
            v = held;
        } else if op == 60u32 {
            let p = a as usize;
            let mut lo = 0usize;
            if t + 1 > p {
                lo = t + 1 - p;
            }
            v = x[lo];
            for j in lo..(t + 1) {
                if x[j] > v {
                    v = x[j];
                }
            }
        } else if op == 61u32 {
            let p = a as usize;
            let mut lo = 0usize;
            if t + 1 > p {
                lo = t + 1 - p;
            }
            v = x[lo];
            for j in lo..(t + 1) {
                if x[j] < v {
                    v = x[j];
                }
            }
        } else if op == 62u32 {
            // Gann HiLo activator state machine: x = close, y = ma(high), z = ma(low), b = ready
            // bar count; `a` = 0 activator, 1 side. Side starts long; activator starts 0.
            if (t as f32) + 1.0f32 >= b {
                if held2 > 0.0f32 {
                    if x[t] < z[t] {
                        held2 = -1.0f32;
                        held = y[t];
                    } else {
                        held = z[t];
                    }
                } else if x[t] > y[t] {
                    held2 = 1.0f32;
                    held = z[t];
                } else {
                    held = y[t];
                }
            }
            if a < 0.5f32 {
                v = held;
            } else {
                v = held2;
            }
        } else if op == 64u32 {
            // Pressure: x = 2c - l - h, y = ATR, z = volume, w = avg volume, window a
            if (t as f32) + 1.0f32 >= a {
                let mut aa = y[t];
                if aa < 0.0f32 {
                    aa = -aa;
                }
                let mut av = w[t];
                if av < 0.0f32 {
                    av = -av;
                }
                if av >= 1.0e-12f32 && aa >= 1.0e-12f32 {
                    v = x[t] / y[t] * (z[t] / w[t]);
                }
            }
        } else if op == 65u32 {
            // population z-score of x[t] over the last a values (partial window, n >= 2),
            // variance as E[x^2] - mean^2, 0 when std <= 1e-12
            let p = a as usize;
            let mut lo = 0usize;
            if t + 1 > p {
                lo = t + 1 - p;
            }
            let nn = (t + 1 - lo) as f32;
            if nn >= 2.0f32 {
                let mut sum = 0.0f32;
                let mut sq = 0.0f32;
                for j in lo..(t + 1) {
                    sum = sum + x[j];
                    sq = sq + x[j] * x[j];
                }
                let mean = sum / nn;
                let var = sq / nn - mean * mean;
                if var > 0.0f32 {
                    let sd = var.sqrt();
                    if sd > 1.0e-12f32 {
                        v = (x[t] - mean) / sd;
                    }
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

/// Rolling OLS over the last `period` values with x = 1..=period (`LinearRegressionMA`):
/// `line` (fitted value at the last x, plus the slope when `zero_lag != 0`), `slope`,
/// `icpt`, `r2` and `rstd` (population std of the residuals, used by the regression
/// channels). All 0 until the window is full.
#[cube]
fn lr_scan(
    x: &[f32],
    line: &mut [f32],
    slope: &mut [f32],
    icpt: &mut [f32],
    r2: &mut [f32],
    rstd: &mut [f32],
    period: u32,
    zero_lag: u32,
) {
    let n = x.len();
    let p = period as usize;
    let pf = p as f32;
    let xs = 0.5f32 * pf * (pf + 1.0f32);
    let xms = xs * (2.0f32 * pf + 1.0f32) / 3.0f32;
    let divisor = pf * xms - xs * xs;
    for t in 0..n {
        let mut ln = 0.0f32;
        let mut sl = 0.0f32;
        let mut ic = 0.0f32;
        let mut rr = 0.0f32;
        let mut rs = 0.0f32;
        if p >= 1 && t + 1 >= p {
            let st = t + 1 - p;
            let mut ysum = 0.0f32;
            let mut sxy = 0.0f32;
            for j in 0..p {
                ysum = ysum + x[st + j];
                sxy = sxy + ((j + 1) as f32) * x[st + j];
            }
            sl = (pf * sxy - xs * ysum) / divisor;
            ic = (ysum * xms - xs * sxy) / divisor;
            ln = sl * pf + ic;
            if zero_lag != 0u32 {
                ln = ln + sl;
            }
            let mean = ysum / pf;
            let mut sres = 0.0f32;
            let mut sres2 = 0.0f32;
            let mut stot = 0.0f32;
            for j in 0..p {
                let res = sl * ((j + 1) as f32) + ic - x[st + j];
                sres = sres + res;
                sres2 = sres2 + res * res;
                let dm = x[st + j] - mean;
                stot = stot + dm * dm;
            }
            rr = 1.0f32 - sres2 / stot;
            let rm = sres / pf;
            let mut vv = 0.0f32;
            for j in 0..p {
                let res = sl * ((j + 1) as f32) + ic - x[st + j];
                let d = (-res) - (-rm);
                vv = vv + d * d;
            }
            rs = (vv / pf).sqrt();
        }
        line[t] = ln;
        slope[t] = sl;
        icpt[t] = ic;
        r2[t] = rr;
        rstd[t] = rs;
    }
}

#[cube(launch_unchecked)]
fn lr_map(
    x: &[f32],
    line: &mut [f32],
    slope: &mut [f32],
    icpt: &mut [f32],
    r2: &mut [f32],
    rstd: &mut [f32],
    period: u32,
    zero_lag: u32,
) {
    lr_scan(x, line, slope, icpt, r2, rstd, period, zero_lag);
}

/// `[line, slope, intercept, r2, residual std]` of the rolling regression.
pub(crate) fn lr_all(x: &[f32], period: u32, zero_lag: bool) -> Vec<Vec<f32>> {
    let n = x.len();
    if n == 0 {
        return vec![Vec::new(); 5];
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let xb = client.create_from_slice(f32::as_bytes(x));
    let bufs: Vec<_> = (0..5).map(|_| client.empty(n * core::mem::size_of::<f32>())).collect();
    unsafe {
        lr_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(xb, n),
            BufferArg::from_raw_parts(bufs[0].clone(), n),
            BufferArg::from_raw_parts(bufs[1].clone(), n),
            BufferArg::from_raw_parts(bufs[2].clone(), n),
            BufferArg::from_raw_parts(bufs[3].clone(), n),
            BufferArg::from_raw_parts(bufs[4].clone(), n),
            period,
            zero_lag as u32,
        );
    }
    bufs.into_iter()
        .map(|b| f32::from_bytes(&client.read_one_unchecked(b)).to_vec())
        .collect()
}

/// `BasicKalmanFilter` (2-state position / velocity filter, H = [[1,0],[0,0]], R = diag(r, 1e12),
/// P0 = diag(1000, 100)), scalar form of the CPU matrix algebra. `pos` is the corrected
/// position (`filtered_value`), `velp` the velocity right after the predict step (the CPU
/// `FilterResult::velocity`, which the slope indicators read; 0 on the first bar).
/// `adaptive != 0` runs the 10-sample innovation window noise adaptation.
#[cube]
fn kal_scan(x: &[f32], pos: &mut [f32], velp: &mut [f32], dt: f32, q: f32, r0: f32, adaptive: u32) {
    let n = x.len();
    let mut win = Array::<f32>::new(10usize);
    let mut wlen = 0usize;
    let mut wpos = 0usize;
    let mut r = r0;
    let mut px = 0.0f32;
    let mut pv = 0.0f32;
    let mut p00 = 1000.0f32;
    let mut p01 = 0.0f32;
    let mut p10 = 0.0f32;
    let mut p11 = 100.0f32;
    let q00 = q * dt * dt * dt / 3.0f32;
    let q01 = q * dt * dt / 2.0f32;
    let q11 = q * dt;
    for t in 0..n {
        if t == 0 {
            px = x[0];
            pv = 0.0f32;
            p00 = 1000.0f32;
            p01 = 0.0f32;
            p10 = 0.0f32;
            p11 = 100.0f32;
            pos[0] = x[0];
            velp[0] = 0.0f32;
        } else {
            // predict
            px = px + dt * pv;
            let a00 = p00 + dt * p10;
            let a01 = p01 + dt * p11;
            let n00 = a00 + dt * a01 + q00;
            let n01 = a01 + q01;
            let n10 = p10 + dt * p11 + q01;
            let n11 = p11 + q11;
            p00 = n00;
            p01 = n01;
            p10 = n10;
            p11 = n11;
            velp[t] = pv;
            // correct
            let innov = x[t] - px;
            let s00 = p00 + r;
            let det = s00 * 1.0e12f32;
            let mut adet = det;
            if adet < 0.0f32 {
                adet = -adet;
            }
            if adet >= 1.0e-12f32 {
                let i00 = 1.0e12f32 / det;
                let k0 = p00 * i00;
                let k1 = p10 * i00;
                px = px + k0 * innov;
                pv = pv + k1 * innov;
                let c00 = (1.0f32 - k0) * p00;
                let c01 = (1.0f32 - k0) * p01;
                let c10 = p10 - k1 * p00;
                let c11 = p11 - k1 * p01;
                p00 = c00;
                p01 = c01;
                p10 = c10;
                p11 = c11;
                if adaptive != 0u32 {
                    win[wpos] = innov;
                    wpos = (wpos + 1) % 10;
                    if wlen < 10 {
                        wlen = wlen + 1;
                    }
                    if wlen >= 5 {
                        let mut sum = 0.0f32;
                        for j in 0..wlen {
                            sum = sum + win[j];
                        }
                        let mean = sum / (wlen as f32);
                        let mut ss = 0.0f32;
                        for j in 0..wlen {
                            let d = win[j] - mean;
                            ss = ss + d * d;
                        }
                        let var = ss / ((wlen - 1) as f32);
                        if var > r * 2.0f32 {
                            let mut nr = r * 1.1f32;
                            if var < nr {
                                nr = var;
                            }
                            r = nr;
                        } else if var < r * 0.5f32 {
                            let mut nr = r * 0.9f32;
                            if nr < 1.0e-12f32 {
                                nr = 1.0e-12f32;
                            }
                            r = nr;
                        }
                    }
                }
            }
            pos[t] = px;
        }
    }
}

#[cube(launch_unchecked)]
fn kal_map(x: &[f32], pos: &mut [f32], velp: &mut [f32], dt: f32, q: f32, r0: f32, adaptive: u32) {
    kal_scan(x, pos, velp, dt, q, r0, adaptive);
}

/// `[filtered position, velocity after predict]`; `dt`, process and measurement noise are
/// clamped like `BasicKalmanFilter::new`.
pub(crate) fn kalman_all(x: &[f32], dt: f32, q: f32, r: f32, adaptive: bool) -> Vec<Vec<f32>> {
    let n = x.len();
    if n == 0 {
        return vec![Vec::new(), Vec::new()];
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let xb = client.create_from_slice(f32::as_bytes(x));
    let pb = client.empty(n * core::mem::size_of::<f32>());
    let vb = client.empty(n * core::mem::size_of::<f32>());
    unsafe {
        kal_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(xb, n),
            BufferArg::from_raw_parts(pb.clone(), n),
            BufferArg::from_raw_parts(vb.clone(), n),
            dt.max(1.0e-6),
            q.max(1.0e-12),
            r.max(1.0e-12),
            adaptive as u32,
        );
    }
    vec![
        f32::from_bytes(&client.read_one_unchecked(pb)).to_vec(),
        f32::from_bytes(&client.read_one_unchecked(vb)).to_vec(),
    ]
}

/// Alpha-beta-gamma filter `[pos, vel, acc]` with coefficients `alpha`, `beta`, `gamma`.
#[cube]
fn abg_scan(x: &[f32], pos: &mut [f32], vel: &mut [f32], acc: &mut [f32], al: f32, be: f32, ga: f32) {
    let n = x.len();
    let mut p = 0.0f32;
    let mut v = 0.0f32;
    let mut a = 0.0f32;
    for t in 0..n {
        if t == 0 {
            p = x[0];
            v = 0.0f32;
            a = 0.0f32;
        } else {
            let pp = p + v + 0.5f32 * a;
            let vp = v + a;
            let res = x[t] - pp;
            p = pp + al * res;
            v = vp + be * res;
            a = a + ga * res;
        }
        pos[t] = p;
        vel[t] = v;
        acc[t] = a;
    }
}

#[cube(launch_unchecked)]
fn abg_map(x: &[f32], pos: &mut [f32], vel: &mut [f32], acc: &mut [f32], al: f32, be: f32, ga: f32) {
    abg_scan(x, pos, vel, acc, al, be, ga);
}

pub(crate) fn abg_all(x: &[f32], al: f32, be: f32, ga: f32) -> Vec<Vec<f32>> {
    let n = x.len();
    if n == 0 {
        return vec![Vec::new(); 3];
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let xb = client.create_from_slice(f32::as_bytes(x));
    let bufs: Vec<_> = (0..3).map(|_| client.empty(n * core::mem::size_of::<f32>())).collect();
    unsafe {
        abg_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(xb, n),
            BufferArg::from_raw_parts(bufs[0].clone(), n),
            BufferArg::from_raw_parts(bufs[1].clone(), n),
            BufferArg::from_raw_parts(bufs[2].clone(), n),
            al,
            be,
            ga,
        );
    }
    bufs.into_iter()
        .map(|b| f32::from_bytes(&client.read_one_unchecked(b)).to_vec())
        .collect()
}

/// Direct-form IIR as `ButterworthFilter::update` runs it: buffers pre-filled with the first
/// value, output `sum_i b[i] x[t-i] - sum_{i>=1} a[i] y[t-i]` over `i <= order`, first output
/// is the first value. `a` and `b` hold `order + 1` coefficients.
#[cube]
fn iir_scan(x: &[f32], a: &[f32], b: &[f32], out: &mut [f32], order: u32) {
    let n = x.len();
    let o = order as usize;
    for t in 0..n {
        if t == 0 {
            out[0] = x[0];
        } else {
            let mut acc = 0.0f32;
            for i in 0..(o + 1) {
                let mut xv = x[0];
                if t >= i {
                    xv = x[t - i];
                }
                acc = acc + b[i] * xv;
            }
            for i in 1..(o + 1) {
                let mut yv = x[0];
                if t >= i {
                    yv = out[t - i];
                }
                acc = acc - a[i] * yv;
            }
            out[t] = acc;
        }
    }
}

#[cube(launch_unchecked)]
fn iir_map(x: &[f32], a: &[f32], b: &[f32], out: &mut [f32], order: u32) {
    iir_scan(x, a, b, out, order);
}

/// FIR over the last `w` values; the raw value until the window is full
/// (`SavitzkyGolayFilter`). `c[i]` multiplies the i-th oldest value of the window.
#[cube]
fn fir_scan(x: &[f32], c: &[f32], out: &mut [f32], w: u32) {
    let n = x.len();
    let ww = w as usize;
    for t in 0..n {
        if t + 1 < ww {
            out[t] = x[t];
        } else {
            let mut acc = 0.0f32;
            for i in 0..ww {
                acc = acc + c[i] * x[t + 1 - ww + i];
            }
            out[t] = acc;
        }
    }
}

#[cube(launch_unchecked)]
fn fir_map(x: &[f32], c: &[f32], out: &mut [f32], w: u32) {
    fir_scan(x, c, out, w);
}

/// Roofing filter: 2-pole high-pass then super smoother, 0 for the first two bars.
#[cube]
fn roof_scan(x: &[f32], out: &mut [f32], h0: f32, h1: f32, h2: f32, s1: f32, s2: f32, s3: f32) {
    let n = x.len();
    let mut p0 = 0.0f32;
    let mut p1 = 0.0f32;
    let mut hp0 = 0.0f32;
    let mut hp1 = 0.0f32;
    let mut ss0 = 0.0f32;
    let mut ss1 = 0.0f32;
    let mut val = 0.0f32;
    for t in 0..n {
        let c = x[t];
        if t < 2 {
            if t == 0 {
                p1 = c;
            } else {
                p0 = p1;
                p1 = c;
            }
        } else {
            let hp = h0 * (c - 2.0f32 * p1 + p0) + h1 * hp1 - h2 * hp0;
            let ss = s1 * (hp + hp1) / 2.0f32 + s2 * ss1 + s3 * ss0;
            val = ss;
            p0 = p1;
            p1 = c;
            hp0 = hp1;
            hp1 = hp;
            ss0 = ss1;
            ss1 = ss;
        }
        out[t] = val;
    }
}

#[cube(launch_unchecked)]
fn roof_map(x: &[f32], out: &mut [f32], h0: f32, h1: f32, h2: f32, s1: f32, s2: f32, s3: f32) {
    roof_scan(x, out, h0, h1, h2, s1, s2, s3);
}

fn run_buf(x: &[f32], extra: &[&[f32]], which: u32, u: &[u32], f: &[f32]) -> Vec<f32> {
    let n = x.len();
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let xb = client.create_from_slice(f32::as_bytes(x));
    let ob = client.empty(n * 4);
    let eb: Vec<_> = extra.iter().map(|e| client.create_from_slice(f32::as_bytes(e))).collect();
    unsafe {
        match which {
            0 => iir_map::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(xb, n),
                BufferArg::from_raw_parts(eb[0].clone(), extra[0].len()),
                BufferArg::from_raw_parts(eb[1].clone(), extra[1].len()),
                BufferArg::from_raw_parts(ob.clone(), n),
                u[0],
            ),
            1 => fir_map::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(xb, n),
                BufferArg::from_raw_parts(eb[0].clone(), extra[0].len()),
                BufferArg::from_raw_parts(ob.clone(), n),
                u[0],
            ),
            _ => roof_map::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(xb, n),
                BufferArg::from_raw_parts(ob.clone(), n),
                f[0],
                f[1],
                f[2],
                f[3],
                f[4],
                f[5],
            ),
        }
    }
    f32::from_bytes(&client.read_one_unchecked(ob)).to_vec()
}

pub(crate) fn ew2(op: u32, x: &[f32], y: &[f32], a: f32) -> Vec<f32> {
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

pub(crate) fn lane_series(samples: &[GpuSample], params: CubeParams, lane: OhlcvField) -> Vec<f32> {
    let mut p = params;
    p.lane = lane;
    launch_cube(CubeFormula::Identity, samples, p)
}

pub(crate) fn sm(s: &[f32], which: CubeSmoother, period: u32, skip: u32, p: CubeParams) -> Vec<f32> {
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
        // CCI over `period` (`smoother` mean, `a` = scalar) on the typical price.
        CubeFormula::CciComp => {
            let tp = lane_series(samples, p, OhlcvField::HLC3);
            let mean = sm(&tp, p.smoother, p.period, 0, p);
            vec![ewc(34, &tp, &mean, &tp, p.period as f32, p.a)]
        }
        // Chaikin volatility: range smoothed by `smoother` over `period`, ROC over `slow` bars
        // (k, at least 1) of the smoothed range from the bar the smoother is ready.
        CubeFormula::CvComp => {
            let h = lane_series(samples, p, OhlcvField::High);
            let l = lane_series(samples, p, OhlcvField::Low);
            let rng = ew2(35, &h, &l, 0.0);
            let n = p.period.max(1);
            let k = p.slow.max(1);
            let e = sm(&rng, p.smoother, n, 0, p);
            vec![ewc(36, &e, &e, &e, k as f32, (n - 1 + k) as f32)]
        }
        // Mass index: two chained smoothers of high - low over `period`, sum of the ratio over
        // `slow` bars.
        CubeFormula::MiComp => {
            let h = lane_series(samples, p, OhlcvField::High);
            let l = lane_series(samples, p, OhlcvField::Low);
            let hl = ew2(2, &h, &l, 0.0);
            let e1 = sm(&hl, p.smoother, p.period, 0, p);
            let e2 = sm(&e1, p.smoother, p.period, 0, p);
            let ratio = ew2(37, &e1, &e2, 0.0);
            vec![ewc(38, &ratio, &ratio, &ratio, p.slow as f32, 0.0)]
        }
        // Relative momentum index: momentum lookback `period`, up / down averages with
        // `smoother` over `smooth_period`, fed from bar `period`.
        CubeFormula::RmiComp => {
            let src = lane_series(samples, p, p.lane);
            let m = p.period.max(1);
            let up = ewc(39, &src, &src, &src, m as f32, 0.0);
            let dn = ewc(40, &src, &src, &src, m as f32, 0.0);
            let ua = sm(&up, p.smoother, p.smooth_period, m, p);
            let da = sm(&dn, p.smoother, p.smooth_period, m, p);
            vec![ewc(41, &ua, &da, &ua, m as f32, 0.0)]
        }
        // Relative volatility index over `period`, one `smoother` for all three slots.
        CubeFormula::RviComp => {
            let src = lane_series(samples, p, p.lane);
            let pp = p.period.max(1);
            let mean = sm(&src, p.smoother, pp, 0, p);
            let sd = ewc(31, &src, &mean, &src, pp as f32, 0.0);
            let pi = ewc(32, &src, &sd, &src, 0.0, 0.0);
            let ni = ewc(33, &src, &sd, &src, 0.0, 0.0);
            let pos = sm(&pi, p.smoother, pp, 1, p);
            let neg = sm(&ni, p.smoother, pp, 1, p);
            let den = ew2(11, &pos, &neg, 0.0);
            let raw = ew2(6, &pos, &den, 0.0);
            vec![ewc(15, &raw, &raw, &raw, pp as f32, 0.0)]
        }
        // Detrended synthetic price: `x - smoother(x)` over `period` (at least 2).
        CubeFormula::DspComp => {
            let src = lane_series(samples, p, p.lane);
            let m = sm(&src, p.smoother, p.period.max(2), 0, p);
            vec![ew2(2, &src, &m, 0.0)]
        }
        // Didi index `[short, long]` ratios to the mid average (`a` unused): periods
        // `fast` (short), `slow` (mid, at least 2), `signal` (long, at least 3), one smoother.
        CubeFormula::DidiCols => {
            let src = lane_series(samples, p, p.lane);
            let sh = sm(&src, p.smoother, p.fast, 0, p);
            let mi = sm(&src, p.smoother, p.slow.max(2), 0, p);
            let lg = sm(&src, p.smoother, p.signal.max(3), 0, p);
            vec![ewc(42, &sh, &mi, &lg, 0.0, 0.0), ewc(42, &sh, &mi, &lg, 1.0, 0.0)]
        }
        // SSL channel `[up, down]`: `smoother` over highs and lows, `period`.
        CubeFormula::SslCols => {
            let h = lane_series(samples, p, OhlcvField::High);
            let l = lane_series(samples, p, OhlcvField::Low);
            let c = lane_series(samples, p, OhlcvField::Close);
            let mh = sm(&h, p.smoother, p.period, 0, p);
            let ml = sm(&l, p.smoother, p.period, 0, p);
            vec![ewc(43, &c, &mh, &ml, 0.0, 0.0), ewc(43, &c, &mh, &ml, 1.0, 0.0)]
        }
        // Relative vigor index `[rvgi, signal]`: SMA(close - open) / SMA(max(high - low, 1e-12))
        // over `period`, SMA signal over `signal` fed once both are full.
        CubeFormula::RvgiCols => {
            let o = lane_series(samples, p, OhlcvField::Open);
            let h = lane_series(samples, p, OhlcvField::High);
            let l = lane_series(samples, p, OhlcvField::Low);
            let c = lane_series(samples, p, OhlcvField::Close);
            let pp = p.period.max(1);
            let num = ew2(2, &c, &o, 0.0);
            let den = ew2(45, &h, &l, 0.0);
            let n = sm(&num, CubeSmoother::Sma, pp, 0, p);
            let d = sm(&den, CubeSmoother::Sma, pp, 0, p);
            let r = ew2(44, &n, &d, 0.0);
            let sig = sm(&r, CubeSmoother::Sma, p.signal.max(1), pp - 1, p);
            vec![r, sig]
        }
        // Slope of a smoother over `slow` bars (lookback), smoother `period`.
        CubeFormula::EmaSlopeComp => {
            let src = lane_series(samples, p, p.lane);
            let lb = p.slow.max(1);
            let e = sm(&src, p.smoother, p.period, 0, p);
            let first = lb.max(p.period.max(1)) - 1;
            vec![ewc(46, &e, &e, &e, lb as f32, first as f32)]
        }
        // Trend intensity index over `period` (clamped 2..=512), SMA centre.
        CubeFormula::TiiComp => {
            let src = lane_series(samples, p, p.lane);
            let w = p.period.clamp(2, 512);
            let m = sm(&src, CubeSmoother::Sma, w, 0, p);
            vec![ewc(47, &src, &m, &src, w as f32, 0.0)]
        }
        // Price z-score: mean and variance smoothers (one id) over `period` (at least 2).
        CubeFormula::PriceZComp => {
            let src = lane_series(samples, p, p.lane);
            let n = p.period.max(2);
            let mean = sm(&src, p.smoother, n, 0, p);
            let diff = ew2(2, &src, &mean, 0.0);
            let sq = ew2(48, &diff, &diff, 0.0);
            let var = sm(&sq, p.smoother, n, 0, p);
            vec![ew2(49, &diff, &var, 0.0)]
        }
        // Volume price trend.
        CubeFormula::VptComp => {
            let c = lane_series(samples, p, OhlcvField::Close);
            let v = lane_series(samples, p, OhlcvField::Volume);
            vec![ew2(50, &c, &v, 0.0)]
        }
        // Rolling linear regression `[line, gradient, intercept, r2]` over `period`;
        // `flag != 0` is the zero-lag variant.
        CubeFormula::LrCols => {
            let src = lane_series(samples, p, p.lane);
            let mut r = super::kernels_comp::lr_all(&src, p.period, p.flag != 0);
            r.truncate(4);
            r
        }
        // Regression channels `[upper, middle, lower]`, `a` = multiplier, `flag`: 0 standard
        // (residual std), 1 percentage, 2 R2-weighted. Bands are 0 until a full window.
        CubeFormula::RegChanCols | CubeFormula::RegChanWidthComp => {
            let src = lane_series(samples, p, p.lane);
            let wide = formula == CubeFormula::RegChanWidthComp;
            let (per, mult) = if wide { (p.period.max(2), p.a.max(0.1)) } else { (p.period, p.a) };
            let r = lr_all(&src, per, false);
            let mid = r[0].clone();
            let g = per.max(1) as f32;
            let band: Vec<f32> = match p.flag {
                1 => mid.iter().map(|m| *m * (mult / 100.0)).collect(),
                2 => ewc(52, &r[4], &r[3], &r[4], 0.0, 0.0).iter().map(|w| *w * mult).collect(),
                _ => r[4].iter().map(|w| *w * mult).collect(),
            };
            let up = ewc(15, &ewc(13, &mid, &band, &mid, 1.0, 0.0), &mid, &mid, g, 0.0);
            let lo = ewc(15, &ewc(14, &mid, &band, &mid, 1.0, 0.0), &mid, &mid, g, 0.0);
            if formula == CubeFormula::RegChanWidthComp {
                return vec![ewc(53, &ew2(2, &up, &lo, 0.0), &up, &up, 0.0, 0.0)];
            }
            vec![up, mid, lo]
        }
        // Standard-deviation channels `[upper, middle, lower]` (the 2-sigma bands about the
        // regression line): `a` = multiplier, `flag` 1 = population (n - 1) denominator
        // (simple and adaptive both divide by n: the adaptive factor stays 1).
        CubeFormula::StdDevChanCols | CubeFormula::StdDevWidthComp => {
            let src = lane_series(samples, p, p.lane);
            let wide = formula == CubeFormula::StdDevWidthComp;
            let (per, mult) = if wide { (p.period.max(2), p.a.max(0.1)) } else { (p.period, p.a) };
            let r = lr_all(&src, per, false);
            let mid = r[0].clone();
            let sd = ewc(51, &src, &mid, &src, per as f32, if p.flag == 1 { 1.0 } else { 0.0 });
            let band: Vec<f32> = sd.iter().map(|w| *w * mult).collect();
            let up = ewc(13, &mid, &band, &mid, 1.0, 0.0);
            let lo = ewc(14, &mid, &band, &mid, 1.0, 0.0);
            if formula == CubeFormula::StdDevWidthComp {
                return vec![ewc(53, &ew2(2, &up, &lo, 0.0), &up, &up, 0.0, 0.0)];
            }
            vec![up, mid, lo]
        }
        // Basic Kalman filter filtered position. `a` = dt, `b` = process noise,
        // `c` = measurement noise, `flag != 0` adaptive noise.
        CubeFormula::KalmanComp | CubeFormula::RtsComp => {
            let src = lane_series(samples, p, p.lane);
            let (dt, q, r, ad) = if formula == CubeFormula::RtsComp { (1.0, 1.0, 1.0, false) } else { (p.a, p.b, p.c, p.flag != 0) };
            vec![kalman_all(&src, dt, q, r, ad).swap_remove(0)]
        }
        // Kalman trend slope `[slope, slope_z]`; window = `period` (at least 10).
        CubeFormula::KslopeCols => {
            let src = lane_series(samples, p, p.lane);
            let k = kalman_all(&src, p.a, p.b, p.c, p.flag != 0);
            let z = ewc(54, &k[1], &k[1], &k[1], p.period.max(10) as f32, 0.0);
            vec![k[1].clone(), z]
        }
        // Kalman regime score: `0.5 * (tanh(slope_z) + 1)`, window `period` (at least 10).
        CubeFormula::KscrComp => {
            let src = lane_series(samples, p, p.lane);
            let k = kalman_all(&src, p.a, p.b, p.c, p.flag != 0);
            let z = ewc(54, &k[1], &k[1], &k[1], p.period.max(10) as f32, 0.0);
            vec![ewc(56, &z, &z, &z, 0.0, 0.0)]
        }
        // Kalman slope z-score, window `period` (at least 20), 0 until the window is full.
        CubeFormula::KslopezComp => {
            let src = lane_series(samples, p, p.lane);
            let k = kalman_all(&src, p.a, p.b, p.c, p.flag != 0);
            vec![ewc(55, &k[1], &k[1], &k[1], p.period.max(20) as f32, 0.0)]
        }
        // Alpha-beta-gamma filter `[pos, vel, acc]`: `flag == 0` derives the coefficients from
        // `period` (alpha = 2 / (n + 1), beta = alpha^2 / 2, gamma = beta * alpha / 2),
        // otherwise alpha, beta, gamma = `a`, `b`, `c`.
        CubeFormula::AbgCols => {
            let src = lane_series(samples, p, p.lane);
            let n = p.period.max(1) as f32;
            let (al, be, ga) = if p.flag == 0 {
                let al = 2.0 / (n + 1.0);
                let be = al * al / 2.0;
                (al, be, be * al / 2.0)
            } else {
                (p.a, p.b, p.c)
            };
            abg_all(&src, al, be, ga)
        }
        // Butterworth: `ext[0]` filter type (0 low, 1 high, 2 band-pass, 3 band-stop),
        // `ext[1]` order, `a` cutoff, `b` sampling rate. Coefficients come from the CPU design.
        CubeFormula::ButterComp => {
            use crate::indicators::signal_processing::butterworth::{ButterworthFilter, FilterType};
            let src = lane_series(samples, p, p.lane);
            let ft = match p.ext[0] {
                1 => FilterType::HighPass,
                2 => FilterType::BandPass,
                3 => FilterType::BandStop,
                _ => FilterType::LowPass,
            };
            let order = (p.ext[1] as usize).clamp(1, 8);
            let f = ButterworthFilter::new(ft, order, p.a as f64, p.b as f64);
            let (ac, bc) = f.coefficients();
            let mut a: Vec<f32> = ac.iter().map(|v| *v as f32).collect();
            let mut b: Vec<f32> = bc.iter().map(|v| *v as f32).collect();
            a.resize(order + 1, 0.0);
            b.resize(order + 1, 0.0);
            vec![run_buf(&src, &[&a, &b], 0, &[order as u32], &[])]
        }
        // Savitzky-Golay: `period` window (made odd, 5..=63), `slow` polynomial order, `flag`
        // derivative order (0 smoothing .. 3). Window coefficients are read off the CPU filter by
        // feeding it unit impulses (its output is linear in the window).
        CubeFormula::SgComp => {
            use crate::indicators::signal_processing::savitzky_golay::{DerivativeOrder, SavitzkyGolayFilter};
            let src = lane_series(samples, p, p.lane);
            let d = match p.flag {
                1 => DerivativeOrder::FirstDerivative,
                2 => DerivativeOrder::SecondDerivative,
                3 => DerivativeOrder::ThirdDerivative,
                _ => DerivativeOrder::Smoothing,
            };
            let mut w = p.period as usize;
            if w % 2 == 0 {
                w += 1;
            }
            let w = w.clamp(5, 63);
            let mut c = Vec::with_capacity(w);
            for j in 0..w {
                let mut f = SavitzkyGolayFilter::new(w, p.slow as usize, d);
                let mut y = 0.0;
                for i in 0..w {
                    y = f.feed(if i == j { 1.0 } else { 0.0 });
                }
                c.push(y as f32);
            }
            vec![run_buf(&src, &[&c], 1, &[w as u32], &[])]
        }
        // Roofing filter: `flag == 0` periods `a` (high-pass) and `b` (smoother), otherwise
        // alphas (period = 2 / clamp(alpha, 0.001, 1)).
        CubeFormula::RoofComp => {
            let src = lane_series(samples, p, p.lane);
            let (hp, lp) = if p.flag == 0 {
                (p.a as f64, p.b as f64)
            } else {
                (2.0 / (p.a as f64).clamp(0.001, 1.0), 2.0 / (p.b as f64).clamp(0.001, 1.0))
            };
            let hp = hp.max(2.0);
            let lp = lp.max(2.0);
            let pi = std::f64::consts::PI;
            let sq2 = 2.0f64.sqrt();
            let c1 = (sq2 * pi / hp).cos();
            let s1 = (sq2 * pi / hp).sin();
            let al = (c1 + s1 - 1.0) / c1;
            let h0 = (1.0 - al / 2.0).powi(2);
            let h1 = 2.0 * (1.0 - al);
            let h2 = (1.0 - al).powi(2);
            let bb = (-sq2 * pi / lp).exp();
            let sc2 = 2.0 * bb * (sq2 * pi / lp).cos();
            let sc3 = -bb * bb;
            let sc1 = 1.0 - sc2 - sc3;
            vec![run_buf(
                &src,
                &[],
                2,
                &[],
                &[h0 as f32, h1 as f32, h2 as f32, sc1 as f32, sc2 as f32, sc3 as f32],
            )]
        }
        // TRIMA bands `[upper, middle, lower]`: TRIMA(`period`, at least 2) centre, population
        // std over `clamp(period, 2, 512)` bars times `a` (non-positive -> 2); bands stay 0
        // until the std window is full.
        CubeFormula::TrimaBandsCols => {
            let src = lane_series(samples, p, p.lane);
            let mid = sm(&src, CubeSmoother::Tma, p.period.max(2), 0, p);
            let w = p.period.clamp(2, 512);
            let k = if p.a > 0.0 { p.a } else { 2.0 };
            let sd = ewc(57, &src, &src, &src, w as f32, 0.0);
            let up = ewc(15, &ewc(13, &mid, &sd, &mid, k, 0.0), &mid, &mid, w as f32, 0.0);
            let lo = ewc(15, &ewc(14, &mid, &sd, &mid, k, 0.0), &mid, &mid, w as f32, 0.0);
            vec![up, mid, lo]
        }
        // Keltner position of the SMA(typical) Keltner (`VoKcCols` inputs): `(close - middle) /
        // ((upper - lower) / 2)`, 0 when the half width is not positive.
        CubeFormula::KpComp => {
            let c = lane_series(samples, p, OhlcvField::Close);
            let k = launch_cube_comp(CubeFormula::VoKcCols, samples, p);
            vec![ewb(58, &c, &k[0], &k[2], &k[1], 0.0, 0.0)]
        }
        // Chandelier stop long level: `max(high, period) - mult * ATR(Rma, period)`, `a` = mult.
        CubeFormula::ChandComp => {
            let h = lane_series(samples, p, OhlcvField::High);
            let hh = ewc(60, &h, &h, &h, p.period as f32, 0.0);
            let atr = atr_series(samples, p, CubeSmoother::Rma, p.period, 0);
            vec![ewc(14, &hh, &atr, &hh, p.a, 0.0)]
        }
        // Chande-Kroll stop long level: highest high over `slow` bars minus `a` (non-positive ->
        // 1.5) times ATR(Rma, `period`).
        CubeFormula::CksComp => {
            let h = lane_series(samples, p, OhlcvField::High);
            let hh = ewc(60, &h, &h, &h, p.slow.max(1) as f32, 0.0);
            let atr = atr_series(samples, p, CubeSmoother::Rma, p.period.max(1), 0);
            let k = if p.a > 0.0 { p.a } else { 1.5 };
            vec![ewc(14, &hh, &atr, &hh, k, 0.0)]
        }
        // ATR trailing stop long level: ratcheting `max(high, period) - mult * ATR`
        // (smoother `smoother2`, default Wilder), `a` = mult.
        CubeFormula::AtrtsComp => {
            let h = lane_series(samples, p, OhlcvField::High);
            let c = lane_series(samples, p, OhlcvField::Close);
            let hh = ewc(60, &h, &h, &h, p.period as f32, 0.0);
            let atr = atr_series(samples, p, p.smoother2, p.period, 0);
            let nl = ewc(14, &hh, &atr, &hh, p.a, 0.0);
            vec![ew2(59, &nl, &c, 0.0)]
        }
        // Gann HiLo activator `[activator, side]`: `smoother` of highs / lows over `period`.
        CubeFormula::GannHiloCols => {
            let h = lane_series(samples, p, OhlcvField::High);
            let l = lane_series(samples, p, OhlcvField::Low);
            let c = lane_series(samples, p, OhlcvField::Close);
            let pp = p.period.max(1);
            let mh = sm(&h, p.smoother, pp, 0, p);
            let ml = sm(&l, p.smoother, pp, 0, p);
            vec![
                ewb(62, &c, &mh, &ml, &c, 0.0, pp as f32),
                ewb(62, &c, &mh, &ml, &c, 1.0, pp as f32),
            ]
        }
        // Buy/sell pressure: `period`, volume smoother `smoother`, ATR smoother `smoother2`.
        CubeFormula::PressureComp => {
            let h = lane_series(samples, p, OhlcvField::High);
            let l = lane_series(samples, p, OhlcvField::Low);
            let c = lane_series(samples, p, OhlcvField::Close);
            let v = lane_series(samples, p, OhlcvField::Volume);
            let pp = p.period.max(1);
            let atr = atr_series(samples, p, p.smoother2, pp, 0);
            let av = sm(&v, p.smoother, pp, 0, p);
            let c2 = ewc(13, &c, &c, &c, 1.0, 0.0);
            let n = ew2(2, &ew2(2, &c2, &l, 0.0), &h, 0.0);
            vec![ewb(64, &n, &atr, &v, &av, pp as f32, 0.0)]
        }
        // MACD histogram z-score: histogram of (fast, slow, signal) smoothers (`smoother`,
        // `smoother2`, `smoother3`) z-scored over `period` bars.
        CubeFormula::MacdHistZComp => {
            let src = lane_series(samples, p, p.lane);
            let fast = sm(&src, p.smoother, p.fast, 0, p);
            let slow = sm(&src, p.smoother2, p.slow, 0, p);
            let line = ew2(2, &fast, &slow, 0.0);
            let rdy = p.fast.max(1).max(p.slow.max(1)) - 1;
            let sig = sm(&line, p.smoother3, p.signal, rdy, p);
            let hist = ew2(2, &line, &sig, 0.0);
            vec![ewc(65, &hist, &hist, &hist, p.period.max(2) as f32, 0.0)]
        }
        _ => Vec::new(),
    }
}
