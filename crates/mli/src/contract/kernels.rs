//! One CubeCL kernel for every [`CubeFormula`].
//!
//! The launch takes the same bar columns a factory feed reads (`open`, `high`,
//! `low`, `close`, `volume`) plus [`CubeParams`]: source lanes, `period`,
//! `fast` / `slow` / `signal`, and the extra coefficients (`a`, `b`, `flag`).
//! Time stays off this kernel. `MarketSample::Bar` carries no timestamp either.
//! CPU cores stay f64. This kernel is the f32 batch map.
//! It is compiled only with the `gpu` feature.

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::gpu::CubeParams;
use super::gpu_sample::GpuSample;
use super::CubeFormula;
#[cfg(test)]
use super::ResearchBar;

#[cube]
fn fld(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    i: usize,
    lane: u32,
) -> f32 {
    let mut v = close[i];
    if lane == 0u32 {
        v = open[i];
    } else if lane == 1u32 {
        v = high[i];
    } else if lane == 2u32 {
        v = low[i];
    } else if lane == 3u32 {
        v = close[i];
    } else if lane == 4u32 {
        v = volume[i];
    } else if lane == 5u32 {
        v = (high[i] + low[i]) / 2.0f32;
    } else if lane == 6u32 {
        v = (high[i] + low[i] + close[i]) / 3.0f32;
    } else if lane == 7u32 {
        v = (open[i] + high[i] + low[i] + close[i]) / 4.0f32;
    }
    v
}

#[cube]
fn wma_point(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    i: usize,
    wp: usize,
    lane: u32,
) -> f32 {
    let mut out = fld(open, high, low, close, volume, i, lane);
    if i + 1 >= wp {
        let start = i + 1 - wp;
        let mut acc = 0.0f32;
        let mut wsum = 0.0f32;
        for k in 0..wp {
            let w = (k + 1) as f32;
            acc = acc + fld(open, high, low, close, volume, start + k, lane) * w;
            wsum = wsum + w;
        }
        out = acc / wsum;
    }
    out
}

#[cube]
fn prefix_mean(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    i: usize,
    period: usize,
    lane: u32,
) -> f32 {
    let mut window = period;
    if window > i + 1 {
        window = i + 1;
    }
    let start = i + 1 - window;
    let mut acc = 0.0f32;
    for k in 0..window {
        acc = acc + fld(open, high, low, close, volume, start + k, lane);
    }
    acc / (window as f32)
}

#[cube]
fn depth_sum(sz: &[f32], nside: &[f32], i: usize, depth: usize, levels: usize) -> f32 {
    let mut n = nside[i] as usize;
    if n > depth {
        n = depth;
    }
    if n > levels {
        n = levels;
    }
    let base = i * depth;
    let mut acc = 0.0f32;
    for k in 0..n {
        acc = acc + sz[base + k];
    }
    acc
}

/// Newton square root. Zero for a non-positive input.
#[cube]
fn sqrt_f(x: f32) -> f32 {
    let mut y = 0.0f32;
    if x > 0.0f32 {
        y = x;
        for _step in 0..12 {
            y = 0.5f32 * (y + x / y);
        }
    }
    y
}

/// `slot` 0..3 reads `s0`..`s3`. Anything else is `s0`.
#[cube]
fn slot_at(s0: &[f32], s1: &[f32], s2: &[f32], s3: &[f32], i: usize, slot: u32) -> f32 {
    let mut v = s0[i];
    if slot == 1u32 {
        v = s1[i];
    } else if slot == 2u32 {
        v = s2[i];
    } else if slot == 3u32 {
        v = s3[i];
    }
    v
}

