//! More event-frame formulas, codes 940..=999 (L3, tick-window and vol-index rows).
//!
//! Same frame and conventions as [`super::kernels_ev`]: column-major `x[c * n + i]`, `side`,
//! `ts` in ms from the first event. Output column `c` of event `i` is `out[c * n + i]`
//! (up to four columns). UNTESTED on GPU (no GPU on the authoring box).
//!
//! L3 frame (`GpuEventFrame::from_l3`): `x0` price, `x1` quantity, `x2` action
//! (`1` add, `2` modify, `3` delete), `side` `1` bid / `-1` ask.

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::event_frame::GpuEventFrame;
use super::gpu::CubeParams;
use super::kernels_ev::win_start;
use super::kernels_post::kth_in;
use super::CubeFormula;

/// Median of `src[start..start + len]` (mean of the two middle values when `len` is even).
#[cube]
fn med_in(src: &[f32], start: usize, len: usize) -> f32 {
    let mut r = 0.0f32;
    if len > 0 {
        if len % 2 == 1 {
            r = kth_in(src, start, len, (len / 2) as u32);
        } else {
            let a = kth_in(src, start, len, (len / 2 - 1) as u32);
            let b = kth_in(src, start, len, (len / 2) as u32);
            r = (a + b) / 2.0f32;
        }
    }
    r
}

