//! Event-frame formulas, codes 900..=999: one sequential scan over a [`GpuEventFrame`].
//!
//! Column layout per stream: see [`super::event_frame`]. Output column `c` of event `i` is
//! `out[c * n + i]`. Parameters: `period` is the ring window, `a` / `b` / `c` the thresholds
//! (`CubeParams::a`, `b`, `c`). UNTESTED on GPU (no GPU on the authoring box).

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::event_frame::GpuEventFrame;
use super::gpu::CubeParams;
use super::CubeFormula;

#[cube]
fn ev_sqrt(x: f32) -> f32 {
    let mut y = 0.0f32;
    if x > 0.0f32 {
        y = x;
        for _step in 0..12 {
            y = 0.5f32 * (y + x / y);
        }
    }
    y
}

/// Mean and population std of `x[col * n + start ..= i]`.
#[cube]
fn ev_mean(x: &[f32], col: usize, n: usize, start: usize, i: usize) -> f32 {
    let mut sum = 0.0f32;
    for j in start..(i + 1) {
        sum = sum + x[col * n + j];
    }
    sum / ((i + 1 - start) as f32)
}

#[cube]
fn ev_std(x: &[f32], col: usize, n: usize, start: usize, i: usize, mean: f32) -> f32 {
    let mut acc = 0.0f32;
    for j in start..(i + 1) {
        let d = x[col * n + j] - mean;
        acc = acc + d * d;
    }
    ev_sqrt(acc / ((i + 1 - start) as f32))
}