/// Sequential formulas. One unit writes the whole lane.
#[cube]
fn scan_lane(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    bid_px: &[f32],
    bid_sz: &[f32],
    ask_px: &[f32],
    ask_sz: &[f32],
    bid_n: &[f32],
    ask_n: &[f32],
    s0: &[f32],
    s1: &[f32],
    s2: &[f32],
    s3: &[f32],
    output: &mut [f32],
    lane: u32,
    lane2: u32,
    period: u32,
    fast: u32,
    slow: u32,
    signal: u32,
    a: f32,
    b: f32,
    flag: u32,
    depth: u32,
    levels: u32,
    slot: u32,
    formula: u32,
) {
    let n = open.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    let pf = p as f32;
    let alpha = 2.0f32 / (pf + 1.0f32);

    if formula == 5u32 {
        let mut y = fld(open, high, low, close, volume, 0, lane);
        output[0] = y;
        for k in 1..n {
            let x = fld(open, high, low, close, volume, k, lane);
            y = alpha * x + (1.0f32 - alpha) * y;
            output[k] = y;
        }
    } else if formula == 6u32 {
        let mut y = fld(open, high, low, close, volume, 0, lane);
        output[0] = y;
        for k in 1..n {
            let x = fld(open, high, low, close, volume, k, lane);
            y = (y * (pf - 1.0f32) + x) / pf;
            output[k] = y;
        }
    } else if formula == 7u32 {
        let mut e1 = fld(open, high, low, close, volume, 0, lane);
        let mut e2 = e1;
        output[0] = 2.0f32 * e1 - e2;
        for k in 1..n {
            let x = fld(open, high, low, close, volume, k, lane);
            e1 = alpha * x + (1.0f32 - alpha) * e1;
            e2 = alpha * e1 + (1.0f32 - alpha) * e2;
            output[k] = 2.0f32 * e1 - e2;
        }
    } else if formula == 8u32 {
        let mut e1 = fld(open, high, low, close, volume, 0, lane);
        let mut e2 = e1;
        let mut e3 = e2;
        output[0] = 3.0f32 * e1 - 3.0f32 * e2 + e3;
        for k in 1..n {
            let x = fld(open, high, low, close, volume, k, lane);
            e1 = alpha * x + (1.0f32 - alpha) * e1;
            e2 = alpha * e1 + (1.0f32 - alpha) * e2;
            e3 = alpha * e2 + (1.0f32 - alpha) * e3;
            output[k] = 3.0f32 * e1 - 3.0f32 * e2 + e3;
        }
    } else if formula == 9u32 {
        for idx in 0..n {
            let mut window = p;
            if window > idx + 1 {
                window = idx + 1;
            }
            let start = idx + 1 - window;
            let mut acc = 0.0f32;
            for j in 0..window {
                acc = acc + prefix_mean(open, high, low, close, volume, start + j, p, lane);
            }
            output[idx] = acc / (window as f32);
        }
    } else if formula == 10u32 {
        let mut half = p / 2;
        if half < 1 {
            half = 1;
        }
        let mut sq = 1usize;
        for cand in 1..p + 1 {
            if cand * cand <= p {
                sq = cand;
            }
        }
        for idx in 0..n {
            if idx + 1 < sq {
                let w1 = wma_point(open, high, low, close, volume, idx, half, lane);
                let w2 = wma_point(open, high, low, close, volume, idx, p, lane);
                output[idx] = 2.0f32 * w1 - w2;
            } else {
                let start = idx + 1 - sq;
                let mut acc = 0.0f32;
                let mut wsum = 0.0f32;
                for k in 0..sq {
                    let w1 = wma_point(open, high, low, close, volume, start + k, half, lane);
                    let w2 = wma_point(open, high, low, close, volume, start + k, p, lane);
                    let diff = 2.0f32 * w1 - w2;
                    let w = (k + 1) as f32;
                    acc = acc + diff * w;
                    wsum = wsum + w;
                }
                output[idx] = acc / wsum;
            }
        }
    } else if formula == 11u32 {
        let m = a * (pf - 1.0f32);
        let mut s = pf / b;
        if s < 1.0e-9 {
            s = 1.0e-9;
        }
        for idx in 0..n {
            if idx + 1 < p {
                output[idx] = 0.0f32;
            } else {
                let start = idx + 1 - p;
                let mut acc = 0.0f32;
                let mut wsum = 0.0f32;
                for k in 0..p {
                    let x = ((k as f32) - m) / s;
                    let wi = (-0.5f32 * x * x).exp();
                    acc = acc + fld(open, high, low, close, volume, start + k, lane) * wi;
                    wsum = wsum + wi;
                }
                output[idx] = acc / wsum;
            }
        }
    } else if formula == 12u32 {
        let one = 1.0f32 - a;
        let c6 = a * a * a;
        let c5 = 3.0f32 * a * a * one;
        let c4 = 3.0f32 * a * one * one;
        let c3 = one * one * one;
        let mut e1 = fld(open, high, low, close, volume, 0, lane);
        let mut e2 = e1;
        let mut e3 = e2;
        let mut e4 = e3;
        let mut e5 = e4;
        let mut e6 = e5;
        output[0] = e6 * c6 + e5 * c5 + e4 * c4 + e3 * c3;
        for k in 1..n {
            let x = fld(open, high, low, close, volume, k, lane);
            e1 = alpha * x + (1.0f32 - alpha) * e1;
            e2 = alpha * e1 + (1.0f32 - alpha) * e2;
            e3 = alpha * e2 + (1.0f32 - alpha) * e3;
            e4 = alpha * e3 + (1.0f32 - alpha) * e4;
            e5 = alpha * e4 + (1.0f32 - alpha) * e5;
            e6 = alpha * e5 + (1.0f32 - alpha) * e6;
            output[k] = e6 * c6 + e5 * c5 + e4 * c4 + e3 * c3;
        }
    } else if formula == 13u32 {
        let mut y = fld(open, high, low, close, volume, 0, lane);
        output[0] = y;
        for k in 1..n {
            let x = fld(open, high, low, close, volume, k, lane);
            if y == 0.0f32 {
                y = x;
            } else {
                let mut ratio = x / y;
                if ratio < 0.0f32 {
                    ratio = -ratio;
                }
                let r2 = ratio * ratio;
                let mut denom = pf * r2 * r2;
                if denom < 1.0e-9 {
                    denom = 1.0e-9;
                }
                y = y + (x - y) / denom;
            }
            output[k] = y;
        }
    } else if formula == 14u32 {
        for idx in 0..n {
            if idx + 1 < p {
                output[idx] = 0.0f32;
            } else {
                let prev = fld(open, high, low, close, volume, idx - (p - 1), lane);
                let x = fld(open, high, low, close, volume, idx, lane);
                if flag == 1u32 {
                    let ratio = x / prev;
                    output[idx] = ratio.ln() / (10.0f32).ln();
                } else {
                    output[idx] = (x - prev) / prev;
                }
            }
        }
    } else if formula == 15u32 {
        output[0] = 0.0f32;
        let mut prev = fld(open, high, low, close, volume, 0, lane);
        let mut gain = 0.0f32;
        let mut loss = 0.0f32;
        let mut gcount = 0usize;
        let mut lcount = 0usize;
        let mut value = 0.0f32;
        for k in 1..n {
            let x = fld(open, high, low, close, volume, k, lane);
            let diff = x - prev;
            let mut up = 0.0f32;
            let mut down = 0.0f32;
            if diff > 0.0f32 {
                up = diff;
            } else {
                down = -diff;
            }
            prev = x;
            if gcount == 0 {
                gain = up;
            } else {
                gain = (gain * (pf - 1.0f32) + up) / pf;
            }
            gcount = gcount + 1;
            if lcount == 0 {
                loss = down;
            } else {
                loss = (loss * (pf - 1.0f32) + down) / pf;
            }
            lcount = lcount + 1;
            if gcount >= p && lcount >= p {
                let mut la = loss;
                if la < 0.0f32 {
                    la = -la;
                }
                if la < 1.0e-12 {
                    value = 100.0f32;
                } else {
                    let rs = gain / loss;
                    value = 100.0f32 * (1.0f32 - (1.0f32 / (1.0f32 + rs)));
                }
            }
            output[k] = value;
        }
    } else if formula == 16u32 {
        output[0] = 0.0f32;
        let mut prev = fld(open, high, low, close, volume, 0, lane);
        let mut gain = 0.0f32;
        let mut loss = 0.0f32;
        let mut gcount = 0usize;
        let mut lcount = 0usize;
        let mut index = 1usize;
        for k in 1..n {
            let x = fld(open, high, low, close, volume, k, lane);
            let diff = x - prev;
            let mut up = 0.0f32;
            let mut down = 0.0f32;
            if diff > 0.0f32 {
                up = diff;
            } else {
                down = -diff;
            }
            prev = x;
            if gcount == 0 {
                gain = up;
            } else {
                gain = (gain * (pf - 1.0f32) + up) / pf;
            }
            gcount = gcount + 1;
            if lcount == 0 {
                loss = down;
            } else {
                loss = (loss * (pf - 1.0f32) + down) / pf;
            }
            lcount = lcount + 1;
            index = index + 1;
            let denom = gain + loss;
            let mut da = denom;
            if da < 0.0f32 {
                da = -da;
            }
            if index >= p && da >= 1.0e-12 {
                output[k] = 100.0f32 * (gain - loss) / denom;
            } else {
                output[k] = 0.0f32;
            }
        }
    } else if formula == 17u32 {
        for idx in 0..n {
            if idx + 1 < p {
                output[idx] = 0.0f32;
            } else {
                let start = idx + 1 - p;
                let mut acc = 0.0f32;
                for j in 0..p {
                    acc = acc + fld(open, high, low, close, volume, start + j, lane);
                }
                let sma = acc / pf;
                let mut sa = sma;
                if sa < 0.0f32 {
                    sa = -sa;
                }
                let x = fld(open, high, low, close, volume, idx, lane);
                if sa < 1.0e-12 {
                    output[idx] = 0.0f32;
                } else {
                    output[idx] = x / sma - 1.0f32;
                }
            }
        }
    } else if formula == 18u32 {
        let mut hl = high[0] - low[0];
        if hl < 0.0f32 {
            hl = -hl;
        }
        output[0] = hl;
        let mut pc = close[0];
        for k in 1..n {
            let mut range = high[k] - low[k];
            if range < 0.0f32 {
                range = -range;
            }
            let mut hg = high[k] - pc;
            if hg < 0.0f32 {
                hg = -hg;
            }
            let mut lg = low[k] - pc;
            if lg < 0.0f32 {
                lg = -lg;
            }
            let mut tr = range;
            if hg > tr {
                tr = hg;
            }
            if lg > tr {
                tr = lg;
            }
            output[k] = tr;
            pc = close[k];
        }
    } else if formula == 19u32 {
        let mut y = high[0] - low[0];
        output[0] = y;
        let mut pc = close[0];
        for k in 1..n {
            let hl = high[k] - low[k];
            let mut hg = high[k] - pc;
            if hg < 0.0f32 {
                hg = -hg;
            }
            let mut lg = low[k] - pc;
            if lg < 0.0f32 {
                lg = -lg;
            }
            let mut tr = hl;
            if hg > tr {
                tr = hg;
            }
            if lg > tr {
                tr = lg;
            }
            y = (y * (pf - 1.0f32) + tr) / pf;
            output[k] = y;
            pc = close[k];
        }
    } else if formula == 21u32 {
        let mut value = 0.0f32;
        for idx in 0..n {
            let mut window = p;
            if window > idx + 1 {
                window = idx + 1;
            }
            let start = idx + 1 - window;
            let mut spv = 0.0f32;
            let mut sv = 0.0f32;
            for j in 0..window {
                let price = fld(open, high, low, close, volume, start + j, lane);
                let weight = fld(open, high, low, close, volume, start + j, lane2);
                spv = spv + price * weight;
                sv = sv + weight;
            }
            if sv > 0.0f32 {
                value = spv / sv;
            }
            output[idx] = value;
        }
    } else if formula == 22u32 || formula == 23u32 {
        let mut sp = signal as usize;
        if sp < 1 {
            sp = 1;
        }
        if formula == 22u32 && sp == 0 {
            output[0] = output[0];
        }
        let mut nf = fast as usize;
        if nf < 1 {
            nf = 1;
        }
        let mut ns = slow as usize;
        if ns < 1 {
            ns = 1;
        }
        let af = 2.0f32 / ((nf as f32) + 1.0f32);
        let aslow = 2.0f32 / ((ns as f32) + 1.0f32);
        let mut e1 = fld(open, high, low, close, volume, 0, lane);
        let mut e2 = fld(open, high, low, close, volume, 0, lane);
        if formula == 22u32 {
            e2 = fld(open, high, low, close, volume, 0, lane2);
        }
        output[0] = e1 - e2;
        for k in 1..n {
            let x1 = fld(open, high, low, close, volume, k, lane);
            let mut x2 = x1;
            if formula == 22u32 {
                x2 = fld(open, high, low, close, volume, k, lane2);
            }
            e1 = af * x1 + (1.0f32 - af) * e1;
            e2 = aslow * x2 + (1.0f32 - aslow) * e2;
            output[k] = e1 - e2;
        }
    } else if formula == 24u32 {
        let d = depth as usize;
        let mut y = 0.0f32;
        for i in 0..n {
            let bn = bid_n[i] as usize;
            let an = ask_n[i] as usize;
            if bn >= 1 && an >= 1 {
                let base = i * d;
                let bsz = bid_sz[base];
                let asz = ask_sz[base];
                let tot = bsz + asz;
                if tot > 0.0f32 {
                    y = (bsz * ask_px[base] + asz * bid_px[base]) / tot;
                }
            }
            output[i] = y;
        }
    } else if formula == 26u32 {
        let d = depth as usize;
        let mut lv = levels as usize;
        if lv < 1 {
            lv = 1;
        }
        let mut win = p;
        if win < 2 {
            win = 2;
        }
        for i in 0..n {
            let mut cnt = i + 1;
            if cnt > win {
                cnt = win;
            }
            if cnt < 2 {
                output[i] = 0.0f32;
            } else {
                let first = i + 1 - cnt;
                let bid_last = depth_sum(bid_sz, bid_n, i, d, lv);
                let bid_first = depth_sum(bid_sz, bid_n, first, d, lv);
                let ask_last = depth_sum(ask_sz, ask_n, i, d, lv);
                let ask_first = depth_sum(ask_sz, ask_n, first, d, lv);
                let denom = cnt as f32;
                output[i] = (bid_last - bid_first) / denom - (ask_last - ask_first) / denom;
            }
        }
    } else if formula == 28u32 || formula == 29u32 {
        let mut win = p;
        if win < 2 {
            win = 2;
        }
        for i in 0..n {
            let mut cnt = i + 1;
            if cnt > win {
                cnt = win;
            }
            if cnt < 2 {
                output[i] = 0.0f32;
            } else if formula == 28u32 {
                let first = i + 1 - cnt;
                let oldest = slot_at(s0, s1, s2, s3, first, slot);
                let latest = slot_at(s0, s1, s2, s3, i, slot);
                output[i] = (latest - oldest) / ((cnt - 1) as f32);
            } else {
                let start = i + 1 - cnt;
                let mut sum = 0.0f32;
                for k in 0..cnt {
                    sum = sum + slot_at(s0, s1, s2, s3, start + k, slot);
                }
                let mean = sum / (cnt as f32);
                let mut var = 0.0f32;
                for k in 0..cnt {
                    let d = slot_at(s0, s1, s2, s3, start + k, slot) - mean;
                    var = var + d * d;
                }
                var = var / (cnt as f32);
                let mut std = 0.0f32;
                if var > 0.0f32 {
                    std = var;
                    for _step in 0..12 {
                        std = 0.5f32 * (std + var / std);
                    }
                }
                if std == 0.0f32 {
                    output[i] = 0.0f32;
                } else {
                    let cur = slot_at(s0, s1, s2, s3, i, slot);
                    output[i] = (cur - mean) / std;
                }
            }
        }
    } else if formula == 31u32 {
        let mut win = p;
        if win < 2 {
            win = 2;
        }
        for i in 0..n {
            let mut cnt = i + 1;
            if cnt > win {
                cnt = win;
            }
            if cnt < 2 {
                output[i] = 0.0f32;
            } else {
                let start = i + 1 - cnt;
                let mut sum = 0.0f32;
                for k in 0..cnt {
                    sum = sum + slot_at(s0, s1, s2, s3, start + k, slot);
                }
                let mean = sum / (cnt as f32);
                let mut var = 0.0f32;
                for k in 0..cnt {
                    let d = slot_at(s0, s1, s2, s3, start + k, slot) - mean;
                    var = var + d * d;
                }
                output[i] = sqrt_f(var / (cnt as f32));
            }
        }
    } else if formula == 32u32 {
        let win = p;
        for i in 0..n {
            let mut cnt = i;
            if cnt > win {
                cnt = win;
            }
            if cnt == 0 {
                output[i] = 0.0f32;
            } else {
                let start = i - cnt;
                let cur = slot_at(s0, s1, s2, s3, i, slot);
                let mut below = 0.0f32;
                for k in 0..cnt {
                    if slot_at(s0, s1, s2, s3, start + k, slot) < cur {
                        below = below + 1.0f32;
                    }
                }
                output[i] = below / (cnt as f32);
            }
        }
    } else if formula == 33u32 {
        let win = p;
        for i in 0..n {
            let mut cnt = i + 1;
            if cnt > win {
                cnt = win;
            }
            let start = i + 1 - cnt;
            let mut sum = 0.0f32;
            for k in 0..cnt {
                sum = sum + slot_at(s0, s1, s2, s3, start + k, slot);
            }
            let mean = sum / (cnt as f32);
            if mean == 0.0f32 {
                output[i] = 1.0f32;
            } else {
                output[i] = slot_at(s0, s1, s2, s3, i, slot) / mean;
            }
        }
    } else if formula == 34u32 {
        let mut ep = p;
        if ep < 2 {
            ep = 2;
        }
        let alpha = 2.0f32 / ((ep as f32) + 1.0f32);
        let mut prev = slot_at(s0, s1, s2, s3, 0, slot);
        output[0] = 0.0f32;
        for k in 1..n {
            let x = slot_at(s0, s1, s2, s3, k, slot);
            let ema = alpha * x + (1.0f32 - alpha) * prev;
            output[k] = ema - prev;
            prev = ema;
        }
    }
}