/// 940 L3 cancel ratio (`period` events), 941 auction price deviation (always 0: the CPU
/// indicator has no close in the contract path), 942 L3 order rate (`a` window ms, per
/// second), 943 L3 spoofer score (`period` window >= 4, `a` size multiplier >= 1),
/// 944 L3 large order `[side, size, price]` (`period` window >= 2, `a` multiplier),
/// 945 trade cluster `[signal, price, size]` (`a` bucket, `period` threshold >= 2, `b` window ms),
/// 946 volume imbalance zone `[side, low, high]` (`a` window ms, `b` delta threshold),
/// 947 VWAP deviation `[price, vwap, deviation]` (`a` window ms), 948 vol-index spike (`period`).
#[cube]
fn evx_scan(
    x: &[f32],
    side: &[f32],
    ts: &[f32],
    out: &mut [f32],
    formula: u32,
    period: u32,
    a: f32,
    b: f32,
) {
    let n = side.len();
    let mut h0 = 0.0f32;
    let mut h1 = 0.0f32;
    for i in 0..n {
        let mut v0 = 0.0f32;
        let mut v1 = 0.0f32;
        let mut v2 = 0.0f32;
        if formula == 940u32 {
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            let mut lo = 0usize;
            if i + 1 > w {
                lo = i + 1 - w;
            }
            let mut add = 0.0f32;
            let mut del = 0.0f32;
            for j in lo..(i + 1) {
                let act = x[2 * n + j];
                if act == 1.0f32 {
                    add = add + 1.0f32;
                }
                if act == 3.0f32 {
                    del = del + 1.0f32;
                }
            }
            if add > 0.0f32 {
                v0 = del / add;
            }
        } else if formula == 942u32 {
            let s = win_start(ts, i, a);
            v0 = ((i + 1 - s) as f32) / (a / 1000.0f32);
        } else if formula == 943u32 {
            let mut w = period as usize;
            if w < 4 {
                w = 4;
            }
            let mut m = a;
            if m < 1.0f32 {
                m = 1.0f32;
            }
            let mut lo = 0usize;
            if i + 1 > w {
                lo = i + 1 - w;
            }
            let len = i + 1 - lo;
            let mut add = 0.0f32;
            let mut del = 0.0f32;
            for j in lo..(i + 1) {
                let act = x[2 * n + j];
                if act == 1.0f32 {
                    add = add + 1.0f32;
                }
                if act == 3.0f32 {
                    del = del + 1.0f32;
                }
            }
            let mut cr = 0.0f32;
            if add > 0.0f32 {
                cr = del / add;
                if cr > 1.0f32 {
                    cr = 1.0f32;
                }
            }
            let med = med_in(x, n + lo, len);
            let thr = med * m;
            let mut large = 0.0f32;
            for j in lo..(i + 1) {
                if x[n + j] > thr {
                    large = large + 1.0f32;
                }
            }
            v0 = (cr + large / (len as f32)) / 2.0f32;
        } else if formula == 944u32 {
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            let mut lo = 0usize;
            if i + 1 > w {
                lo = i + 1 - w;
            }
            let med = med_in(x, n + lo, i + 1 - lo);
            if med > 0.0f32 && x[n + i] > a * med {
                v0 = 0.0f32 - side[i];
            }
            v1 = x[n + i];
            v2 = x[i];
        } else if formula == 945u32 {
            let mut bucket = a;
            if bucket <= 0.0f32 {
                bucket = 0.01f32;
            }
            let mut th = period as f32;
            if th < 2.0f32 {
                th = 2.0f32;
            }
            let s = win_start(ts, i, b);
            let bid = (x[i] / bucket).floor();
            let mut cnt = 0.0f32;
            let mut buys = 0.0f32;
            let mut sz = 0.0f32;
            for j in s..(i + 1) {
                if (x[j] / bucket).floor() == bid {
                    cnt = cnt + 1.0f32;
                    sz = sz + x[n + j];
                    if side[j] > 0.0f32 {
                        buys = buys + 1.0f32;
                    }
                }
            }
            if cnt >= th {
                h0 = bid * bucket;
                h1 = sz;
                let sells = cnt - buys;
                if buys > sells {
                    v0 = 1.0f32;
                } else if sells > buys {
                    v0 = -1.0f32;
                }
            }
            v1 = h0;
            v2 = h1;
        } else if formula == 946u32 {
            let s = win_start(ts, i, a);
            let mut bv = 0.0f32;
            let mut sv = 0.0f32;
            for j in s..(i + 1) {
                if side[j] > 0.0f32 {
                    bv = bv + x[n + j];
                } else {
                    sv = sv + x[n + j];
                }
            }
            let total = bv + sv;
            if total > 1.0e-12f32 {
                let mut d = bv - sv;
                if d < 0.0f32 {
                    d = 0.0f32 - d;
                }
                let mut th = b;
                if th < 1.0e-12f32 {
                    th = 1.0e-12f32;
                }
                if th > 1.0f32 {
                    th = 1.0f32;
                }
                if d / total > th {
                    let dom_buy = bv >= sv;
                    let mut lo = 0.0f32;
                    let mut hi = 0.0f32;
                    let mut any = false;
                    for j in s..(i + 1) {
                        let jb = side[j] > 0.0f32;
                        if jb == dom_buy {
                            if !any {
                                lo = x[j];
                                hi = x[j];
                                any = true;
                            } else {
                                if x[j] < lo {
                                    lo = x[j];
                                }
                                if x[j] > hi {
                                    hi = x[j];
                                }
                            }
                        }
                    }
                    if dom_buy {
                        v0 = 1.0f32;
                    } else {
                        v0 = -1.0f32;
                    }
                    v1 = lo;
                    v2 = hi;
                }
            }
        } else if formula == 947u32 {
            let s = win_start(ts, i, a);
            let mut q = 0.0f32;
            let mut pq = 0.0f32;
            for j in s..(i + 1) {
                q = q + x[n + j];
                pq = pq + x[j] * x[n + j];
            }
            let mut vw = x[i];
            if q > 0.0f32 {
                vw = pq / q;
            }
            v0 = x[i];
            v1 = vw;
            if vw > 0.0f32 {
                v2 = (x[i] - vw) / vw;
            }
        } else if formula == 948u32 {
            let mut w = period as usize;
            if w < 3 {
                w = 3;
            }
            let mut lo = 0usize;
            if i > w {
                lo = i - w;
            }
            let len = i - lo;
            if len >= 2 {
                let mut k = len * 95 / 100;
                if k > len - 1 {
                    k = len - 1;
                }
                let p95 = kth_in(x, lo, len, k as u32);
                if x[i] > p95 {
                    v0 = 1.0f32;
                }
            }
        } else if formula == 949u32 {
            // tick volume analyzer value: cumulative buy minus sell volume
            if side[i] > 0.0f32 {
                h0 = h0 + x[n + i];
            } else {
                h0 = h0 - x[n + i];
            }
            v0 = h0;
        } else if formula == 950u32 || formula == 951u32 {
            // trade flow imbalance `[imbalance, volume]` / uptick-downtick volume `[up, down]`
            // over the last `period` ticks (at least 1)
            let mut w = period as usize;
            if w < 1 {
                w = 1;
            }
            let mut lo = 0usize;
            if i + 1 > w {
                lo = i + 1 - w;
            }
            let mut bv = 0.0f32;
            let mut sv = 0.0f32;
            for j in lo..(i + 1) {
                if side[j] > 0.0f32 {
                    bv = bv + x[n + j];
                } else {
                    sv = sv + x[n + j];
                }
            }
            if formula == 950u32 {
                let tot = bv + sv;
                v1 = tot;
                if tot > 0.0f32 {
                    v0 = (bv - sv) / tot;
                }
            } else {
                v0 = bv;
                v1 = sv;
            }
        } else if formula == 952u32 {
            // funding extreme alert `[signal, magnitude]`: window `period` >= 2, sigma `a`,
            // value = x0, history includes the current value
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            let mut lo = 0usize;
            if i + 1 > w {
                lo = i + 1 - w;
            }
            let cnt = i + 1 - lo;
            if cnt >= 2 {
                let nn = cnt as f32;
                let mut sum = 0.0f32;
                for j in lo..(i + 1) {
                    sum = sum + x[j];
                }
                let mean = sum / nn;
                let mut ss = 0.0f32;
                for j in lo..(i + 1) {
                    let dd = x[j] - mean;
                    ss = ss + dd * dd;
                }
                let sd = (ss / nn).sqrt();
                if sd >= 1.0e-15f32 {
                    let dev = x[i] - mean;
                    let mut mag = dev / sd;
                    if mag < 0.0f32 {
                        mag = 0.0f32 - mag;
                    }
                    v1 = mag;
                    if mag > a {
                        if dev > 0.0f32 {
                            v0 = 1.0f32;
                        } else {
                            v0 = -1.0f32;
                        }
                    }
                }
            }
        } else if formula == 953u32 || formula == 954u32 {
            // EMA momentum `[ema, slope]` of funding rate (x0) or index price (x1, mark when
            // absent / 0): alpha = 2 / (period + 1), seeded by the first value
            let mut p = period as f32;
            if p < 1.0f32 {
                p = 1.0f32;
            }
            let mut price = x[i];
            if formula == 954u32 && x[n + i] != 0.0f32 {
                price = x[n + i];
            }
            let prev = h0;
            if i == 0 {
                h0 = price;
            } else {
                h0 = h0 + (2.0f32 / (p + 1.0f32)) * (price - h0);
            }
            v0 = h0;
            v1 = h0 - prev;
        } else if formula == 955u32 {
            // mark price gap detector `[signal, jump, sigma ratio]`: window `period` >= 2,
            // sigma `a`, mark = x0
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            let mut lo = 0usize;
            if i + 1 > w {
                lo = i + 1 - w;
            }
            let mut jump = 0.0f32;
            let mut dir = 0.0f32;
            if i > 0 {
                jump = x[i] - x[i - 1];
                if jump > 0.0f32 {
                    dir = 1.0f32;
                } else if jump < 0.0f32 {
                    dir = -1.0f32;
                    jump = 0.0f32 - jump;
                }
            }
            let cnt = i + 1 - lo;
            let mut sd = 0.0f32;
            if cnt >= 2 {
                let nn = cnt as f32;
                let mut sum = 0.0f32;
                for j in lo..(i + 1) {
                    sum = sum + x[j];
                }
                let mean = sum / nn;
                let mut ss = 0.0f32;
                for j in lo..(i + 1) {
                    let dd = x[j] - mean;
                    ss = ss + dd * dd;
                }
                sd = (ss / nn).sqrt();
            }
            if sd > 1.0e-15f32 {
                v2 = jump / sd;
            }
            if v2 > a {
                v0 = dir;
            }
            v1 = jump;
        } else if formula == 956u32 {
            // adaptive threshold `[mean, std, threshold]`: window `period` >= 2 of prices (x0),
            // sample std, `a` multiplier
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            let mut lo = 0usize;
            if i + 1 > w {
                lo = i + 1 - w;
            }
            let cnt = i + 1 - lo;
            let nn = cnt as f32;
            let mut sum = 0.0f32;
            for j in lo..(i + 1) {
                sum = sum + x[j];
            }
            let mean = sum / nn;
            let mut sd = 0.0f32;
            if cnt >= 2 {
                let mut ss = 0.0f32;
                for j in lo..(i + 1) {
                    let dd = x[j] - mean;
                    ss = ss + dd * dd;
                }
                sd = (ss / (nn - 1.0f32)).sqrt();
            }
            v0 = mean;
            v1 = sd;
            v2 = mean + a * sd;
        } else if formula == 957u32 {
            // large trade filter `[signal, ratio]`: window `period` >= 2, multiplier `a`
            // (non-positive -> 2); outputs stay 0 until the window is full
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            let mut m = a;
            if m <= 0.0f32 {
                m = 2.0f32;
            }
            if i + 1 >= w {
                let med = med_in(x, n + i + 1 - w, w);
                let mut ratio = 0.0f32;
                if med > 1.0e-12f32 {
                    ratio = x[n + i] / med;
                }
                h1 = ratio;
                h0 = 0.0f32;
                if ratio >= m {
                    if side[i] > 0.0f32 {
                        h0 = 1.0f32;
                    } else {
                        h0 = -1.0f32;
                    }
                }
            }
            v0 = h0;
            v1 = h1;
        } else if formula == 958u32 {
            // agg-trade size distribution `[median, p95, current]` over the last `period`
            // (>= 1) quantities (x1): sorted[len / 2] and sorted[(len - 1) * 0.95]
            let mut w = period as usize;
            if w < 1 {
                w = 1;
            }
            let mut lo = 0usize;
            if i + 1 > w {
                lo = i + 1 - w;
            }
            let len = i + 1 - lo;
            v0 = kth_in(x, n + lo, len, (len / 2) as u32);
            v1 = kth_in(x, n + lo, len, ((len - 1) * 95 / 100) as u32);
            v2 = x[n + i];
        } else if formula == 959u32 {
            // liquidation cluster detector `[price, count, volume]`: `a` window ms, `b` price
            // bucket, `period` min count. The CPU picks the max-count bucket through a HashMap
            // (tie order is arbitrary there); this takes the earliest event's bucket on ties.
            let s = win_start(ts, i, a);
            let mut best_cnt = 0.0f32;
            let mut best_vol = 0.0f32;
            let mut best_bucket = 0.0f32;
            for j in s..(i + 1) {
                let bj = (x[j] / b).floor();
                let mut cnt = 0.0f32;
                let mut vol = 0.0f32;
                for k in s..(i + 1) {
                    if (x[k] / b).floor() == bj {
                        cnt = cnt + 1.0f32;
                        vol = vol + x[2 * n + k];
                    }
                }
                if cnt > best_cnt {
                    best_cnt = cnt;
                    best_vol = vol;
                    best_bucket = bj;
                }
            }
            if best_cnt >= (period as f32) {
                v0 = best_bucket * b + b * 0.5f32;
                v1 = best_cnt;
                v2 = best_vol;
            }
        }
        out[i] = v0;
        out[n + i] = v1;
        out[2 * n + i] = v2;
    }
}