/// Opcodes (formula code): 900 funding direction shift, 901 long/short extreme,
/// 902 predicted funding extreme, 903 leverage reduction warning, 904 high-low range ratio,
/// 905 ticker spread ratio, 906 IV skew, 907 risk limit proximity, 908 MMR tracker,
/// 909 theta decay (rolling sum), 910 24h price change z-score, 911 HV spike,
/// 912 insurance fund stress, 913 pin risk, 914 aggressor imbalance, 915 auction liquidity
/// score, 916 block trade size anomaly, 917 trade run `[side, run_length]`.
#[cube]
fn ev_scan(
    x: &[f32],
    side: &[f32],
    out: &mut [f32],
    formula: u32,
    window: u32,
    a: f32,
    b: f32,
    c: f32,
) {
    let n = side.len();
    let mut w = window as usize;
    if w < 1 {
        w = 1;
    }
    let mut run = 0.0f32;
    let mut cur_side = 0.0f32;
    for i in 0..n {
        let mut v0 = 0.0f32;
        let mut v1 = 0.0f32;
        if formula == 900u32 {
            if i > 0 {
                let p = x[i - 1];
                let q = x[i];
                if p > 0.0f32 && q < 0.0f32 {
                    v0 = -1.0f32;
                } else if p < 0.0f32 && q > 0.0f32 {
                    v0 = 1.0f32;
                }
            }
        } else if formula == 901u32 {
            let r = x[i];
            if r > a {
                v0 = 1.0f32;
            } else if r < b {
                v0 = -1.0f32;
            }
        } else if formula == 902u32 {
            let r = x[i];
            if r > a {
                v0 = 1.0f32;
            } else if r < -a {
                v0 = -1.0f32;
            }
        } else if formula == 903u32 {
            if i > 0 {
                let p = x[n + i - 1];
                let q = x[n + i];
                if q < p {
                    v0 = 1.0f32;
                } else if q > p {
                    v0 = -1.0f32;
                }
            }
        } else if formula == 904u32 {
            let last = x[i];
            if last > 0.0f32 {
                v0 = (x[3 * n + i] - x[4 * n + i]) / last;
            }
        } else if formula == 905u32 {
            let last = x[i];
            let mut al = last;
            if al < 0.0f32 {
                al = -al;
            }
            if al > 1.0e-15f32 {
                v0 = (x[2 * n + i] - x[n + i]) / last;
            }
        } else if formula == 906u32 {
            v0 = x[6 * n + i] - x[7 * n + i];
        } else if formula == 907u32 {
            v0 = (x[3 * n + i] + x[4 * n + i]) / 2.0f32;
        } else if formula == 908u32 {
            v0 = x[3 * n + i];
        } else if formula == 909u32 {
            let mut len = w;
            if len > i + 1 {
                len = i + 1;
            }
            let start = i + 1 - len;
            let mut sum = 0.0f32;
            for j in start..(i + 1) {
                sum = sum + x[3 * n + j];
            }
            v0 = sum;
        } else if formula == 910u32 {
            let mut pw = w;
            if pw < 2 {
                pw = 2;
            }
            let mut len = pw;
            if len > i + 1 {
                len = i + 1;
            }
            if len >= 2 {
                let start = i + 1 - len;
                let mean = ev_mean(x, 6, n, start, i);
                let sd = ev_std(x, 6, n, start, i, mean);
                if sd >= 1.0e-14f32 {
                    v0 = (x[6 * n + i] - mean) / sd;
                }
            }
        } else if formula == 911u32 {
            let mut pw = w;
            if pw < 2 {
                pw = 2;
            }
            let mut len = pw;
            if len > i + 1 {
                len = i + 1;
            }
            if len >= 2 {
                let start = i + 1 - len;
                let mean = ev_mean(x, 0, n, start, i);
                if mean > 1.0e-12f32 && x[i] > a * mean {
                    v0 = 1.0f32;
                }
            }
        } else if formula == 912u32 {
            let mut pw = w;
            if pw < 2 {
                pw = 2;
            }
            let mut len = pw;
            if len > i + 1 {
                len = i + 1;
            }
            if len >= 2 {
                let start = i + 1 - len;
                let slope = (x[i] - x[start]) / ((len - 1) as f32);
                let mut th = a;
                if th < 0.0f32 {
                    th = -th;
                }
                if slope < -th {
                    v0 = 1.0f32;
                }
            }
        } else if formula == 913u32 {
            let mut d = x[i];
            if d < 0.0f32 {
                d = -d;
            }
            let mut dd = d - a;
            if dd < 0.0f32 {
                dd = -dd;
            }
            let mut th = x[3 * n + i];
            if th < 0.0f32 {
                th = -th;
            }
            if dd <= b && th >= c {
                v0 = 1.0f32;
            }
        } else if formula == 914u32 {
            let mut len = w;
            if len > i + 1 {
                len = i + 1;
            }
            let start = i + 1 - len;
            let mut buy = 0.0f32;
            for j in start..(i + 1) {
                if side[j] > 0.0f32 {
                    buy = buy + 1.0f32;
                }
            }
            let total = len as f32;
            v0 = (buy - (total - buy)) / total;
        } else if formula == 915u32 {
            let mut len = w;
            if len > i + 1 {
                len = i + 1;
            }
            let start = i + 1 - len;
            let mean = ev_mean(x, 1, n, start, i);
            v0 = 1.0f32;
            if mean > 0.0f32 {
                v0 = x[n + i] / mean;
            }
        } else if formula == 916u32 {
            let mut pw = w;
            if pw < 2 {
                pw = 2;
            }
            let mut len = pw;
            if len > i + 1 {
                len = i + 1;
            }
            if len >= 2 {
                let start = i + 1 - len;
                let mean = ev_mean(x, 1, n, start, i);
                let sd = ev_std(x, 1, n, start, i, mean);
                if sd > 1.0e-9f32 {
                    v0 = (x[n + i] - mean) / sd;
                }
            }
        } else if formula == 917u32 {
            let s = if side[i] > 0.0f32 { 1.0f32 } else { -1.0f32 };
            if cur_side == s {
                run = run + 1.0f32;
            } else {
                cur_side = s;
                run = 1.0f32;
            }
            v0 = cur_side;
            v1 = run;
        }
        out[i] = v0;
        out[n + i] = v1;
    }
}

#[cube(launch_unchecked)]
fn ev_map(
    x: &[f32],
    side: &[f32],
    out: &mut [f32],
    formula: u32,
    window: u32,
    a: f32,
    b: f32,
    c: f32,
) {
    ev_scan(x, side, out, formula, window, a, b, c);
}


/// Number of events `j <= i` with `ts[i] - ts[j] <= w_ms` (the CPU deque after eviction).
#[cube]
pub(crate) fn win_start(ts: &[f32], i: usize, w_ms: f32) -> usize {
    let mut start = i;
    let mut go = true;
    while go {
        if start > 0 {
            if ts[i] - ts[start - 1] <= w_ms {
                start = start - 1;
            } else {
                go = false;
            }
        } else {
            go = false;
        }
    }
    start
}