#[cube(launch_unchecked)]
fn lane_map(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    bid_px: &[f32],
    bid_sz: &[f32],
    ask_px: &[f32],
    ask_sz: &[f32],
    bid_n: &[f32],
    ask_n: &[f32],
    s0: &[f32],
    s1: &[f32],
    s2: &[f32],
    s3: &[f32],
    side: &[f32],
    output: &mut [f32],
    lane: u32,
    lane2: u32,
    period: u32,
    fast: u32,
    slow: u32,
    signal: u32,
    a: f32,
    b: f32,
    flag: u32,
    depth: u32,
    levels: u32,
    slot: u32,
    formula: u32,
) {
    let i = ABSOLUTE_POS;
    if i < open.len() {
        let mut touch = s1[0] + s2[0] + s3[0] + side[0];
        if touch > 1.0e30 {
            touch = 0.0f32;
        }
        if formula == 20u32 {
            let mut denom = high[i] - low[i];
            if denom < 0.0f32 {
                denom = -denom;
            }
            if denom < 1.0e-12 {
                denom = 1.0e-12;
            }
            output[i] = (close[i] - open[i]) / denom;
        } else if formula == 25u32 {
            let d = depth as usize;
            let mut lv = levels as usize;
            if lv < 1 {
                lv = 1;
            }
            let bid = depth_sum(bid_sz, bid_n, i, d, lv);
            let ask = depth_sum(ask_sz, ask_n, i, d, lv);
            let tot = bid + ask;
            if tot > 0.0f32 {
                output[i] = (bid - ask) / tot;
            } else {
                output[i] = 0.0f32;
            }
        } else if formula == 27u32 {
            output[i] = s0[i] + touch * 0.0f32;
        } else if formula == 30u32 {
            output[i] = s0[i] * a;
        } else if formula >= 5u32 {
            if i == 0 {
                scan_lane(
                    open, high, low, close, volume, bid_px, bid_sz, ask_px, ask_sz, bid_n, ask_n,
                    s0, s1, s2, s3, output, lane, lane2, period, fast, slow, signal, a, b, flag,
                    depth, levels, slot, formula,
                );
            }
        } else if formula == 0u32 {
            output[i] = fld(open, high, low, close, volume, i, lane);
        } else {
            let mut p = period as usize;
            if p < 1 {
                p = 1;
            }
            let mut window = p;
            if window > i + 1 {
                window = i + 1;
            }
            let start = i + 1 - window;
            if formula == 4u32 {
                if i + 1 < p {
                    output[i] = fld(open, high, low, close, volume, i, lane);
                } else {
                    let mut acc = 0.0f32;
                    let mut wsum = 0.0f32;
                    for k in 0..p {
                        let w = (k + 1) as f32;
                        acc = acc
                            + fld(open, high, low, close, volume, start + k, lane) * w;
                        wsum = wsum + w;
                    }
                    output[i] = acc / wsum;
                }
            } else if formula == 2u32 {
                let mut best = fld(open, high, low, close, volume, start, lane);
                for k in 0..window {
                    let v = fld(open, high, low, close, volume, start + k, lane);
                    if v > best {
                        best = v;
                    }
                }
                output[i] = best;
            } else if formula == 3u32 {
                let mut best = fld(open, high, low, close, volume, start, lane);
                for k in 0..window {
                    let v = fld(open, high, low, close, volume, start + k, lane);
                    if v < best {
                        best = v;
                    }
                }
                output[i] = best;
            } else {
                let mut acc = 0.0f32;
                for k in 0..window {
                    acc = acc + fld(open, high, low, close, volume, start + k, lane);
                }
                output[i] = acc / (window as f32);
            }
        }
    }
}

