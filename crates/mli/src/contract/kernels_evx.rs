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
use super::kernels_ev::{ev_mean, win_start};
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
    c: f32,
) {
    let n = side.len();
    let mut h0 = 0.0f32;
    let mut h1 = 0.0f32;
    let mut h2 = 0.0f32;
    let mut h3 = 0.0f32;
    let mut pa = 0.0f32;
    let mut pb = 0.0f32;
    let mut cs = 0.0f32;
    let mut lv = 0usize;
    let mut lm = 0usize;
    let mut ll = 0usize;
    let mut li = 0usize;
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
        } else if formula == 980u32 {
            // book churn rate: mean changed levels (x0) over the last `period` (>= 1) deltas
            let mut w = period as usize;
            if w < 1 {
                w = 1;
            }
            let mut lo = 0usize;
            if i + 1 > w {
                lo = i + 1 - w;
            }
            v0 = ev_mean(x, 0, n, lo, i);
        } else if formula == 981u32 {
            // level replenishment rate: updated levels (x1) are the events; the last `period`
            // (>= 2) events across deltas, rate = count / span seconds (span >= 1 ms)
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            let mut total = 0.0f32;
            for j in 0..(i + 1) {
                total = total + x[n + j];
            }
            let mut cnt = total;
            if cnt > (w as f32) {
                cnt = w as f32;
            }
            if cnt < 2.0f32 {
                v0 = cnt;
            } else {
                let mut nj = i;
                while x[n + nj] <= 0.0f32 {
                    nj = nj - 1;
                }
                let mut acc = 0.0f32;
                let mut oj = nj;
                let mut go = true;
                let mut j = nj + 1;
                while go && j > 0 {
                    j = j - 1;
                    acc = acc + x[n + j];
                    oj = j;
                    if acc >= cnt {
                        go = false;
                    }
                }
                let mut span = ts[nj] - ts[oj];
                if span < 1.0f32 {
                    span = 1.0f32;
                }
                v0 = cnt / (span / 1000.0f32);
            }
        } else if formula == 982u32 {
            // quote stuffing `[rate, signal]`: deltas in the last `a` ms per second, `b` threshold
            let s = win_start(ts, i, a);
            v0 = ((i + 1 - s) as f32) / (a / 1000.0f32);
            let mut th = b;
            if th < 0.0f32 {
                th = 0.0f32;
            }
            if v0 > th {
                v1 = 1.0f32;
            }
        } else if formula == 983u32 {
            // basis extreme: +1 above the 95th / -1 below the 5th percentile of the previous
            // `period` (>= 3) values (needs 2 of them)
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
                let mut k95 = len * 95 / 100;
                if k95 > len - 1 {
                    k95 = len - 1;
                }
                let k5 = len * 5 / 100;
                let p95 = kth_in(x, lo, len, k95 as u32);
                let p5 = kth_in(x, lo, len, k5 as u32);
                if x[i] > p95 {
                    v0 = 1.0f32;
                } else if x[i] < p5 {
                    v0 = -1.0f32;
                }
            }
        } else if formula == 1070u32 {
            // funding drift: slot 1 predicted rate (x0) minus slot 2 funding rate (x1)
            v0 = x[i] - x[n + i];
        } else if formula == 1071u32 {
            // funding x OI pressure `[funding, oi_delta, pressure]`: slot 1 funding (x0),
            // slot 2 open interest (x1); delta of the last two OI events
            if side[i] == 2.0f32 {
                pb = h1;
                h1 = x[n + i];
                pa = pa + 1.0f32;
                if pa >= 2.0f32 {
                    h2 = h1 - pb;
                }
            }
            v0 = x[i];
            v1 = h2;
            v2 = x[i] * h2;
        } else if formula == 1072u32 {
            // funding vs price momentum divergence `[funding_slope, price_slope, signal]`:
            // slot 1 funding (x0), slot 2 price (x1); `period` funding EMA period, `b` price
            // EMA period; the signal needs both counts at their periods
            let mut fp = period as f32;
            if fp < 1.0f32 {
                fp = 1.0f32;
            }
            let mut pp = b;
            if pp < 1.0f32 {
                pp = 1.0f32;
            }
            if side[i] == 1.0f32 {
                let prev = h0;
                if pa == 0.0f32 {
                    h0 = x[i];
                } else {
                    h0 = h0 + (2.0f32 / (fp + 1.0f32)) * (x[i] - h0);
                }
                pa = pa + 1.0f32;
                h1 = h0 - prev;
            } else {
                let mut prev = h2;
                if pb == 0.0f32 {
                    h2 = x[n + i];
                    prev = h2;
                } else {
                    h2 = h2 + (2.0f32 / (pp + 1.0f32)) * (x[n + i] - h2);
                }
                pb = pb + 1.0f32;
                cs = h2 - prev;
            }
            v0 = h1;
            v1 = cs;
            if pa >= fp && pb >= pp {
                if h1 > 0.0f32 && cs < 0.0f32 {
                    v2 = 1.0f32;
                } else if h1 < 0.0f32 && cs > 0.0f32 {
                    v2 = -1.0f32;
                }
            }
        } else if formula == 1073u32 {
            // funding / sentiment alignment: slot 1 funding (x0), slot 2 long ratio (x1),
            // 0.5 until the first ratio
            if side[i] == 2.0f32 {
                pa = 1.0f32;
            }
            let mut lr = 0.5f32;
            if pa > 0.0f32 {
                lr = x[n + i];
            }
            if x[i] > 0.0f32 && lr > 0.5f32 {
                v0 = 1.0f32;
            } else if x[i] < 0.0f32 && lr < 0.5f32 {
                v0 = -1.0f32;
            }
        } else if formula == 1074u32 {
            // IV/HV spread `[iv, hv, spread]`: slot 1 historical vol (x0), slot 2 vol index (x1)
            v0 = x[n + i];
            v1 = x[i];
            v2 = x[n + i] - x[i];
        } else if formula == 1075u32 {
            // long squeeze detector: slot 1 open interest (x0), slot 2 mark price (x1);
            // `prev` values only move once a value was seen before; the signal holds
            // between recomputes and starts at 0
            if side[i] == 1.0f32 {
                if pa > 0.0f32 {
                    pb = h1;
                }
                h1 = x[i];
                pa = pa + 1.0f32;
            } else {
                if cs > 0.0f32 {
                    h2 = h0;
                }
                h0 = x[n + i];
                cs = cs + 1.0f32;
            }
            if pa > 0.0f32 && cs > 0.0f32 {
                let doi = h1 - pb;
                let dpr = h0 - h2;
                h3 = 0.0f32;
                if doi < 0.0f32 && dpr < 0.0f32 {
                    h3 = 1.0f32;
                } else if doi < 0.0f32 && dpr > 0.0f32 {
                    h3 = -1.0f32;
                }
            }
            v0 = h3;
        } else if formula == 1076u32 {
            // mark vs last traded `[deviation, deviation_pct]`: slot 1 mark (x0), slot 2 ticker
            // last price (x1, taken only when > 0)
            if side[i] == 1.0f32 {
                pa = 1.0f32;
            } else if x[n + i] > 0.0f32 {
                h0 = x[n + i];
            }
            if pa > 0.0f32 && h0 > 0.0f32 && h0 >= 1.0e-12f32 {
                // h0 holds the last accepted traded price
                v0 = x[i] - h0;
                v1 = (x[i] - h0) / h0 * 100.0f32;
            }
        } else if formula == 1077u32 {
            // index tracking error: slot 1 index price (x0), slot 2 composite (x1); every row
            // with both > 0 pushes `index - composite`; population std of the last `period`
            // (>= 2) diffs, needs 2
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            let mut cnt = 0usize;
            let mut sum = 0.0f32;
            let mut j = i + 1;
            while cnt < w && j > 0 {
                j = j - 1;
                if x[j] > 0.0f32 && x[n + j] > 0.0f32 {
                    cnt = cnt + 1;
                    sum = sum + (x[j] - x[n + j]);
                }
            }
            if cnt >= 2 {
                let mean = sum / (cnt as f32);
                let mut ss = 0.0f32;
                let mut c2 = 0usize;
                let mut j2 = i + 1;
                while c2 < cnt && j2 > 0 {
                    j2 = j2 - 1;
                    if x[j2] > 0.0f32 && x[n + j2] > 0.0f32 {
                        c2 = c2 + 1;
                        let dd = x[j2] - x[n + j2] - mean;
                        ss = ss + dd * dd;
                    }
                }
                v0 = (ss / (cnt as f32)).sqrt();
            }
        } else if formula == 1078u32 {
            // OI / price correlation: slot 1 open interest (x0), slot 2 mark (x1); a pair is
            // pushed on every row once both streams have been seen; Pearson over the last
            // `period` (>= 2) pairs, clamped to [-1, 1], needs 2 pairs
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            if side[i] == 1.0f32 && pa == 0.0f32 {
                pa = 1.0f32;
                h0 = i as f32;
            }
            if side[i] == 2.0f32 && pb == 0.0f32 {
                pb = 1.0f32;
                h1 = i as f32;
            }
            if pa > 0.0f32 && pb > 0.0f32 {
                let mut st = h0 as usize;
                if (h1 as usize) > st {
                    st = h1 as usize;
                }
                let mut lo = st;
                if i + 1 > w && i + 1 - w > lo {
                    lo = i + 1 - w;
                }
                let cnt = i + 1 - lo;
                if cnt >= 2 {
                    let nf = cnt as f32;
                    let mut sx = 0.0f32;
                    let mut sy = 0.0f32;
                    for j in lo..(i + 1) {
                        sx = sx + x[j];
                        sy = sy + x[n + j];
                    }
                    let mx = sx / nf;
                    let my = sy / nf;
                    let mut cov = 0.0f32;
                    let mut vx = 0.0f32;
                    let mut vy = 0.0f32;
                    for j in lo..(i + 1) {
                        let dx = x[j] - mx;
                        let dy = x[n + j] - my;
                        cov = cov + dx * dy;
                        vx = vx + dx * dx;
                        vy = vy + dy * dy;
                    }
                    let den = (vx * vy).sqrt();
                    if den > 0.0f32 {
                        v0 = cov / den;
                        if v0 > 1.0f32 {
                            v0 = 1.0f32;
                        }
                        if v0 < -1.0f32 {
                            v0 = -1.0f32;
                        }
                    }
                }
                cs = v0;
            }
            v0 = cs;
        } else if formula == 1079u32 {
            // price vs index spread `[price, index, spread]`: slot 1 mark (x0), slot 2 index
            // (x1). The CPU outputs NaN until a stream was seen; the launcher patches those
            // rows to NaN on the host, the kernel writes 0 there.
            v0 = x[i];
            v1 = x[n + i];
            v2 = x[i] - x[n + i];
        } else if formula == 1080u32 {
            // vol regime entry: slot 1 volatility index (x0), slot 2 mark price (x1);
            // `period` history length (>= 4). History = the positive vol values seen before the
            // latest vol event (last `period` of them); 75th percentile of that history.
            let mut hl = period as usize;
            if hl < 4 {
                hl = 4;
            }
            if side[i] == 1.0f32 {
                lv = i + 1;
            } else {
                h1 = h0;
                h0 = x[n + i];
            }
            let mut sig = 0.0f32;
            if lv > 0 {
                let l = lv - 1;
                // lower bound row of the history window
                let mut cnt = 0usize;
                let mut lb = 0usize;
                let mut j = l;
                while cnt < hl && j > 0 {
                    j = j - 1;
                    if side[j] == 1.0f32 && x[j] > 0.0f32 {
                        cnt = cnt + 1;
                        lb = j;
                    }
                }
                if cnt >= 4 && x[l] > 0.0f32 {
                    let mut k = (cnt as f32 * 0.75f32) as usize;
                    if k > cnt - 1 {
                        k = cnt - 1;
                    }
                    let mut p75 = 0.0f32;
                    for q in lb..l {
                        if side[q] == 1.0f32 && x[q] > 0.0f32 {
                            let mut lt = 0usize;
                            let mut le = 0usize;
                            for r in lb..l {
                                if side[r] == 1.0f32 && x[r] > 0.0f32 {
                                    if x[r] < x[q] {
                                        lt = lt + 1;
                                    }
                                    if x[r] <= x[q] {
                                        le = le + 1;
                                    }
                                }
                            }
                            if lt <= k && k < le {
                                p75 = x[q];
                            }
                        }
                    }
                    let is_high = x[l] >= p75;
                    let falling = h1 > 0.0f32 && h0 < h1;
                    if is_high && h2 == 0.0f32 && falling {
                        sig = 1.0f32;
                    } else if !is_high && h2 > 0.0f32 {
                        sig = -1.0f32;
                    }
                    if is_high {
                        h2 = 1.0f32;
                    } else {
                        h2 = 0.0f32;
                    }
                }
            }
            v0 = sig;
        } else if formula == 1081u32 {
            // settlement vs mark spread `[settlement, mark, spread]`: slot 1 settlement price
            // (x0), slot 2 mark (x1)
            v0 = x[i];
            v1 = x[n + i];
            v2 = x[i] - x[n + i];
        } else if formula == 1082u32 || formula == 1086u32 {
            // squeeze probability `[prob, direction]`: slot 1 open interest (x0), slot 2 mark
            // (x1), slot 3 liquidation (x2, counted); `a` window ms, `b` max expected
            // liquidations (>= 1). Each stream's window ends at its own latest event.
            if side[i] == 1.0f32 {
                lv = i + 1;
            } else if side[i] == 2.0f32 {
                lm = i + 1;
            } else {
                ll = i + 1;
            }
            let mut oi_score = 0.0f32;
            if lv > 0 {
                let l = lv - 1;
                let mut cnt = 1usize;
                let mut old = x[l];
                let mut j = l;
                let mut go = true;
                while go && j > 0 {
                    j = j - 1;
                    if side[j] == 1.0f32 {
                        if ts[l] - ts[j] <= a {
                            cnt = cnt + 1;
                            old = x[j];
                        } else {
                            go = false;
                        }
                    }
                }
                if cnt >= 2 && old > 0.0f32 {
                    let mut d = x[l] - old;
                    if d < 0.0f32 {
                        d = 0.0f32 - d;
                    }
                    oi_score = d / old * 10.0f32;
                    if oi_score > 1.0f32 {
                        oi_score = 1.0f32;
                    }
                }
            }
            let mut pr_score = 0.0f32;
            if lm > 0 {
                let l = lm - 1;
                let mut cnt = 1usize;
                let mut old = x[n + l];
                let mut j = l;
                let mut go = true;
                while go && j > 0 {
                    j = j - 1;
                    if side[j] == 2.0f32 {
                        if ts[l] - ts[j] <= a {
                            cnt = cnt + 1;
                            old = x[n + j];
                        } else {
                            go = false;
                        }
                    }
                }
                if cnt >= 2 && old > 0.0f32 {
                    let rel = (x[n + l] - old) / old;
                    let mut ar = rel;
                    if ar < 0.0f32 {
                        ar = 0.0f32 - ar;
                    }
                    pr_score = ar * 100.0f32;
                    if pr_score > 1.0f32 {
                        pr_score = 1.0f32;
                    }
                    if rel > 0.0f32 {
                        v1 = 1.0f32;
                    } else if rel < 0.0f32 {
                        v1 = -1.0f32;
                    }
                }
            }
            let mut liq_score = 0.0f32;
            if ll > 0 {
                let l = ll - 1;
                let mut cnt = 1.0f32;
                let mut j = l;
                let mut go = true;
                while go && j > 0 {
                    j = j - 1;
                    if side[j] == 3.0f32 {
                        if ts[l] - ts[j] <= a {
                            cnt = cnt + 1.0f32;
                        } else {
                            go = false;
                        }
                    }
                }
                let mut mx = b;
                if mx < 1.0f32 {
                    mx = 1.0f32;
                }
                liq_score = cnt / mx;
                if liq_score > 1.0f32 {
                    liq_score = 1.0f32;
                }
            }
            if formula == 1086u32 {
                // compound squeeze: slot 4 funding (x3); 1000x |rate| clamped to 1
                let mut fa = x[3 * n + i];
                if fa < 0.0f32 {
                    fa = 0.0f32 - fa;
                }
                let mut fs = fa * 1000.0f32;
                if fs > 1.0f32 {
                    fs = 1.0f32;
                }
                v0 = 0.3f32 * oi_score + 0.25f32 * liq_score + 0.25f32 * pr_score + 0.2f32 * fs;
            } else {
                v0 = 0.4f32 * oi_score + 0.3f32 * pr_score + 0.3f32 * liq_score;
            }
        } else if formula == 1083u32 {
            // risk-off detector: slots [vol index (x0), liquidation (x1), funding (x2),
            // insurance fund balance (x3)]; `a` window ms, `b` vol threshold, `period` liq count
            // threshold, `c` funding threshold (percent). Liquidations older than the current
            // event's window are dropped on every event.
            if side[i] == 4.0f32 {
                li = i + 1;
            }
            let mut liqn = 0.0f32;
            let mut j = i + 1;
            let mut go = true;
            while go && j > 0 {
                j = j - 1;
                if side[j] == 2.0f32 {
                    if ts[i] - ts[j] <= a {
                        liqn = liqn + 1.0f32;
                    } else {
                        go = false;
                    }
                }
            }
            let mut depleting = false;
            if li > 0 {
                let l = li - 1;
                let mut cnt = 1usize;
                let mut first = l;
                let mut q = l;
                while cnt < 10 && q > 0 {
                    q = q - 1;
                    if side[q] == 4.0f32 {
                        cnt = cnt + 1;
                        first = q;
                    }
                }
                if cnt >= 2 && x[3 * n + l] < x[3 * n + first] {
                    depleting = true;
                }
            }
            let mut fa = x[2 * n + i];
            if fa < 0.0f32 {
                fa = 0.0f32 - fa;
            }
            let mut act = 0.0f32;
            if x[i] > b {
                act = act + 1.0f32;
            }
            if liqn >= (period as f32) {
                act = act + 1.0f32;
            }
            if fa * 100.0f32 > c {
                act = act + 1.0f32;
            }
            if depleting {
                act = act + 1.0f32;
            }
            if act >= 2.0f32 {
                v0 = 1.0f32;
            }
        } else if formula == 1084u32 {
            // market stress composite: same slots; `a` window ms (liquidations, ends at the
            // latest liquidation), `b` max expected liquidations (>= 1), `period` vol history
            // cap (>= 4), `c` depletion slope threshold (per ms)
            let mut cap = period as usize;
            if cap < 4 {
                cap = 4;
            }
            if side[i] == 1.0f32 {
                lv = i + 1;
            } else if side[i] == 2.0f32 {
                ll = i + 1;
            } else if side[i] == 4.0f32 {
                li = i + 1;
            }
            // vol score
            let mut vol_score = 0.0f32;
            if lv > 0 {
                let l = lv - 1;
                let mut cnt = 1usize;
                let mut lb = l;
                let mut q = l;
                while cnt < cap && q > 0 {
                    q = q - 1;
                    if side[q] == 1.0f32 {
                        cnt = cnt + 1;
                        lb = q;
                    }
                }
                let mut k = (cnt as f32 * 0.95f32) as usize;
                if k > cnt - 1 {
                    k = cnt - 1;
                }
                let mut p95 = 0.0f32;
                for r in lb..(l + 1) {
                    if side[r] == 1.0f32 {
                        let mut lt = 0usize;
                        let mut le = 0usize;
                        for u in lb..(l + 1) {
                            if side[u] == 1.0f32 {
                                if x[u] < x[r] {
                                    lt = lt + 1;
                                }
                                if x[u] <= x[r] {
                                    le = le + 1;
                                }
                            }
                        }
                        if lt <= k && k < le {
                            p95 = x[r];
                        }
                    }
                }
                if p95 < 1.0e-12f32 {
                    p95 = 1.0e-12f32;
                }
                vol_score = x[l] / p95;
                if vol_score > 1.0f32 {
                    vol_score = 1.0f32;
                }
                if vol_score < 0.0f32 {
                    vol_score = 0.0f32;
                }
            } else {
                // empty history: p95 = 1.0, current vol 0
                vol_score = 0.0f32;
            }
            let mut liq_score = 0.0f32;
            if ll > 0 {
                let l = ll - 1;
                let mut cnt = 1.0f32;
                let mut j = l;
                let mut go = true;
                while go && j > 0 {
                    j = j - 1;
                    if side[j] == 2.0f32 {
                        if ts[l] - ts[j] <= a {
                            cnt = cnt + 1.0f32;
                        } else {
                            go = false;
                        }
                    }
                }
                let mut mx = b;
                if mx < 1.0f32 {
                    mx = 1.0f32;
                }
                liq_score = cnt / mx;
                if liq_score > 1.0f32 {
                    liq_score = 1.0f32;
                }
            }
            let mut fa = x[2 * n + i];
            if fa < 0.0f32 {
                fa = 0.0f32 - fa;
            }
            let mut fund_score = fa * 100.0f32;
            if fund_score > 1.0f32 {
                fund_score = 1.0f32;
            }
            let mut dep = 0.0f32;
            if li > 0 {
                let l = li - 1;
                let mut cnt = 1usize;
                let mut first = l;
                let mut q = l;
                while cnt < 10 && q > 0 {
                    q = q - 1;
                    if side[q] == 4.0f32 {
                        cnt = cnt + 1;
                        first = q;
                    }
                }
                if cnt >= 2 {
                    let mut dt = ts[l] - ts[first];
                    if dt < 1.0f32 {
                        dt = 1.0f32;
                    }
                    let slope = (x[3 * n + l] - x[3 * n + first]) / dt;
                    if slope < c {
                        dep = 1.0f32;
                    }
                }
            }
            v0 = 0.3f32 * vol_score + 0.3f32 * liq_score + 0.2f32 * fund_score + 0.2f32 * dep;
        } else if formula == 1085u32 {
            // sentiment composite: slots [long/short ratio (x0), agg trade (x1 price, x2
            // quantity, x3 is_buy), funding (x4)]; `a` window ms (agg trades, ends at the
            // latest agg trade). Mean of ratio norm, taker flow imbalance, funding norm.
            if side[i] == 1.0f32 {
                pa = 1.0f32;
            } else if side[i] == 2.0f32 {
                ll = i + 1;
            }
            let mut lsn = 0.0f32;
            if pa > 0.0f32 {
                lsn = (x[i] - 0.5f32) * 2.0f32;
                if lsn > 1.0f32 {
                    lsn = 1.0f32;
                }
                if lsn < -1.0f32 {
                    lsn = -1.0f32;
                }
            }
            let mut fl = 0.0f32;
            if ll > 0 {
                let l = ll - 1;
                let mut buy = 0.0f32;
                let mut tot = 0.0f32;
                let mut j = l + 1;
                let mut go = true;
                while go && j > 0 {
                    j = j - 1;
                    if side[j] == 2.0f32 {
                        if ts[l] - ts[j] <= a {
                            let q = x[n + j] * x[2 * n + j];
                            tot = tot + q;
                            // CPU stores `!is_buy` and counts those as "buy" volume
                            if x[3 * n + j] < 0.5f32 {
                                buy = buy + q;
                            }
                        } else {
                            go = false;
                        }
                    }
                }
                if tot >= 1.0e-12f32 {
                    fl = buy / tot * 2.0f32 - 1.0f32;
                    if fl > 1.0f32 {
                        fl = 1.0f32;
                    }
                    if fl < -1.0f32 {
                        fl = -1.0f32;
                    }
                }
            }
            let mut fnn = x[4 * n + i] * 1000.0f32;
            if fnn > 1.0f32 {
                fnn = 1.0f32;
            }
            if fnn < -1.0f32 {
                fnn = -1.0f32;
            }
            v0 = (lsn + fl + fnn) / 3.0f32;
        } else if formula == 1087u32 {
            // capitulation detector: slots [liquidation (x0 side: +1 buy / -1 sell), agg trade
            // (x1 price, x2 quantity), mark (x3)]; `period` liq spike count, `a` window ms,
            // `b` volume spike threshold. Every stream's window ends at its own latest event.
            if side[i] == 1.0f32 {
                lv = i + 1;
            } else if side[i] == 2.0f32 {
                lm = i + 1;
            } else {
                ll = i + 1;
            }
            let mut longs = 0.0f32;
            let mut shorts = 0.0f32;
            if lv > 0 {
                let l = lv - 1;
                let mut j = l + 1;
                let mut go = true;
                while go && j > 0 {
                    j = j - 1;
                    if side[j] == 1.0f32 {
                        if ts[l] - ts[j] <= a {
                            if x[j] > 0.0f32 {
                                longs = longs + 1.0f32;
                            } else {
                                shorts = shorts + 1.0f32;
                            }
                        } else {
                            go = false;
                        }
                    }
                }
            }
            let mut vsum = 0.0f32;
            if lm > 0 {
                let l = lm - 1;
                let mut j = l + 1;
                let mut go = true;
                while go && j > 0 {
                    j = j - 1;
                    if side[j] == 2.0f32 {
                        if ts[l] - ts[j] <= a {
                            vsum = vsum + x[n + j] * x[2 * n + j];
                        } else {
                            go = false;
                        }
                    }
                }
            }
            let spike = vsum >= b;
            let thr = period as f32;
            let mut dropped = false;
            let mut bounced = false;
            let mut rose = false;
            let mut fell = false;
            if ll > 0 {
                let l = ll - 1;
                let mut cnt = 0usize;
                let mut mn = x[3 * n + l];
                let mut mx = mn;
                let mut old = mn;
                let mut j = l + 1;
                let mut go = true;
                while go && j > 0 {
                    j = j - 1;
                    if side[j] == 3.0f32 {
                        if ts[l] - ts[j] <= a {
                            cnt = cnt + 1;
                            let pv = x[3 * n + j];
                            old = pv;
                            if pv < mn {
                                mn = pv;
                            }
                            if pv > mx {
                                mx = pv;
                            }
                        } else {
                            go = false;
                        }
                    }
                }
                if cnt >= 3 {
                    let nw = x[3 * n + l];
                    dropped = old > 0.0f32 && (mn - old) / old < -0.01f32;
                    bounced = nw > mn * 1.005f32;
                    rose = old > 0.0f32 && (mx - old) / old > 0.01f32;
                    fell = nw < mx * 0.995f32;
                }
            }
            if longs >= thr && spike && dropped && bounced {
                v0 = 1.0f32;
            } else if shorts >= thr && spike && rose && fell {
                v0 = -1.0f32;
            }
        } else if formula == 1088u32 {
            // block trade volume ratio: slots [block trade (x0 price, x1 quantity, x2 is_iv),
            // agg trade (x3 price, x4 quantity)]; `a` window ms. IV block trades are ignored
            // entirely (the output holds). Both windows end at the current event.
            let mut ignore = false;
            if side[i] == 1.0f32 && x[2 * n + i] > 0.5f32 {
                ignore = true;
            }
            if !ignore {
                let mut bs = 0.0f32;
                let mut ag = 0.0f32;
                let mut j = i + 1;
                let mut go = true;
                while go && j > 0 {
                    j = j - 1;
                    if ts[i] - ts[j] <= a {
                        if side[j] == 1.0f32 {
                            if x[2 * n + j] <= 0.5f32 {
                                bs = bs + x[j] * x[n + j];
                            }
                        } else {
                            ag = ag + x[3 * n + j] * x[4 * n + j];
                        }
                    } else {
                        go = false;
                    }
                }
                h0 = 0.0f32;
                if ag > 0.0f32 {
                    h0 = bs / ag;
                }
            }
            v0 = h0;
        } else if formula == 1089u32 {
            // stop hunt detector: slots [liquidation (x0 quote value, x1 side +1 buy / -1 sell),
            // mark (x2)]; `a` reversal window ms, `b` spike threshold (USD). The signal is
            // recomputed on mark events only and held between them.
            if side[i] == 2.0f32 {
                let mut cnt = 0usize;
                let mut old = x[2 * n + i];
                let mut j = i + 1;
                let mut go = true;
                while go && j > 0 {
                    j = j - 1;
                    if side[j] == 2.0f32 {
                        if ts[i] - ts[j] <= a {
                            cnt = cnt + 1;
                            old = x[2 * n + j];
                        } else {
                            go = false;
                        }
                    }
                }
                h0 = 0.0f32;
                if cnt >= 2 {
                    let mut lv_ = 0.0f32;
                    let mut sv_ = 0.0f32;
                    let mut q = i + 1;
                    let mut go2 = true;
                    while go2 && q > 0 {
                        q = q - 1;
                        if ts[i] - ts[q] <= a {
                            if side[q] == 1.0f32 {
                                if x[n + q] > 0.0f32 {
                                    lv_ = lv_ + x[q];
                                } else {
                                    sv_ = sv_ + x[q];
                                }
                            }
                        } else {
                            go2 = false;
                        }
                    }
                    let nw = x[2 * n + i];
                    if lv_ >= b && nw > old {
                        h0 = 1.0f32;
                    } else if sv_ >= b && nw < old {
                        h0 = -1.0f32;
                    }
                }
            }
            v0 = h0;
        } else if formula == 1090u32 {
            // ratio vs price divergence `[score, side]`: slots [long ratio (x0), price (x1)];
            // `period` history (>= 2) of each; recomputed on every event once both have 2
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            // collect bounds of each stream's last-w rows
            let mut rc = 0usize;
            let mut rlo = i;
            let mut rhi = i;
            let mut rfound = false;
            let mut pc = 0usize;
            let mut plo = i;
            let mut phi = i;
            let mut pfound = false;
            let mut j = i + 1;
            while j > 0 {
                j = j - 1;
                if side[j] == 1.0f32 && rc < w {
                    rc = rc + 1;
                    rlo = j;
                    if !rfound {
                        rhi = j;
                        rfound = true;
                    }
                }
                if side[j] == 2.0f32 && pc < w {
                    pc = pc + 1;
                    plo = j;
                    if !pfound {
                        phi = j;
                        pfound = true;
                    }
                }
            }
            if rc >= 2 && pc >= 2 {
                let dr = x[rhi] - x[rlo];
                let dp = x[n + phi] - x[n + plo];
                let mut adr = dr;
                if adr < 0.0f32 {
                    adr = 0.0f32 - adr;
                }
                let mut adp = dp;
                if adp < 0.0f32 {
                    adp = 0.0f32 - adp;
                }
                let sr = dr >= 0.0f32;
                let sp = dp >= 0.0f32;
                h0 = 0.0f32;
                h1 = 0.0f32;
                if sr != sp && adr > 1.0e-9f32 && adp > 1.0e-9f32 {
                    let mut rmax = x[rlo];
                    let mut rmin = x[rlo];
                    let mut pmax = x[n + plo];
                    let mut pmin = x[n + plo];
                    let mut rcnt = 0usize;
                    let mut pcnt = 0usize;
                    for q in 0..(i + 1) {
                        if side[q] == 1.0f32 && q >= rlo && rcnt < rc {
                            rcnt = rcnt + 1;
                            if x[q] > rmax {
                                rmax = x[q];
                            }
                            if x[q] < rmin {
                                rmin = x[q];
                            }
                        }
                        if side[q] == 2.0f32 && q >= plo && pcnt < pc {
                            pcnt = pcnt + 1;
                            if x[n + q] > pmax {
                                pmax = x[n + q];
                            }
                            if x[n + q] < pmin {
                                pmin = x[n + q];
                            }
                        }
                    }
                    let mut rn = 0.0f32;
                    if rmax - rmin > 1.0e-9f32 {
                        rn = adr / (rmax - rmin);
                    }
                    let mut pn = 0.0f32;
                    if pmax - pmin > 1.0e-9f32 {
                        pn = adp / (pmax - pmin);
                    }
                    h0 = rn * pn;
                    if h0 > 1.0f32 {
                        h0 = 1.0f32;
                    }
                    if dr > 0.0f32 {
                        h1 = 1.0f32;
                    } else {
                        h1 = -1.0f32;
                    }
                }
            }
            v0 = h0;
            v1 = h1;
        } else if formula == 1091u32 {
            // funding settlement impact: slots [settlement (x0 settlement time, ms from the
            // first row), mark (x1)]; `period` mark buffer (>= 4). Impact (after - before) /
            // before around a settlement time, pending until two marks straddle it.
            let mut bs = period as usize;
            if bs < 4 {
                bs = 4;
            }
            let mut do_calc = false;
            let mut st = 0.0f32;
            if side[i] == 1.0f32 {
                do_calc = true;
                st = x[i];
            } else if pa > 0.0f32 {
                do_calc = true;
                st = h1;
            }
            if do_calc {
                let mut cnt = 0usize;
                let mut before = 0.0f32;
                let mut has_b = false;
                let mut after = 0.0f32;
                let mut has_a = false;
                let mut j = i + 1;
                while cnt < bs && j > 0 {
                    j = j - 1;
                    if side[j] == 2.0f32 {
                        cnt = cnt + 1;
                        if ts[j] <= st {
                            if !has_b {
                                before = x[n + j];
                                has_b = true;
                            }
                        } else {
                            after = x[n + j];
                            has_a = true;
                        }
                    }
                }
                let mut imp = 0.0f32;
                if cnt >= 2 && has_b && has_a {
                    let mut ab = before;
                    if ab < 0.0f32 {
                        ab = 0.0f32 - ab;
                    }
                    if ab > 1.0e-15f32 {
                        imp = (after - before) / before;
                    }
                }
                let mut aimp = imp;
                if aimp < 0.0f32 {
                    aimp = 0.0f32 - aimp;
                }
                if aimp > 0.0f32 {
                    h2 = imp;
                    pa = 0.0f32;
                } else if side[i] == 1.0f32 {
                    pa = 1.0f32;
                    h1 = st;
                }
            }
            v0 = h2;
        } else if formula == 990u32 {
            // tick CVD: sum of signed size (side * size) over the last max(period, 1) ticks
            let mut w = period as usize;
            if w < 1usize {
                w = 1usize;
            }
            let mut lo = 0usize;
            if i + 1 > w {
                lo = i + 1 - w;
            }
            let mut sm = 0.0f32;
            for q in lo..(i + 1) {
                sm = sm + side[q] * x[n + q];
            }
            v0 = sm;
        } else if formula == 991u32 {
            // tick volume delta: signed size of the current tick
            v0 = side[i] * x[n + i];
        } else if formula == 992u32 {
            // VPIN: volume buckets of size `a`; completed bucket |buy - sell| / a goes to out[n + k]
            // (cols 1 and 2 are not written for this formula), value = mean of the last
            // clamp(period, 1, n) buckets. h0 = buy, h1 = sell, h2 = bucket volume, cs = buckets, h3 = last
            let bs = a.max(1.0e-9f32);
            let mut sw = period as usize;
            if sw < 1usize {
                sw = 1usize;
            }
            if side[i] > 0.0f32 {
                h0 = h0 + x[n + i];
            } else {
                h1 = h1 + x[n + i];
            }
            h2 = h2 + x[n + i];
            if h2 >= bs {
                out[n + (cs as usize)] = (h0 - h1).abs() / bs;
                cs = cs + 1.0f32;
                h0 = 0.0f32;
                h1 = 0.0f32;
                h2 = 0.0f32;
            }
            let nb = cs as usize;
            if nb > 0usize {
                let mut lo = 0usize;
                if nb > sw {
                    lo = nb - sw;
                }
                let mut sm = 0.0f32;
                for q in lo..nb {
                    sm = sm + out[n + q];
                }
                h3 = sm / ((nb - lo) as f32);
            }
            v0 = h3;
        } else if formula == 993u32 {
            // absorption detector `[score, signal]` over the last max(period, 2) ticks (held until the window is full)
            let mut w = period as usize;
            if w < 2usize {
                w = 2usize;
            }
            if i + 1 >= w {
                let lo = i + 1 - w;
                let mut tv = 0.0f32;
                let mut bv = 0.0f32;
                for q in lo..(i + 1) {
                    tv = tv + x[n + q];
                    if side[q] > 0.0f32 {
                        bv = bv + x[n + q];
                    }
                }
                let pr = (x[i] - x[lo]).abs();
                if pr > 1.0e-9f32 {
                    h0 = tv / pr;
                } else if tv > 0.0f32 {
                    h0 = tv * 1000.0f32;
                } else {
                    h0 = 0.0f32;
                }
                let sv = tv - bv;
                if bv > sv * 1.5f32 {
                    h1 = 1.0f32;
                } else if sv > bv * 1.5f32 {
                    h1 = 0.0f32 - 1.0f32;
                } else {
                    h1 = 0.0f32;
                }
            }
            v0 = h0;
            v1 = h1;
        } else if formula == 994u32 {
            // adaptive window selector: `period` short window, `b` long window (>= short), `a` volatility threshold;
            // short window when the sample std of the last `short` prices exceeds it, else long
            let mut sw = period as usize;
            if sw < 2usize {
                sw = 2usize;
            }
            let mut lw = b as usize;
            if lw < sw {
                lw = sw;
            }
            let mut cnt = i + 1;
            if cnt > sw {
                cnt = sw;
            }
            let mut sd = 0.0f32;
            if cnt >= 2usize {
                let mut mean = 0.0f32;
                for q in (i + 1 - cnt)..(i + 1) {
                    mean = mean + x[q];
                }
                mean = mean / (cnt as f32);
                let mut va = 0.0f32;
                for q in (i + 1 - cnt)..(i + 1) {
                    va = va + (x[q] - mean) * (x[q] - mean);
                }
                sd = (va / ((cnt - 1) as f32)).sqrt();
            }
            if sd > a {
                v0 = sw as f32;
            } else {
                v0 = lw as f32;
            }
        } else if formula == 995u32 {
            // warning frequency filter: x0 = interned kind id, `a` = min interval ms. h0 = last kind (0 = none),
            // pa = last emission ts; emit (1) when no previous kind, the kind changed, or ts - last > a
            let mut emit = false;
            if h0 == 0.0f32 {
                emit = true;
            } else if h0 != x[i] {
                emit = true;
            } else if ts[i] - pa > a.max(0.0f32) {
                emit = true;
            }
            if emit {
                h0 = x[i];
                pa = ts[i];
                v0 = 1.0f32;
            }
        } else if formula == 1402u32 {
            // quote lifecycle tracker: x2 = action (1 add, 3 delete), x3 = interned order id (1-based).
            // out[n + id] = pending add time + 1 (0 = none), out[2n + k] = k-th completed lifetime;
            // value = mean of the last max(period, 2) lifetimes (0 before the first)
            if i == 0usize {
                for q in 0..(2usize * n) {
                    out[n + q] = 0.0f32;
                }
            }
            let act = x[2usize * n + i];
            let id = x[3usize * n + i] as usize;
            if act == 1.0f32 {
                out[n + id - 1usize] = ts[i] + 1.0f32;
            } else if act == 3.0f32 {
                let pend = out[n + id - 1usize];
                if pend > 0.0f32 {
                    out[n + id - 1usize] = 0.0f32;
                    out[2usize * n + (cs as usize)] = (ts[i] - (pend - 1.0f32)).max(0.0f32);
                    cs = cs + 1.0f32;
                    let mut sw = period as usize;
                    if sw < 2usize {
                        sw = 2usize;
                    }
                    let nb = cs as usize;
                    let mut lo = 0usize;
                    if nb > sw {
                        lo = nb - sw;
                    }
                    let mut sm = 0.0f32;
                    for q in lo..nb {
                        sm = sm + out[2usize * n + q];
                    }
                    h3 = sm / ((nb - lo) as f32);
                }
            }
            v0 = h3;
        } else if formula == 996u32 {
            // gamma squeeze detector (OptionGreeks stream): the CPU consumer never receives a price on this
            // stream (prev / last price stay NaN), so `price_moved` is false and the signal is always 0;
            // the gamma test is kept for parity of the expression
            if x[n + i] > a && false {
                v0 = 1.0f32;
            }
        }
        out[i] = v0;
        if formula != 992u32 && formula != 1402u32 {
            out[n + i] = v1;
            out[2 * n + i] = v2;
        }
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
    c: f32,
) {
    evx_scan(x, side, ts, out, formula, period, a, b, c);
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
            params.c,
        );
    }
    let bytes = client.read_one_unchecked(out);
    let flat = f32::from_bytes(&bytes).to_vec();
    let cols = formula.output_count() as usize;
    let mut res: Vec<Vec<f32>> = (0..cols).map(|k| flat[k * n..(k + 1) * n].to_vec()).collect();
    if formula == CubeFormula::PriceVsIndexMg {
        // CPU: price / index are NaN until their stream was seen, spread until both were.
        let (mut sp, mut si) = (false, false);
        for i in 0..n {
            if frame.side[i] == 1.0 {
                sp = true;
            } else if frame.side[i] == 2.0 {
                si = true;
            }
            if !sp {
                res[0][i] = f32::NAN;
            }
            if !si {
                res[1][i] = f32::NAN;
            }
            if !(sp && si) {
                res[2][i] = f32::NAN;
            }
        }
    }
    res
}