/// Time-window event formulas. Window length in ms is `a`, a second window `b`, count `period`,
/// thresholds `b` / `c` as noted. 920 liquidation rate (per second), 921 liquidation cooldown
/// (seconds since the previous event), 922 liquidation volume velocity (quote value per minute),
/// 923 liquidation volume imbalance `[imbalance, long, short]`, 924 liquidation cascade
/// `[flag, count]` (`period` = threshold count), 925 agg-trade flow imbalance, 926 OI change
/// rate `[rate, current]`, 927 block-trade net flow, 928 block-trade rate (per minute),
/// 929 funding time decay / 930 settlement approach (`a` = max window ms, held while invalid),
/// 931 charm (delta change per second), 932 warning rate (per minute), 933 tick frequency
/// anomaly (short `a`, long `b`), 934 aggressor burst (`period` min count, `c` threshold),
/// 935 large tick momentum (`b` = size threshold), 936 size-weighted directional momentum.
#[cube]
fn evt_scan(
    x: &[f32],
    side: &[f32],
    ts: &[f32],
    out: &mut [f32],
    formula: u32,
    period: u32,
    a: f32,
    b: f32,
    c: f32,
) {
    let n = side.len();
    let mut held = 0.0f32;
    for i in 0..n {
        let mut v0 = 0.0f32;
        let mut v1 = 0.0f32;
        let mut v2 = 0.0f32;
        if formula == 920u32 {
            let s = win_start(ts, i, a);
            v0 = ((i + 1 - s) as f32) / (a / 1000.0f32);
        } else if formula == 921u32 {
            if i > 0 {
                v0 = (ts[i] - ts[i - 1]) / 1000.0f32;
            }
        } else if formula == 922u32 {
            let s = win_start(ts, i, a);
            let mut sum = 0.0f32;
            for j in s..(i + 1) {
                sum = sum + x[2 * n + j];
            }
            v0 = sum / (a / 60000.0f32);
        } else if formula == 923u32 {
            let s = win_start(ts, i, a);
            let mut lv = 0.0f32;
            let mut sv = 0.0f32;
            for j in s..(i + 1) {
                if side[j] > 0.0f32 {
                    lv = lv + x[2 * n + j];
                } else {
                    sv = sv + x[2 * n + j];
                }
            }
            let total = lv + sv;
            if total > 0.0f32 {
                v0 = (sv - lv) / total;
            }
            v1 = lv;
            v2 = sv;
        } else if formula == 924u32 {
            let s = win_start(ts, i, a);
            let cnt = (i + 1 - s) as u32;
            let mut th = period;
            if th < 1u32 {
                th = 1u32;
            }
            if cnt >= th {
                v0 = 1.0f32;
            }
            v1 = cnt as f32;
        } else if formula == 925u32 {
            let s = win_start(ts, i, a);
            let mut buy = 0.0f32;
            let mut sell = 0.0f32;
            for j in s..(i + 1) {
                if side[j] > 0.0f32 {
                    buy = buy + x[n + j];
                } else {
                    sell = sell + x[n + j];
                }
            }
            let total = buy + sell;
            if total > 0.0f32 {
                v0 = (buy - sell) / total;
            }
        } else if formula == 926u32 {
            if i > 0 {
                let mut dt = ts[i] - ts[i - 1];
                if dt < 1.0f32 {
                    dt = 1.0f32;
                }
                held = (x[i] - x[i - 1]) / (dt / 1000.0f32);
            }
            v0 = held;
            v1 = x[i];
        } else if formula == 927u32 {
            let s = win_start(ts, i, a);
            let mut net = 0.0f32;
            for j in s..(i + 1) {
                if side[j] > 0.0f32 {
                    net = net + x[n + j];
                } else {
                    net = net - x[n + j];
                }
            }
            v0 = net;
        } else if formula == 928u32 || formula == 932u32 {
            let s = win_start(ts, i, a);
            v0 = ((i + 1 - s) as f32) / (a / 60000.0f32);
        } else if formula == 929u32 || formula == 930u32 {
            if x[2 * n + i] > 0.5f32 {
                let mut norm = x[n + i] / a;
                if norm < 0.0f32 {
                    norm = 0.0f32;
                }
                if norm > 1.0f32 {
                    norm = 1.0f32;
                }
                held = 1.0f32 - norm;
            }
            v0 = held;
        } else if formula == 931u32 {
            if i > 0 {
                let dt = (ts[i] - ts[i - 1]) / 1000.0f32;
                if dt > 1.0e-9f32 {
                    held = (x[i] - x[i - 1]) / dt;
                }
            }
            v0 = held;
        } else if formula == 933u32 {
            // short window `a`, long window `b` (a < b).
            let s = win_start(ts, i, b);
            let long_count = (i + 1 - s) as f32;
            let mut short_count = 0.0f32;
            for j in s..(i + 1) {
                if ts[j] >= ts[i] - a {
                    short_count = short_count + 1.0f32;
                }
            }
            let cur = short_count / (a / 1000.0f32);
            let base = long_count / (b / 1000.0f32);
            if base > 0.0f32 {
                v0 = cur / base;
            }
        } else if formula == 934u32 {
            let s = win_start(ts, i, a);
            let total = (i + 1 - s) as f32;
            let mut mc = period;
            if mc < 1u32 {
                mc = 1u32;
            }
            if (i + 1 - s) as u32 >= mc {
                let mut buys = 0.0f32;
                for j in s..(i + 1) {
                    if side[j] > 0.0f32 {
                        buys = buys + 1.0f32;
                    }
                }
                let br = buys / total;
                let sr = (total - buys) / total;
                if br >= c {
                    v0 = 1.0f32;
                } else if sr >= c {
                    v0 = -1.0f32;
                }
            }
        } else if formula == 935u32 {
            let s = win_start(ts, i, a);
            let mut ssum = 0.0f32;
            let mut asum = 0.0f32;
            for j in s..(i + 1) {
                if x[n + j] > b {
                    asum = asum + x[n + j];
                    if side[j] > 0.0f32 {
                        ssum = ssum + x[n + j];
                    } else {
                        ssum = ssum - x[n + j];
                    }
                }
            }
            if asum > 0.0f32 {
                let mut m = ssum / asum;
                if m > 1.0f32 {
                    m = 1.0f32;
                }
                if m < -1.0f32 {
                    m = -1.0f32;
                }
                v0 = m;
            }
        } else if formula == 936u32 {
            let s = win_start(ts, i, a);
            let mut ssum = 0.0f32;
            let mut asum = 0.0f32;
            for j in s..(i + 1) {
                asum = asum + x[n + j];
                if side[j] > 0.0f32 {
                    ssum = ssum + x[n + j];
                } else {
                    ssum = ssum - x[n + j];
                }
            }
            if asum > 0.0f32 {
                let mut m = ssum / asum;
                if m > 1.0f32 {
                    m = 1.0f32;
                }
                if m < -1.0f32 {
                    m = -1.0f32;
                }
                v0 = m;
            }
        }
        out[i] = v0;
        out[n + i] = v1;
        out[2 * n + i] = v2;
    }
}