/// Run one cube formula over GPU samples.
///
/// `samples` are the f32 image of the market stream (`GpuSample::Bar` for a
/// bar series, `GpuSample::Book` for a book, and the other stream arms).
/// `params` carries lanes, periods, and book `levels`. An empty slice returns
/// an empty buffer and does not create a device.
pub fn launch_cube(formula: CubeFormula, samples: &[GpuSample], params: CubeParams) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    let n = samples.len();
    let c = GpuSample::columns(samples);
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let open_b = client.create_from_slice(f32::as_bytes(&c.open));
    let high_b = client.create_from_slice(f32::as_bytes(&c.high));
    let low_b = client.create_from_slice(f32::as_bytes(&c.low));
    let close_b = client.create_from_slice(f32::as_bytes(&c.close));
    let volume_b = client.create_from_slice(f32::as_bytes(&c.volume));
    let bid_px_b = client.create_from_slice(f32::as_bytes(&c.bid_px));
    let bid_sz_b = client.create_from_slice(f32::as_bytes(&c.bid_sz));
    let ask_px_b = client.create_from_slice(f32::as_bytes(&c.ask_px));
    let ask_sz_b = client.create_from_slice(f32::as_bytes(&c.ask_sz));
    let bid_n_b = client.create_from_slice(f32::as_bytes(&c.bid_n));
    let ask_n_b = client.create_from_slice(f32::as_bytes(&c.ask_n));
    let s0_b = client.create_from_slice(f32::as_bytes(&c.s0));
    let s1_b = client.create_from_slice(f32::as_bytes(&c.s1));
    let s2_b = client.create_from_slice(f32::as_bytes(&c.s2));
    let s3_b = client.create_from_slice(f32::as_bytes(&c.s3));
    let side_b = client.create_from_slice(f32::as_bytes(&c.side));
    let output = client.empty(n * core::mem::size_of::<f32>());
    let dim = 64u32;
    let cubes = (n as u32).div_ceil(dim);
    let book_len = c.bid_px.len();
    unsafe {
        lane_map::launch_unchecked(
            &client,
            CubeCount::new_1d(cubes),
            CubeDim::new_1d(dim),
            BufferArg::from_raw_parts(open_b, n),
            BufferArg::from_raw_parts(high_b, n),
            BufferArg::from_raw_parts(low_b, n),
            BufferArg::from_raw_parts(close_b, n),
            BufferArg::from_raw_parts(volume_b, n),
            BufferArg::from_raw_parts(bid_px_b, book_len),
            BufferArg::from_raw_parts(bid_sz_b, book_len),
            BufferArg::from_raw_parts(ask_px_b, book_len),
            BufferArg::from_raw_parts(ask_sz_b, book_len),
            BufferArg::from_raw_parts(bid_n_b, n),
            BufferArg::from_raw_parts(ask_n_b, n),
            BufferArg::from_raw_parts(s0_b, n),
            BufferArg::from_raw_parts(s1_b, n),
            BufferArg::from_raw_parts(s2_b, n),
            BufferArg::from_raw_parts(s3_b, n),
            BufferArg::from_raw_parts(side_b, n),
            BufferArg::from_raw_parts(output.clone(), n),
            params.lane.code(),
            params.lane2.code(),
            params.period,
            params.fast,
            params.slow,
            params.signal,
            params.a,
            params.b,
            params.flag,
            c.depth,
            params.levels,
            params.slot,
            formula.code(),
        );
    }
    let bytes = client.read_one_unchecked(output);
    f32::from_bytes(&bytes).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ohlcv_field::OhlcvField;
    use crate::indicators::average::alma::Alma;
    use crate::indicators::average::dema::Dema;
    use crate::indicators::average::ema::Ema;
    use crate::indicators::average::hma::Hma;
    use crate::indicators::average::mcginley_dynamic::McGinleyDynamic;
    use crate::indicators::average::rma::Rma;
    use crate::indicators::average::sma::Sma;
    use crate::indicators::average::t3::T3;
    use crate::indicators::average::tema::Tema;
    use crate::indicators::average::tma::Tma;
    use crate::indicators::average::trima::Trima;
    use crate::indicators::average::vwma::Vwma;
    use crate::indicators::average::wma::Wma;
    use crate::indicators::momentum::apo::Apo;
    use crate::indicators::momentum::bias::Bias;
    use crate::indicators::momentum::bop::Bop;
    use crate::indicators::momentum::cmo::Cmo;
    use crate::indicators::momentum::macd::Macd;
    use crate::indicators::momentum::roc::Roc;
    use crate::indicators::momentum::rsi::Rsi;
    use crate::indicators::book::book_pressure::BookPressure;
    use crate::indicators::book::imbalance::BookImbalanceRatio;
    use crate::indicators::book::microprice::Microprice;
    use crate::indicators::book_advanced::bid_ask_asymmetry::BidAskAsymmetry;
    use crate::indicators::swing::highest::Highest;
    use crate::indicators::swing::lowest::Lowest;
    use crate::indicators::volatility::atr::Atr;
    use crate::indicators::volatility::true_range::TrueRange;
    use crate::core::types::{
        AuctionEvent, Basis, FundingRate, FundingSettlement, HistoricalVolatility, InsuranceFund,
        LongShortRatio, MarkPrice, OpenInterest, OptionGreeks, OrderBook, SettlementEvent, Ticker,
        VolatilityIndex,
    };
    use crate::engine::streams::order_book_consumer::OrderBookConsumer;
    use crate::engine::streams::{
        AuctionEventConsumer, BasisConsumer, FundingRateConsumer, FundingSettlementConsumer,
        HistoricalVolatilityConsumer, InsuranceFundConsumer, LongShortRatioConsumer,
        MarkPriceConsumer, OpenInterestConsumer, OptionGreeksConsumer, SettlementEventConsumer,
        TickerConsumer, VolatilityIndexConsumer,
    };
    use crate::indicators::auction::auction_imbalance::AuctionImbalance;
    use crate::indicators::funding_advanced::annualized_funding_rate::AnnualizedFundingRate;
    use crate::indicators::funding_advanced::funding_z_score::FundingZScore;
    use crate::indicators::funding_advanced::settled_funding_momentum::SettledFundingMomentum;
    use crate::indicators::greeks::delta_exposure_flow::DeltaExposureFlow;
    use crate::indicators::greeks::vega_exposure_flow::VegaExposureFlow;
    use crate::indicators::index_basis::basis_momentum::BasisMomentum;
    use crate::indicators::index_basis::basis_z_score::BasisZScore;
    use crate::indicators::mark_price_advanced::mark_price_momentum::MarkPriceMomentum;
    use crate::indicators::mark_price_advanced::mark_price_volatility::MarkPriceVolatility;
    use crate::indicators::open_interest::oi_momentum::OiMomentum;
    use crate::indicators::open_interest::oi_percentile::OiPercentile;
    use crate::indicators::open_interest::oi_z_score::OiZScore;
    use crate::indicators::sentiment::long_short_ratio_momentum::LongShortRatioMomentum;
    use crate::indicators::settlement::settlement_price_momentum::SettlementPriceMomentum;
    use crate::indicators::stress::fund_depletion_rate::FundDepletionRate;
    use crate::indicators::stress::insurance_fund_momentum::InsuranceFundMomentum;
    use crate::indicators::ticker_advanced::volume_24h_momentum::Volume24hMomentum;
    use crate::indicators::ticker_advanced::volume_24h_z_score::Volume24hZScore;
    use crate::indicators::volatility_advanced::hv_momentum::HvMomentum;
    use crate::indicators::volatility_advanced::vol_idx_momentum::VolIdxMomentum;

    fn assert_close(gpu: &[f32], cpu: &[f64]) {
        assert_eq!(gpu.len(), cpu.len());
        for (g, c) in gpu.iter().zip(cpu) {
            let delta = (*g as f64 - *c).abs();
            assert!(delta < 1e-4, "{g} vs {c} (delta {delta})");
        }
    }

    fn bars(n: usize) -> Vec<ResearchBar> {
        (0..n)
            .map(|i| {
                let close = 10.0 + (i as f64 * 0.35).sin() * 4.0 + ((i % 7) as f64) * 0.5;
                let open = close - 0.3 + ((i % 5) as f64) * 0.1;
                let high = close.max(open) + 0.4 + ((i % 3) as f64) * 0.2;
                let low = close.min(open) - 0.4 - ((i % 4) as f64) * 0.15;
                let volume = 100.0 + ((i % 9) as f64) * 17.0;
                ResearchBar::new(i as i64 * 60_000, open, high, low, close, volume)
            })
            .collect()
    }

    fn run(formula: CubeFormula, bars: &[ResearchBar], params: CubeParams) -> Vec<f32> {
        let samples: Vec<GpuSample> = bars.iter().map(GpuSample::from).collect();
        launch_cube(formula, &samples, params)
    }

    fn cpu(values: &[f64], mut step: impl FnMut(f64) -> f64) -> Vec<f64> {
        values.iter().copied().map(|v| step(v)).collect()
    }

    #[test]
    fn lane_matches_cpu() {
        let bars = bars(70);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let high: Vec<f64> = bars.iter().map(|b| b.high).collect();
        let p5 = CubeParams::period(5);

        let identity = run(CubeFormula::Identity, &bars, CubeParams::period(1));
        assert_close(&identity, &close);

        let mut vol_lane = CubeParams::period(1);
        vol_lane.lane = OhlcvField::Volume;
        let volumes: Vec<f64> = bars.iter().map(|b| b.volume).collect();
        assert_close(
            &run(CubeFormula::Identity, &bars, vol_lane),
            &volumes,
        );

        let mut sma = Sma::new(5);
        assert_close(
            &run(CubeFormula::WindowMean, &bars, p5),
            &cpu(&close, |v| sma.feed(v)),
        );

        let mut sma_hl2 = Sma::new(5);
        let hl2: Vec<f64> = bars.iter().map(|b| (b.high + b.low) / 2.0).collect();
        let mut hl2_params = CubeParams::period(5);
        hl2_params.lane = OhlcvField::HL2;
        assert_close(
            &run(CubeFormula::WindowMean, &bars, hl2_params),
            &cpu(&hl2, |v| sma_hl2.feed(v)),
        );

        let mut highest = Highest::new(5);
        let mut hi_params = CubeParams::period(5);
        hi_params.lane = OhlcvField::High;
        assert_close(
            &run(CubeFormula::WindowMax, &bars, hi_params),
            &cpu(&high, |v| highest.feed(v)),
        );

        let mut lowest = Lowest::new(5);
        assert_close(
            &run(CubeFormula::WindowMin, &bars, p5),
            &cpu(&close, |v| lowest.feed(v)),
        );

        let mut wma = Wma::new(4);
        assert_close(
            &run(CubeFormula::WindowWeighted, &bars, CubeParams::period(4)),
            &cpu(&close, |v| wma.feed(v)),
        );

        let mut ema = Ema::new(5);
        assert_close(
            &run(CubeFormula::Ema, &bars, p5),
            &cpu(&close, |v| ema.feed(v)),
        );
        let mut rma = Rma::new(5);
        assert_close(
            &run(CubeFormula::Rma, &bars, p5),
            &cpu(&close, |v| rma.feed(v)),
        );
        let mut dema = Dema::new(5);
        assert_close(
            &run(CubeFormula::Dema, &bars, p5),
            &cpu(&close, |v| dema.feed(v)),
        );
        let mut tema = Tema::new(5);
        assert_close(
            &run(CubeFormula::Tema, &bars, p5),
            &cpu(&close, |v| tema.feed(v)),
        );
        let mut tma = Tma::new(5);
        assert_close(
            &run(CubeFormula::Tma, &bars, p5),
            &cpu(&close, |v| tma.feed(v)),
        );
        let mut trima = Trima::new(5);
        assert_close(
            &run(CubeFormula::Tma, &bars, p5),
            &cpu(&close, |v| trima.feed(v)),
        );
        let mut hma = Hma::new(8);
        assert_close(
            &run(CubeFormula::Hma, &bars, CubeParams::period(8)),
            &cpu(&close, |v| hma.feed(v)),
        );

        let mut alma = Alma::new(5);
        assert_close(
            &run(CubeFormula::Alma, &bars, p5),
            &cpu(&close, |v| alma.feed(v)),
        );
        let mut alma_custom = Alma::with_params(5, 0.5, 3.0);
        let mut alma_params = CubeParams::period(5);
        alma_params.a = 0.5;
        alma_params.b = 3.0;
        assert_close(
            &run(CubeFormula::Alma, &bars, alma_params),
            &cpu(&close, |v| alma_custom.feed(v)),
        );

        let mut t3 = T3::new(5);
        let mut t3_params = CubeParams::period(5);
        t3_params.a = 0.7;
        assert_close(
            &run(CubeFormula::T3, &bars, t3_params),
            &cpu(&close, |v| t3.feed(v)),
        );
        let mut t3_custom = T3::with_alpha(5, 0.4);
        let mut t3_custom_params = CubeParams::period(5);
        t3_custom_params.a = 0.4;
        assert_close(
            &run(CubeFormula::T3, &bars, t3_custom_params),
            &cpu(&close, |v| t3_custom.feed(v)),
        );

        let mut md = McGinleyDynamic::new(5);
        assert_close(
            &run(CubeFormula::Mcginley, &bars, p5),
            &cpu(&close, |v| md.feed(v)),
        );

        let mut roc = Roc::new(5, false);
        assert_close(
            &run(CubeFormula::Roc, &bars, p5),
            &cpu(&close, |v| roc.feed(v)),
        );
        let mut roc_log = Roc::new(5, true);
        let mut roc_params = CubeParams::period(5);
        roc_params.flag = 1;
        assert_close(
            &run(CubeFormula::Roc, &bars, roc_params),
            &cpu(&close, |v| roc_log.feed(v)),
        );

        let mut rsi = Rsi::new(5);
        assert_close(
            &run(CubeFormula::Rsi, &bars, p5),
            &cpu(&close, |v| rsi.feed(v)),
        );
        let mut cmo = Cmo::new(5);
        assert_close(
            &run(CubeFormula::Cmo, &bars, p5),
            &cpu(&close, |v| cmo.feed(v)),
        );
        let mut bias = Bias::new(5);
        assert_close(
            &run(CubeFormula::Bias, &bars, p5),
            &cpu(&close, |v| bias.feed(v)),
        );

        let mut tr = TrueRange::new();
        let cpu_tr: Vec<f64> = bars
            .iter()
            .map(|b| tr.feed(&[b.high, b.low, b.close]))
            .collect();
        assert_close(
            &run(CubeFormula::TrueRange, &bars, CubeParams::period(1)),
            &cpu_tr,
        );

        let mut atr = Atr::new_wilder(5);
        let cpu_atr: Vec<f64> = bars
            .iter()
            .map(|b| atr.feed(&[b.high, b.low, b.close]))
            .collect();
        assert_close(&run(CubeFormula::Atr, &bars, p5), &cpu_atr);

        let mut bop = Bop::new();
        let cpu_bop: Vec<f64> = bars
            .iter()
            .map(|b| bop.feed(&[b.open, b.high, b.low, b.close]))
            .collect();
        assert_close(
            &run(CubeFormula::Bop, &bars, CubeParams::period(1)),
            &cpu_bop,
        );

        let mut vwma = Vwma::new(5);
        let cpu_vwma: Vec<f64> = bars
            .iter()
            .map(|b| vwma.feed(&[b.close, b.volume]))
            .collect();
        let mut vwma_params = CubeParams::period(5);
        vwma_params.lane2 = OhlcvField::Volume;
        assert_close(
            &run(CubeFormula::Vwma, &bars, vwma_params),
            &cpu_vwma,
        );

        let hold = [
            ResearchBar::new(0, 1.0, 2.0, 0.5, 1.5, 5.0),
            ResearchBar::new(1, 1.0, 2.0, 0.5, 3.0, 0.0),
            ResearchBar::new(2, 1.0, 2.0, 0.5, 4.0, 0.0),
        ];
        let mut vwma_hold = Vwma::new(1);
        let cpu_hold: Vec<f64> = hold
            .iter()
            .map(|b| vwma_hold.feed(&[b.close, b.volume]))
            .collect();
        let mut hold_params = CubeParams::period(1);
        hold_params.lane2 = OhlcvField::Volume;
        assert_close(
            &run(CubeFormula::Vwma, &hold, hold_params),
            &cpu_hold,
        );

        let mut macd = Macd::new(3, 8);
        let cpu_macd: Vec<f64> = bars
            .iter()
            .map(|b| macd.feed(&[b.close, b.open]))
            .collect();
        let mut macd_params = CubeParams::period(1);
        macd_params.fast = 3;
        macd_params.slow = 8;
        macd_params.lane2 = OhlcvField::Open;
        macd_params.signal = 3;
        assert_close(
            &run(CubeFormula::Macd, &bars, macd_params),
            &cpu_macd,
        );

        let mut apo = Apo::new(4, 9);
        assert_close(
            &run(
                CubeFormula::Apo,
                &bars,
                CubeParams {
                    fast: 4,
                    slow: 9,
                    lane: OhlcvField::High,
                    ..CubeParams::period(1)
                },
            ),
            &cpu(&high, |v| apo.feed(v)),
        );

        let books = [
            OrderBook::from_tuples(&[(100.0, 2.0), (99.0, 1.0)], &[(101.0, 3.0), (102.0, 4.0)], 0),
            OrderBook::from_tuples(&[(100.5, 1.0)], &[(101.5, 1.0)], 1),
            OrderBook::from_tuples(&[], &[(101.0, 1.0)], 2),
            OrderBook::from_tuples(
                &[(100.0, 5.0), (99.5, 5.0), (99.0, 5.0)],
                &[(101.0, 1.0), (102.0, 1.0), (103.0, 1.0)],
                3,
            ),
        ];
        let book_samples: Vec<GpuSample> = books.iter().map(GpuSample::from).collect();
        let mut micro = Microprice::new();
        let cpu_micro: Vec<f64> = books
            .iter()
            .map(|b| {
                micro.update_orderbook(b);
                micro.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::Microprice, &book_samples, CubeParams::period(1)),
            &cpu_micro,
        );
        let mut imb = BookImbalanceRatio::with_levels(2);
        let mut asym = BidAskAsymmetry::new(2);
        let cpu_imb: Vec<f64> = books
            .iter()
            .map(|b| {
                imb.update_orderbook(b);
                imb.value()
            })
            .collect();
        let cpu_asym: Vec<f64> = books
            .iter()
            .map(|b| {
                asym.update_orderbook(b);
                asym.value()
            })
            .collect();
        let mut imb_params = CubeParams::period(1);
        imb_params.levels = 2;
        assert_close(
            &launch_cube(CubeFormula::BookImbalance, &book_samples, imb_params),
            &cpu_imb,
        );
        assert_close(
            &launch_cube(CubeFormula::BookImbalance, &book_samples, imb_params),
            &cpu_asym,
        );
        let mut pressure = BookPressure::new(3, 1);
        let cpu_pressure: Vec<f64> = books
            .iter()
            .map(|b| {
                pressure.update_orderbook(b);
                pressure.value()
            })
            .collect();
        let mut pressure_params = CubeParams::period(3);
        pressure_params.levels = 1;
        assert_close(
            &launch_cube(CubeFormula::BookPressure, &book_samples, pressure_params),
            &cpu_pressure,
        );

        let mut rates = Vec::new();
        let funding: Vec<GpuSample> = (0..8)
            .map(|i| {
                let mut fr = FundingRate::default();
                fr.rate = 0.0001 * (i as f64 + 1.0);
                fr.mark_price = Some(100.0 + i as f64);
                rates.push(fr.rate);
                GpuSample::from(&fr)
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::Scalar, &funding, CubeParams::period(1)),
            &rates,
        );

        let mut xs = vec![10.0, 10.0, 10.0];
        for i in 0..12 {
            xs.push(10.0 + (i as f64 * 0.55).sin() * 4.0 + (i as f64) * 0.3);
        }
        let period = 5u32;
        let mut slot0 = CubeParams::period(period);
        slot0.slot = 0;
        let mut slot2 = CubeParams::period(period);
        slot2.slot = 2;
        let mut slot3 = CubeParams::period(period);
        slot3.slot = 3;

        let oi_samples: Vec<GpuSample> = xs
            .iter()
            .map(|v| {
                let mut oi = OpenInterest::default();
                oi.open_interest = *v;
                GpuSample::from(&oi)
            })
            .collect();
        let mut oi_m = OiMomentum::new(period as usize);
        let cpu_oi_m: Vec<f64> = xs
            .iter()
            .map(|v| {
                let mut oi = OpenInterest::default();
                oi.open_interest = *v;
                oi_m.update_oi(&oi);
                oi_m.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::EndpointSlope, &oi_samples, slot0),
            &cpu_oi_m,
        );
        let mut oi_z = OiZScore::new(period as usize);
        let cpu_oi_z: Vec<f64> = xs
            .iter()
            .map(|v| {
                let mut oi = OpenInterest::default();
                oi.open_interest = *v;
                oi_z.update_oi(&oi);
                oi_z.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::PopZScore, &oi_samples, slot0),
            &cpu_oi_z,
        );

        let mark_samples: Vec<GpuSample> = xs
            .iter()
            .map(|v| {
                let mut mp = MarkPrice::default();
                mp.mark_price = *v;
                GpuSample::from(&mp)
            })
            .collect();
        let mut mark_m = MarkPriceMomentum::new(period as usize);
        let cpu_mark: Vec<f64> = xs
            .iter()
            .map(|v| {
                let mut mp = MarkPrice::default();
                mp.mark_price = *v;
                mark_m.update_mark(&mp);
                mark_m.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::EndpointSlope, &mark_samples, slot0),
            &cpu_mark,
        );

        let basis_samples: Vec<GpuSample> = xs
            .iter()
            .map(|v| {
                let mut b = Basis::default();
                b.basis = *v;
                GpuSample::from(&b)
            })
            .collect();
        let mut basis_m = BasisMomentum::new(period as usize);
        let cpu_basis_m: Vec<f64> = xs
            .iter()
            .map(|v| {
                let mut b = Basis::default();
                b.basis = *v;
                basis_m.update_basis(&b);
                basis_m.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::EndpointSlope, &basis_samples, slot0),
            &cpu_basis_m,
        );
        let mut basis_z = BasisZScore::new(period as usize);
        let cpu_basis_z: Vec<f64> = xs
            .iter()
            .map(|v| {
                let mut b = Basis::default();
                b.basis = *v;
                basis_z.update_basis(&b);
                basis_z.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::PopZScore, &basis_samples, slot0),
            &cpu_basis_z,
        );

        let settle_samples: Vec<GpuSample> = xs
            .iter()
            .map(|v| {
                let s = FundingSettlement {
                    settled_rate: *v,
                    settlement_time: 0,
                    timestamp: 0,
                };
                GpuSample::from(&s)
            })
            .collect();
        let mut settled = SettledFundingMomentum::new(period as usize);
        let cpu_settled: Vec<f64> = xs
            .iter()
            .map(|v| {
                let s = FundingSettlement {
                    settled_rate: *v,
                    settlement_time: 0,
                    timestamp: 0,
                };
                settled.update_funding_settlement(&s);
                settled.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::EndpointSlope, &settle_samples, slot0),
            &cpu_settled,
        );

        let hv_samples: Vec<GpuSample> = xs
            .iter()
            .map(|v| {
                let hv = HistoricalVolatility { volatility: *v, timestamp: 0 };
                GpuSample::from(&hv)
            })
            .collect();
        let mut hv_m = HvMomentum::new(period as usize);
        let cpu_hv: Vec<f64> = xs
            .iter()
            .map(|v| {
                let hv = HistoricalVolatility { volatility: *v, timestamp: 0 };
                hv_m.update_historical_volatility(&hv);
                hv_m.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::EndpointSlope, &hv_samples, slot0),
            &cpu_hv,
        );

        let vi_samples: Vec<GpuSample> = xs
            .iter()
            .map(|v| {
                let mut vi = VolatilityIndex::default();
                vi.value = *v;
                GpuSample::from(&vi)
            })
            .collect();
        let mut vi_m = VolIdxMomentum::new(period as usize);
        let cpu_vi: Vec<f64> = xs
            .iter()
            .map(|v| {
                let mut vi = VolatilityIndex::default();
                vi.value = *v;
                vi_m.update_volatility_index(&vi);
                vi_m.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::EndpointSlope, &vi_samples, slot0),
            &cpu_vi,
        );

        let tickers: Vec<Ticker> = xs
            .iter()
            .map(|v| {
                let mut t = Ticker::default();
                t.last_price = 999.0;
                t.volume_24h = Some(*v);
                t
            })
            .collect();
        let ticker_samples: Vec<GpuSample> = tickers.iter().map(GpuSample::from).collect();
        let mut vol_m = Volume24hMomentum::new(period as usize);
        let cpu_vol_m: Vec<f64> = tickers
            .iter()
            .map(|t| {
                vol_m.update_ticker(t);
                vol_m.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::EndpointSlope, &ticker_samples, slot3),
            &cpu_vol_m,
        );
        let mut vol_z = Volume24hZScore::new(period as usize);
        let cpu_vol_z: Vec<f64> = tickers
            .iter()
            .map(|t| {
                vol_z.update_ticker(t);
                vol_z.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::PopZScore, &ticker_samples, slot3),
            &cpu_vol_z,
        );

        let greeks: Vec<OptionGreeks> = xs
            .iter()
            .map(|v| OptionGreeks {
                delta: *v,
                gamma: 0.0,
                vega: *v * 2.0 + 3.0,
                theta: 0.0,
                rho: 0.0,
                mark_iv: 0.0,
                bid_iv: None,
                ask_iv: None,
                timestamp: 0,
            })
            .collect();
        let greek_samples: Vec<GpuSample> = greeks.iter().map(GpuSample::from).collect();
        let mut delta_m = DeltaExposureFlow::new(period as usize);
        let cpu_delta: Vec<f64> = greeks
            .iter()
            .map(|g| {
                delta_m.update_option_greeks(g);
                delta_m.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::EndpointSlope, &greek_samples, slot0),
            &cpu_delta,
        );
        let mut vega_m = VegaExposureFlow::new(period as usize);
        let cpu_vega: Vec<f64> = greeks
            .iter()
            .map(|g| {
                vega_m.update_option_greeks(g);
                vega_m.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::EndpointSlope, &greek_samples, slot2),
            &cpu_vega,
        );

        let funds: Vec<InsuranceFund> = xs
            .iter()
            .map(|v| InsuranceFund { balance: *v, timestamp: 0 })
            .collect();
        let fund_samples: Vec<GpuSample> = funds.iter().map(GpuSample::from).collect();
        let mut deplete = FundDepletionRate::new(period as usize);
        let cpu_deplete: Vec<f64> = funds
            .iter()
            .map(|f| {
                deplete.update_insurance_fund(f);
                deplete.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::EndpointSlope, &fund_samples, slot0),
            &cpu_deplete,
        );

        let settlements: Vec<SettlementEvent> = xs
            .iter()
            .map(|v| {
                let mut s = SettlementEvent::default();
                s.settlement_price = *v;
                s
            })
            .collect();
        let settlement_samples: Vec<GpuSample> = settlements.iter().map(GpuSample::from).collect();
        let mut settle_m = SettlementPriceMomentum::new(period as usize);
        let cpu_settle_m: Vec<f64> = settlements
            .iter()
            .map(|s| {
                settle_m.update_settlement(s);
                settle_m.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::EndpointSlope, &settlement_samples, slot0),
            &cpu_settle_m,
        );

        let rate_values = [0.001_f64, -0.002, 0.0, 0.004, 0.0005];
        let rate_samples: Vec<GpuSample> = rate_values
            .iter()
            .map(|r| {
                let mut fr = FundingRate::default();
                fr.rate = *r;
                GpuSample::from(&fr)
            })
            .collect();
        for periods in [3.0_f32, 1.0_f32] {
            let mut annual = AnnualizedFundingRate::new(periods as f64);
            let cpu_annual: Vec<f64> = rate_values
                .iter()
                .map(|r| {
                    let mut fr = FundingRate::default();
                    fr.rate = *r;
                    annual.update_funding(&fr);
                    annual.value()
                })
                .collect();
            let mut scale = CubeParams::period(1);
            scale.a = periods * 365.0 * 100.0;
            assert_close(
                &launch_cube(CubeFormula::Scale, &rate_samples, scale),
                &cpu_annual,
            );
        }
        let mut fund_z = FundingZScore::new(period as usize);
        let fund_z_samples: Vec<GpuSample> = xs
            .iter()
            .map(|v| {
                let mut fr = FundingRate::default();
                fr.rate = (*v - 12.0) * 0.01;
                GpuSample::from(&fr)
            })
            .collect();
        let cpu_fund_z: Vec<f64> = xs
            .iter()
            .map(|v| {
                let mut fr = FundingRate::default();
                fr.rate = (*v - 12.0) * 0.01;
                fund_z.update_funding(&fr);
                fund_z.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::PopZScore, &fund_z_samples, slot0),
            &cpu_fund_z,
        );

        let mut ls = LongShortRatioMomentum::new(period as usize);
        let ls_samples: Vec<GpuSample> = xs
            .iter()
            .map(|v| {
                let mut r = LongShortRatio::default();
                r.long_ratio = *v;
                r.short_ratio = 0.5;
                GpuSample::from(&r)
            })
            .collect();
        let cpu_ls: Vec<f64> = xs
            .iter()
            .map(|v| {
                let mut r = LongShortRatio::default();
                r.long_ratio = *v;
                r.short_ratio = 0.5;
                ls.update_long_short_ratio(&r);
                ls.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::EndpointSlope, &ls_samples, slot0),
            &cpu_ls,
        );

        let mut mark_v = MarkPriceVolatility::new(period as usize);
        let cpu_mark_v: Vec<f64> = xs
            .iter()
            .map(|v| {
                let mut mp = MarkPrice::default();
                mp.mark_price = *v;
                mark_v.update_mark(&mp);
                mark_v.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::PopStd, &mark_samples, slot0),
            &cpu_mark_v,
        );

        let mut oi_p = OiPercentile::new(period as usize);
        let cpu_oi_p: Vec<f64> = xs
            .iter()
            .map(|v| {
                let mut oi = OpenInterest::default();
                oi.open_interest = *v;
                oi_p.update_oi(&oi);
                oi_p.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::PercentileRank, &oi_samples, slot0),
            &cpu_oi_p,
        );

        let mut fund_step = InsuranceFundMomentum::new(period as usize);
        let cpu_step: Vec<f64> = funds
            .iter()
            .map(|f| {
                fund_step.update_insurance_fund(f);
                fund_step.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::EmaStep, &fund_samples, slot0),
            &cpu_step,
        );

        let mut slot1 = CubeParams::period(period);
        slot1.slot = 1;
        let qty = [0.0_f64, 0.0, 4.0, 2.0, 6.0, 1.0, 3.0];
        let auctions: Vec<AuctionEvent> = qty
            .iter()
            .map(|v| AuctionEvent {
                auction_id: String::new(),
                indicative_price: 50.0,
                indicative_qty: *v,
                state: String::new(),
                timestamp: 0,
            })
            .collect();
        let auction_samples: Vec<GpuSample> = auctions.iter().map(GpuSample::from).collect();
        let mut auction = AuctionImbalance::new(period as usize);
        let cpu_auction: Vec<f64> = auctions
            .iter()
            .map(|a| {
                auction.update_auction(a);
                auction.value()
            })
            .collect();
        assert_close(
            &launch_cube(CubeFormula::RatioToMean, &auction_samples, slot1),
            &cpu_auction,
        );

        assert!(run(CubeFormula::WindowMean, &[], CubeParams::period(5)).is_empty());
    }
}