#[cube(launch_unchecked)]
fn evx_map(
    x: &[f32],
    side: &[f32],
    ts: &[f32],
    out: &mut [f32],
    formula: u32,
    period: u32,
    a: f32,
    b: f32,
) {
    evx_scan(x, side, ts, out, formula, period, a, b);
}

/// Run an event formula of codes 940..=999. One `Vec` per output column.
pub fn launch_cube_events_x(
    formula: CubeFormula,
    frame: &GpuEventFrame,
    params: CubeParams,
) -> Vec<Vec<f32>> {
    let n = frame.len();
    if n == 0 {
        return Vec::new();
    }
    if formula == CubeFormula::AuctionPriceDeviationEv {
        return vec![vec![0.0; n]];
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let xb = client.create_from_slice(f32::as_bytes(&frame.x));
    let sb = client.create_from_slice(f32::as_bytes(&frame.side));
    let tb = client.create_from_slice(f32::as_bytes(&frame.ts));
    let out = client.empty(3 * n * core::mem::size_of::<f32>());
    unsafe {
        evx_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(xb, frame.x.len()),
            BufferArg::from_raw_parts(sb, n),
            BufferArg::from_raw_parts(tb, n),
            BufferArg::from_raw_parts(out.clone(), 3 * n),
            formula.code(),
            params.period,
            params.a,
            params.b,
        );
    }
    let bytes = client.read_one_unchecked(out);
    let flat = f32::from_bytes(&bytes).to_vec();
    let cols = formula.output_count() as usize;
    (0..cols).map(|k| flat[k * n..(k + 1) * n].to_vec()).collect()
}