#[cube(launch_unchecked)]
fn evt_map(
    x: &[f32],
    side: &[f32],
    ts: &[f32],
    out: &mut [f32],
    formula: u32,
    period: u32,
    a: f32,
    b: f32,
    c: f32,
) {
    evt_scan(x, side, ts, out, formula, period, a, b, c);
}

fn launch_events_timed(
    formula: CubeFormula,
    frame: &GpuEventFrame,
    params: CubeParams,
) -> Vec<Vec<f32>> {
    let n = frame.len();
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let xb = client.create_from_slice(f32::as_bytes(&frame.x));
    let sb = client.create_from_slice(f32::as_bytes(&frame.side));
    let tb = client.create_from_slice(f32::as_bytes(&frame.ts));
    let out = client.empty(3 * n * core::mem::size_of::<f32>());
    unsafe {
        evt_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(xb, frame.x.len()),
            BufferArg::from_raw_parts(sb, n),
            BufferArg::from_raw_parts(tb, n),
            BufferArg::from_raw_parts(out.clone(), 3 * n),
            formula.code(),
            params.period,
            params.a.max(1.0),
            params.b,
            params.c,
        );
    }
    let bytes = client.read_one_unchecked(out);
    let flat = f32::from_bytes(&bytes).to_vec();
    let cols = formula.output_count() as usize;
    (0..cols).map(|k| flat[k * n..(k + 1) * n].to_vec()).collect()
}

/// Run an event formula over `frame`. One column, or two for 917.
pub fn launch_cube_events(
    formula: CubeFormula,
    frame: &GpuEventFrame,
    params: CubeParams,
) -> Vec<Vec<f32>> {
    let n = frame.len();
    if n == 0 {
        return Vec::new();
    }
    if formula.code() >= 940 {
        return super::kernels_evx::launch_cube_events_x(formula, frame, params);
    }
    if formula.code() >= 920 {
        return launch_events_timed(formula, frame, params);
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let xb = client.create_from_slice(f32::as_bytes(&frame.x));
    let sb = client.create_from_slice(f32::as_bytes(&frame.side));
    let out = client.empty(2 * n * core::mem::size_of::<f32>());
    unsafe {
        ev_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(xb, frame.x.len()),
            BufferArg::from_raw_parts(sb, n),
            BufferArg::from_raw_parts(out.clone(), 2 * n),
            formula.code(),
            params.period,
            params.a,
            params.b,
            params.c,
        );
    }
    let bytes = client.read_one_unchecked(out);
    let flat = f32::from_bytes(&bytes).to_vec();
    let cols = formula.output_count() as usize;
    (0..cols).map(|k| flat[k * n..(k + 1) * n].to_vec()).collect()
}
