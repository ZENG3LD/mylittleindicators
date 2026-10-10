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
use super::gpu_sample::{GpuSample, GpuTimes};
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

/// OLS slope of cumulative size against absolute distance from `mid`.
/// Non-negative. Zero when the side has fewer than two levels.
#[cube]
fn side_slope(
    px: &[f32],
    sz: &[f32],
    nside: &[f32],
    i: usize,
    depth: usize,
    levels: usize,
    mid: f32,
) -> f32 {
    let mut n = nside[i] as usize;
    if n > depth {
        n = depth;
    }
    if n > levels {
        n = levels;
    }
    let mut slope = 0.0f32;
    if n >= 2 {
        let base = i * depth;
        let mut cum = 0.0f32;
        let mut sum_x = 0.0f32;
        let mut sum_y = 0.0f32;
        let mut sum_xy = 0.0f32;
        let mut sum_x2 = 0.0f32;
        for k in 0..n {
            cum = cum + sz[base + k];
            let mut y = px[base + k] - mid;
            if y < 0.0f32 {
                y = -y;
            }
            sum_x = sum_x + cum;
            sum_y = sum_y + y;
            sum_xy = sum_xy + cum * y;
            sum_x2 = sum_x2 + cum * cum;
        }
        let nf = n as f32;
        let denom = nf * sum_x2 - sum_x * sum_x;
        let mut ad = denom;
        if ad < 0.0f32 {
            ad = -ad;
        }
        if ad >= 1.0e-12 {
            slope = (nf * sum_xy - sum_x * sum_y) / denom;
            if slope < 0.0f32 {
                slope = 0.0f32;
            }
        }
    }
    slope
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
    } else if formula == 35u32 {
        let d = depth as usize;
        let mut lv = levels as usize;
        if lv < 2 {
            lv = 2;
        }
        let mut y = 0.0f32;
        for i in 0..n {
            let bn = bid_n[i] as usize;
            let an = ask_n[i] as usize;
            if bn >= 1 && an >= 1 {
                let base = i * d;
                let mid = (bid_px[base] + ask_px[base]) / 2.0f32;
                let bid = side_slope(bid_px, bid_sz, bid_n, i, d, lv, mid);
                let ask = side_slope(ask_px, ask_sz, ask_n, i, d, lv, mid);
                y = (bid + ask) / 2.0f32;
            }
            output[i] = y;
        }
    }
}

/// Bar formulas 36 through 44. A second entry keeps the shader that already
/// launched at the size that fit the compiler stack. One unit writes the series.
#[cube]
fn bar_scan(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    a: f32,
    formula: u32,
) {
    let n = open.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    if formula == 36u32 {
        for i in 0..n {
            if i + 1 < p {
                output[i] = -50.0f32;
            } else {
                let start = i + 1 - p;
                let mut hh = high[start];
                let mut ll = low[start];
                for k in 0..p {
                    if high[start + k] > hh {
                        hh = high[start + k];
                    }
                    if low[start + k] < ll {
                        ll = low[start + k];
                    }
                }
                let range = hh - ll;
                let mut ar = range;
                if ar < 0.0f32 {
                    ar = -ar;
                }
                if ar < 1.0e-12 {
                    output[i] = -50.0f32;
                } else {
                    let mut v = ((hh - close[i]) / range) * -100.0f32;
                    if v < -100.0f32 {
                        v = -100.0f32;
                    }
                    if v > 0.0f32 {
                        v = 0.0f32;
                    }
                    output[i] = v;
                }
            }
        }
    } else if formula == 37u32 {
        let mut y = 0.0f32;
        let mut prev = fld(open, high, low, close, volume, 0, lane);
        output[0] = 0.0f32;
        for k in 1..n {
            let x = fld(open, high, low, close, volume, k, lane);
            if x > prev {
                y = y + volume[k];
            } else if x < prev {
                y = y - volume[k];
            }
            prev = x;
            output[k] = y;
        }
    } else if formula == 38u32 {
        let mut y = 0.0f32;
        let mut prev = fld(open, high, low, close, volume, 0, lane);
        output[0] = 0.0f32;
        for k in 1..n {
            let x = fld(open, high, low, close, volume, k, lane);
            let mut ap = prev;
            if ap < 0.0f32 {
                ap = -ap;
            }
            let mut pct = 0.0f32;
            if ap > 1.0e-12 {
                pct = (x - prev) / prev;
            }
            y = y + pct * volume[k];
            prev = x;
            output[k] = y;
        }
    } else if formula == 39u32 {
        for i in 0..n {
            if i + 1 < p {
                output[i] = 50.0f32;
            } else {
                let start = i + 1 - p;
                let mut pos = 0.0f32;
                let mut neg = 0.0f32;
                for k in 0..p {
                    let j = start + k;
                    let tp = (high[j] + low[j] + close[j]) / 3.0f32;
                    let raw = tp * volume[j];
                    let mut up = 0u32;
                    if j == 0 {
                        up = 1u32;
                    } else {
                        let prev = (high[j - 1] + low[j - 1] + close[j - 1]) / 3.0f32;
                        if tp > prev {
                            up = 1u32;
                        }
                    }
                    if up == 1u32 {
                        pos = pos + raw;
                    } else {
                        neg = neg + raw;
                    }
                }
                if neg == 0.0f32 {
                    output[i] = 100.0f32;
                } else if pos == 0.0f32 {
                    output[i] = 0.0f32;
                } else {
                    let ratio = pos / neg;
                    output[i] = 100.0f32 - (100.0f32 / (1.0f32 + ratio));
                }
            }
        }
    } else if formula == 40u32 {
        let mut y = 0.0f32;
        for i in 0..n {
            let range = high[i] - low[i];
            let mut ar = range;
            if ar < 0.0f32 {
                ar = -ar;
            }
            let mut mfm = 0.0f32;
            if ar >= 1.0e-12 {
                mfm = ((close[i] - low[i]) - (high[i] - close[i])) / range;
            }
            y = y + mfm * volume[i];
            output[i] = y;
        }
    } else if formula == 41u32 {
        output[0] = 0.0f32;
        let mut prev_h = high[0];
        let mut prev_l = low[0];
        let mut sum_up = 0.0f32;
        let mut sum_down = 0.0f32;
        let pf = p as f32;
        for k in 1..n {
            let mut up = high[k] - prev_h;
            if up < 0.0f32 {
                up = 0.0f32;
            }
            let mut down = prev_l - low[k];
            if down < 0.0f32 {
                down = 0.0f32;
            }
            sum_up = (sum_up * (pf - 1.0f32) + up) / pf;
            sum_down = (sum_down * (pf - 1.0f32) + down) / pf;
            let denom = sum_up + sum_down;
            if denom > 0.0f32 {
                output[k] = sum_up / denom;
            } else {
                output[k] = 0.0f32;
            }
            prev_h = high[k];
            prev_l = low[k];
        }
    } else if formula == 42u32 {
        let twice = p + p;
        for i in 0..n {
            if i + 2 < twice {
                output[i] = 0.0f32;
            } else {
                let mut sum = 0.0f32;
                for t in 0..p {
                    let j = i - p + 1 + t;
                    let hs = j + 1 - p;
                    let mut hh = fld(open, high, low, close, volume, hs, lane);
                    for u in 0..p {
                        let v = fld(open, high, low, close, volume, hs + u, lane);
                        if v > hh {
                            hh = v;
                        }
                    }
                    let cur = fld(open, high, low, close, volume, j, lane);
                    let mut r = 0.0f32;
                    if hh > 1.0e-12 {
                        r = (cur - hh) / hh * 100.0f32;
                    }
                    sum = sum + r * r;
                }
                output[i] = sqrt_f(sum / (p as f32));
            }
        }
    } else if formula == 43u32 {
        let mut win = p;
        if win < 1 {
            win = 1;
        }
        output[0] = 0.0f32;
        for i in 1..n {
            let mut use_n = i;
            if use_n > win {
                use_n = win;
            }
            let mut sum = 0.0f32;
            for t in 0..use_n {
                let j = i - use_n + 1 + t;
                let mut pv = fld(open, high, low, close, volume, j - 1, lane);
                if pv < 1.0e-12 {
                    pv = 1.0e-12;
                }
                let cx = fld(open, high, low, close, volume, j, lane);
                let r = (cx / pv).ln();
                sum = sum + r * r;
            }
            let vol = sqrt_f(sum / (use_n as f32));
            if a > 0.0f32 {
                output[i] = vol * a;
            } else {
                output[i] = vol;
            }
        }
    } else if formula == 44u32 {
        let mut win = p;
        if win < 2 {
            win = 2;
        }
        for i in 0..n {
            if i + 1 < win {
                output[i] = 0.0f32;
            } else {
                let start = i + 1 - win;
                let newest = fld(open, high, low, close, volume, i, lane);
                let oldest = fld(open, high, low, close, volume, start, lane);
                let mut dir = newest - oldest;
                if dir < 0.0f32 {
                    dir = -dir;
                }
                let mut vol = 0.0f32;
                for k in 0..win - 1 {
                    let mut d = fld(open, high, low, close, volume, start + k + 1, lane)
                        - fld(open, high, low, close, volume, start + k, lane);
                    if d < 0.0f32 {
                        d = -d;
                    }
                    vol = vol + d;
                }
                if vol > 0.0f32 {
                    let mut er = dir / vol;
                    if er < 0.0f32 {
                        er = 0.0f32;
                    }
                    if er > 1.0f32 {
                        er = 1.0f32;
                    }
                    output[i] = er;
                } else {
                    output[i] = 0.0f32;
                }
            }
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    a: f32,
    formula: u32,
) {
    bar_scan(open, high, low, close, volume, output, lane, period, a, formula);
}

/// Bar formulas 45 through 53. A third entry, same reason as `bar_scan`.
#[cube]
fn bar_scan_b(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    formula: u32,
) {
    let n = open.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    if formula == 45u32 {
        let mut win = p;
        if win < 2 {
            win = 2;
        }
        let denom = (win as f32).ln();
        for i in 0..n {
            let mut range = high[i] - low[i];
            if range < 1.0e-9 {
                range = 1.0e-9;
            }
            output[i] = range.ln() / denom;
        }
    } else if formula == 46u32 {
        let mut lb = p;
        if lb < 2 {
            lb = 2;
        }
        if lb > 1024 {
            lb = 1024;
        }
        let mut y = 0.0f32;
        for i in 0..n {
            if i + 1 >= lb {
                let start = i + 1 - lb;
                let mut hh = close[start];
                for k in 0..lb {
                    if close[start + k] > hh {
                        hh = close[start + k];
                    }
                }
                let mut ah = hh;
                if ah < 0.0f32 {
                    ah = -ah;
                }
                if ah > 1.0e-12 {
                    y = (hh - low[i]) / hh * 100.0f32;
                }
            }
            output[i] = y;
        }
    } else if formula == 47u32 {
        output[0] = 0.0f32;
        for i in 1..n {
            let mut use_n = i;
            if use_n > p {
                use_n = p;
            }
            let mut sum = 0.0f32;
            for t in 0..use_n {
                let j = i - use_n + 1 + t;
                let mut pv = fld(open, high, low, close, volume, j - 1, lane);
                if pv < 1.0e-12 {
                    pv = 1.0e-12;
                }
                let cx = fld(open, high, low, close, volume, j, lane);
                let r = (cx / pv).ln();
                let r2 = r * r;
                sum = sum + r2 * r2;
            }
            output[i] = (sum / (use_n as f32)) * 1_000_000.0f32;
        }
    } else if formula == 48u32 {
        let mut win = p;
        if win < 5 {
            win = 5;
        }
        if win > 1024 {
            win = 1024;
        }
        let ann = sqrt_f(252.0f32);
        for i in 0..n {
            if i < win {
                output[i] = 0.0f32;
            } else {
                let start = i - win + 1;
                let mut sum = 0.0f32;
                for t in 0..win {
                    let j = start + t;
                    let r = (fld(open, high, low, close, volume, j, lane)
                        / fld(open, high, low, close, volume, j - 1, lane))
                        .ln();
                    sum = sum + r;
                }
                let mean = sum / (win as f32);
                let mut var = 0.0f32;
                for t in 0..win {
                    let j = start + t;
                    let r = (fld(open, high, low, close, volume, j, lane)
                        / fld(open, high, low, close, volume, j - 1, lane))
                        .ln();
                    let d = r - mean;
                    var = var + d * d;
                }
                output[i] = sqrt_f(var / (win as f32)) * ann;
            }
        }
    } else if formula == 49u32 {
        for i in 0..n {
            if i + 1 < p {
                output[i] = 0.0f32;
            } else {
                let mut start = i + 1 - p;
                if start < 1 {
                    start = 1;
                }
                let mut s = 0.0f32;
                for j in start..i + 1 {
                    if fld(open, high, low, close, volume, j, lane)
                        > fld(open, high, low, close, volume, j - 1, lane)
                    {
                        s = s + 1.0f32;
                    }
                }
                output[i] = 100.0f32 * s / (p as f32);
            }
        }
    } else if formula == 50u32 {
        for i in 0..n {
            let mut start = 0;
            if i + 1 > p {
                start = i + 1 - p;
            }
            let mut su = 0.0f32;
            let mut sd = 0.0f32;
            for j in start..i + 1 {
                let mut range = high[j] - low[j];
                if range < 0.0f32 {
                    range = -range;
                }
                if range < 1.0e-12 {
                    range = 1.0e-12;
                }
                let delta = close[j] - open[j];
                if delta > 0.0f32 {
                    su = su + delta / range;
                } else if delta < 0.0f32 {
                    sd = sd + (-delta) / range;
                }
            }
            let mut denom = su + sd;
            if denom < 1.0e-12 {
                denom = 1.0e-12;
            }
            output[i] = 100.0f32 * su / denom;
        }
    } else if formula == 51u32 {
        let mut win = p;
        if win < 2 {
            win = 2;
        }
        if win > 1024 {
            win = 1024;
        }
        for i in 0..n {
            let mut start = 0;
            if i + 1 > win {
                start = i + 1 - win;
            }
            let mut sp = 0.0f32;
            let mut sn = 0.0f32;
            for j in start..i + 1 {
                let mut diff = 0.0f32;
                if j > 0 {
                    diff = fld(open, high, low, close, volume, j, lane)
                        - fld(open, high, low, close, volume, j - 1, lane);
                }
                if diff > 0.0f32 {
                    sp = sp + diff;
                } else {
                    sn = sn + (-diff);
                }
            }
            let mut denom = sp + sn;
            if denom < 0.0f32 {
                denom = -denom;
            }
            if denom < 1.0e-9 {
                denom = 1.0e-9;
            }
            output[i] = 100.0f32 * (sp - sn) / denom;
        }
    } else if formula == 52u32 {
        let mut win = p;
        if win < 2 {
            win = 2;
        }
        if win > 512 {
            win = 512;
        }
        for i in 0..n {
            if i + 1 < win {
                output[i] = 0.0f32;
            } else {
                let start = i + 1 - win;
                let mut num = 0.0f32;
                let mut den = 0.0f32;
                for k in 0..win {
                    let price = fld(open, high, low, close, volume, start + k, lane);
                    let w = (k + 1) as f32;
                    num = num + w * price;
                    den = den + price;
                }
                let mut ad = den;
                if ad < 0.0f32 {
                    ad = -ad;
                }
                if ad > 1.0e-12 {
                    output[i] = -(num / den) + ((win as f32) + 1.0f32) / 2.0f32;
                } else {
                    output[i] = 0.0f32;
                }
            }
        }
    } else if formula == 53u32 {
        let mut win = p;
        if win < 2 {
            win = 2;
        }
        let pi = 3.14159274f32;
        output[0] = 0.0f32;
        for i in 1..n {
            let k = i;
            let mut len = k;
            if len > win {
                len = win;
            }
            let mut newest = k - 1;
            for _step in 0..k {
                if newest >= win {
                    newest = newest - win;
                }
            }
            let mut s = 0.0f32;
            for t in 1..len {
                let mut age_hi = newest + win - t;
                if t <= newest {
                    age_hi = newest - t;
                }
                let mut age_lo = newest + win - (t - 1);
                if t - 1 <= newest {
                    age_lo = newest - (t - 1);
                }
                let r_hi = k - age_hi;
                let r_lo = k - age_lo;
                let mut pv_hi = fld(open, high, low, close, volume, r_hi - 1, lane);
                if pv_hi < 1.0e-12 {
                    pv_hi = 1.0e-12;
                }
                let mut ar_hi = (fld(open, high, low, close, volume, r_hi, lane) / pv_hi).ln();
                if ar_hi < 0.0f32 {
                    ar_hi = -ar_hi;
                }
                let mut pv_lo = fld(open, high, low, close, volume, r_lo - 1, lane);
                if pv_lo < 1.0e-12 {
                    pv_lo = 1.0e-12;
                }
                let mut ar_lo = (fld(open, high, low, close, volume, r_lo, lane) / pv_lo).ln();
                if ar_lo < 0.0f32 {
                    ar_lo = -ar_lo;
                }
                s = s + ar_hi * ar_lo;
            }
            let mut denom = (len as f32) - 1.0f32;
            if denom < 1.0f32 {
                denom = 1.0f32;
            }
            output[i] = pi * 0.5f32 * (s / denom) * 252.0f32 * 10000.0f32;
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map_b(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    formula: u32,
) {
    bar_scan_b(
        open, high, low, close, volume, output, lane, period, formula,
    );
}

/// Bar formulas 54 through 59. A fourth entry, same reason as `bar_scan`.
#[cube]
fn bar_scan_c(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    fast: u32,
    a: f32,
    formula: u32,
) {
    let n = open.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    if formula == 54u32 {
        for i in 0..n {
            if i + 1 < p {
                output[i] = 0.0f32;
            } else {
                let start = i + 1 - p;
                let mut lo = fld(open, high, low, close, volume, start, lane);
                let mut hi = lo;
                for k in 0..p {
                    let v = fld(open, high, low, close, volume, start + k, lane);
                    if v < lo {
                        lo = v;
                    }
                    if v > hi {
                        hi = v;
                    }
                }
                let mut path = 0.0f32;
                for k in 1..p {
                    let mut d = fld(open, high, low, close, volume, start + k, lane)
                        - fld(open, high, low, close, volume, start + k - 1, lane);
                    if d < 0.0f32 {
                        d = -d;
                    }
                    path = path + d;
                }
                if path < 1.0e-12 {
                    output[i] = 0.0f32;
                } else {
                    output[i] = (hi - lo) / path;
                }
            }
        }
    } else if formula == 55u32 {
        let mut win = p;
        if win < 5 {
            win = 5;
        }
        if win > 1024 {
            win = 1024;
        }
        for i in 0..n {
            if i + 1 < win {
                output[i] = 0.0f32;
            } else {
                let start = i + 1 - win;
                let newest = fld(open, high, low, close, volume, i, lane);
                let oldest = fld(open, high, low, close, volume, start, lane);
                let mut span = newest - oldest;
                if span < 0.0f32 {
                    span = -span;
                }
                let mut path = 0.0f32;
                for k in 1..win {
                    let mut d = fld(open, high, low, close, volume, start + k, lane)
                        - fld(open, high, low, close, volume, start + k - 1, lane);
                    if d < 0.0f32 {
                        d = -d;
                    }
                    path = path + d;
                }
                let mut eff = 0.0f32;
                if path > 1.0e-12 {
                    eff = span / path;
                }
                if newest >= oldest {
                    output[i] = 100.0f32 * eff;
                } else {
                    output[i] = -100.0f32 * eff;
                }
            }
        }
    } else if formula == 56u32 {
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
                let mut sumsq = 0.0f32;
                for k in 0..cnt {
                    let v = volume[start + k];
                    sum = sum + v;
                    sumsq = sumsq + v * v;
                }
                let mean = sum / (cnt as f32);
                let var = sumsq / (cnt as f32) - mean * mean;
                if var > 0.0f32 {
                    let std = sqrt_f(var);
                    if std > 1.0e-12 {
                        output[i] = (volume[i] - mean) / std;
                    } else {
                        output[i] = 0.0f32;
                    }
                } else {
                    output[i] = 0.0f32;
                }
            }
        }
    } else if formula == 57u32 {
        let mut win = fast as usize;
        if win < 2 {
            win = 2;
        }
        for i in 0..n {
            if i < p {
                output[i] = 0.0f32;
            } else {
                let count = i - p + 1;
                let mut use_n = count;
                if use_n > win {
                    use_n = win;
                }
                if use_n < 2 {
                    output[i] = 0.0f32;
                } else {
                    let start = i + 1 - use_n;
                    let mut sum = 0.0f32;
                    let mut sumsq = 0.0f32;
                    for k in 0..use_n {
                        let j = start + k;
                        let diff = fld(open, high, low, close, volume, j, lane)
                            - fld(open, high, low, close, volume, j - p, lane);
                        sum = sum + diff;
                        sumsq = sumsq + diff * diff;
                    }
                    let cur = fld(open, high, low, close, volume, i, lane)
                        - fld(open, high, low, close, volume, i - p, lane);
                    let mean = sum / (use_n as f32);
                    let var = sumsq / (use_n as f32) - mean * mean;
                    if var > 0.0f32 {
                        let std = sqrt_f(var);
                        if std > 1.0e-12 {
                            output[i] = (cur - mean) / std;
                        } else {
                            output[i] = 0.0f32;
                        }
                    } else {
                        output[i] = 0.0f32;
                    }
                }
            }
        }
    } else if formula == 58u32 {
        let mut win = p;
        if win < 2 {
            win = 2;
        }
        let mut kmult = a;
        if kmult < 0.1f32 {
            kmult = 0.1f32;
        }
        for i in 0..n {
            if i + 1 < win {
                output[i] = 0.5f32;
            } else {
                let start = i + 1 - win;
                let mut sum = 0.0f32;
                for k in 0..win {
                    sum = sum + fld(open, high, low, close, volume, start + k, lane);
                }
                let mean = sum / (win as f32);
                let mut var = 0.0f32;
                for k in 0..win {
                    let d = fld(open, high, low, close, volume, start + k, lane) - mean;
                    var = var + d * d;
                }
                let std = sqrt_f(var / (win as f32));
                let width = 2.0f32 * kmult * std;
                if width > 0.0f32 {
                    let lower = mean - kmult * std;
                    let price = fld(open, high, low, close, volume, i, lane);
                    output[i] = (price - lower) / width;
                } else {
                    output[i] = 0.5f32;
                }
            }
        }
    } else if formula == 59u32 {
        let mut win = p;
        if win < 2 {
            win = 2;
        }
        for i in 0..n {
            if i + 1 < win {
                output[i] = 0.0f32;
            } else {
                let start = i + 1 - win;
                let nf = win as f32;
                let x_sum = 0.5f32 * nf * (nf + 1.0f32);
                let x_mul = x_sum * (2.0f32 * nf + 1.0f32) / 3.0f32;
                let divisor = nf * x_mul - x_sum * x_sum;
                let mut y_sum = 0.0f32;
                let mut sum_xy = 0.0f32;
                for k in 0..win {
                    let y = fld(open, high, low, close, volume, start + k, lane);
                    let x = (k + 1) as f32;
                    y_sum = y_sum + y;
                    sum_xy = sum_xy + x * y;
                }
                let slope = (nf * sum_xy - x_sum * y_sum) / divisor;
                let intercept = (y_sum * x_mul - x_sum * sum_xy) / divisor;
                let tsf = slope * nf + intercept;
                let price = fld(open, high, low, close, volume, i, lane);
                let mut ap = price;
                if ap < 0.0f32 {
                    ap = -ap;
                }
                if ap > 1.0e-12 {
                    output[i] = 100.0f32 * (price - tsf) / price;
                } else {
                    output[i] = 0.0f32;
                }
            }
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map_c(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    fast: u32,
    a: f32,
    formula: u32,
) {
    bar_scan_c(
        open, high, low, close, volume, output, lane, period, fast, a, formula,
    );
}

/// Value at `rank` (0-based) in the lane window that starts at `start`.
#[cube]
fn window_rank(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    start: usize,
    win: usize,
    rank: usize,
    lane: u32,
) -> f32 {
    let mut chosen = fld(open, high, low, close, volume, start, lane);
    for a in 0..win {
        let v = fld(open, high, low, close, volume, start + a, lane);
        let mut less = 0usize;
        let mut equal = 0usize;
        for b in 0..win {
            let u = fld(open, high, low, close, volume, start + b, lane);
            if u < v {
                less = less + 1;
            } else if u == v {
                equal = equal + 1;
            }
        }
        if less <= rank {
            if less + equal > rank {
                chosen = v;
            }
        }
    }
    chosen
}

/// Median of absolute deviations from `med`, at `rank`.
#[cube]
fn window_dev_rank(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    start: usize,
    win: usize,
    rank: usize,
    lane: u32,
    med: f32,
) -> f32 {
    let mut first = fld(open, high, low, close, volume, start, lane) - med;
    if first < 0.0f32 {
        first = -first;
    }
    let mut chosen = first;
    for a in 0..win {
        let mut v = fld(open, high, low, close, volume, start + a, lane) - med;
        if v < 0.0f32 {
            v = -v;
        }
        let mut less = 0usize;
        let mut equal = 0usize;
        for b in 0..win {
            let mut u = fld(open, high, low, close, volume, start + b, lane) - med;
            if u < 0.0f32 {
                u = -u;
            }
            if u < v {
                less = less + 1;
            } else if u == v {
                equal = equal + 1;
            }
        }
        if less <= rank {
            if less + equal > rank {
                chosen = v;
            }
        }
    }
    chosen
}

/// Bar formulas 60 through 62.
#[cube]
fn bar_scan_d(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    formula: u32,
) {
    let n = open.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    if formula == 60u32 {
        for i in 0..n {
            let mut cnt = i + 1;
            if cnt > p {
                cnt = p;
            }
            let start = i + 1 - cnt;
            let mut sum = 0.0f32;
            for k in 0..cnt {
                sum = sum + 0.5f32 * (high[start + k] + low[start + k]);
            }
            output[i] = sum / (cnt as f32);
        }
    } else if formula == 61u32 {
        output[0] = 0.0f32;
        let mut prev = close[0];
        let mut y = 0.0f32;
        for k in 1..n {
            let mut trh = high[k];
            if prev > trh {
                trh = prev;
            }
            let mut trl = low[k];
            if prev < trl {
                trl = prev;
            }
            let mut ad = 0.0f32;
            if close[k] > prev {
                ad = close[k] - trl;
            } else if close[k] < prev {
                ad = close[k] - trh;
            }
            y = y + ad;
            prev = close[k];
            output[k] = y;
        }
    } else if formula == 62u32 {
        let mut win = p;
        if win < 3 {
            win = 3;
        }
        for i in 0..n {
            if i + 1 < win {
                output[i] = 0.0f32;
            } else {
                let start = i + 1 - win;
                let mut half = 0usize;
                let mut left = win;
                for _step in 0..win {
                    if left >= 2 {
                        left = left - 2;
                        half = half + 1;
                    }
                }
                let med_hi = window_rank(
                    open, high, low, close, volume, start, win, half, lane,
                );
                let mut med = med_hi;
                if left == 0 {
                    let med_lo = window_rank(
                        open, high, low, close, volume, start, win, half - 1, lane,
                    );
                    med = 0.5f32 * (med_lo + med_hi);
                }
                let mad_hi = window_dev_rank(
                    open, high, low, close, volume, start, win, half, lane, med,
                );
                let mut mad = mad_hi;
                if left == 0 {
                    let mad_lo = window_dev_rank(
                        open, high, low, close, volume, start, win, half - 1, lane, med,
                    );
                    mad = 0.5f32 * (mad_lo + mad_hi);
                }
                let mut denom = mad * 1.4826f32;
                if denom < 1.0e-12 {
                    denom = 1.0e-12;
                }
                let price = fld(open, high, low, close, volume, i, lane);
                output[i] = (price - med) / denom;
            }
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map_d(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    formula: u32,
) {
    bar_scan_d(open, high, low, close, volume, output, lane, period, formula);
}

/// True range. Bar 0 is `high - low`. Later bars take the classic three-way max.
#[cube]
fn wilder_tr(high: &[f32], low: &[f32], close: &[f32], i: usize) -> f32 {
    let mut v = high[i] - low[i];
    if i > 0 {
        let mut hc = high[i] - close[i - 1];
        if hc < 0.0f32 {
            hc = -hc;
        }
        let mut lc = low[i] - close[i - 1];
        if lc < 0.0f32 {
            lc = -lc;
        }
        if hc > v {
            v = hc;
        }
        if lc > v {
            v = lc;
        }
    }
    v
}

/// The `ordinal`-th valid log return on `[1, end]`, oldest first.
#[cube]
fn valid_log_return(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    end: usize,
    ordinal: usize,
    lane: u32,
) -> f32 {
    let mut prev = fld(open, high, low, close, volume, 0, lane);
    let mut seen = 0usize;
    let mut val = 0.0f32;
    let n = open.len();
    for j in 1..n {
        if j <= end {
            let cur = fld(open, high, low, close, volume, j, lane);
            if prev > 0.0f32 {
                if cur > 0.0f32 {
                    if seen == ordinal {
                        val = (cur / prev).ln();
                    }
                    seen = seen + 1;
                }
            }
            prev = cur;
        }
    }
    val
}

/// How many valid log returns sit on `[1, end]`.
#[cube]
fn count_log_returns(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    end: usize,
    lane: u32,
) -> usize {
    let mut prev = fld(open, high, low, close, volume, 0, lane);
    let mut seen = 0usize;
    let n = open.len();
    for j in 1..n {
        if j <= end {
            let cur = fld(open, high, low, close, volume, j, lane);
            if prev > 0.0f32 {
                if cur > 0.0f32 {
                    seen = seen + 1;
                }
            }
            prev = cur;
        }
    }
    seen
}

/// Order statistic `rank` among the `win` log returns that start at `base`.
#[cube]
fn log_return_rank(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    end: usize,
    base: usize,
    win: usize,
    rank: usize,
    lane: u32,
) -> f32 {
    let mut chosen = valid_log_return(open, high, low, close, volume, end, base, lane);
    for a in 0..win {
        let v = valid_log_return(open, high, low, close, volume, end, base + a, lane);
        let mut less = 0usize;
        let mut equal = 0usize;
        for b in 0..win {
            let u = valid_log_return(open, high, low, close, volume, end, base + b, lane);
            if u < v {
                less = less + 1;
            } else if u == v {
                equal = equal + 1;
            }
        }
        if less <= rank {
            if less + equal > rank {
                chosen = v;
            }
        }
    }
    chosen
}

/// Bar formulas 63 through 66.
#[cube]
fn bar_scan_e(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    formula: u32,
) {
    let n = open.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    if formula == 63u32 {
        for i in 0..n {
            if i + 1 < p {
                output[i] = 0.0f32;
            } else {
                let start = i + 1 - p;
                let mut sm = 0.0f32;
                let mut sv = 0.0f32;
                for k in 0..p {
                    let h = high[start + k];
                    let l = low[start + k];
                    let c = close[start + k];
                    let vol = volume[start + k];
                    let mut range = h - l;
                    if range < 0.0f32 {
                        range = -range;
                    }
                    let mut mfm = 0.0f32;
                    if range >= 1.0e-12 {
                        mfm = ((c - l) - (h - c)) / (h - l);
                    }
                    sm = sm + mfm * vol;
                    sv = sv + vol;
                }
                let mut av = sv;
                if av < 0.0f32 {
                    av = -av;
                }
                if av < 1.0e-12 {
                    output[i] = 0.0f32;
                } else {
                    output[i] = sm / sv;
                }
            }
        }
    } else if formula == 64u32 {
        let mut y = 0.0f32;
        for i in 0..n {
            let mut cnt = i + 1;
            if cnt > p {
                cnt = p;
            }
            let start = i + 1 - cnt;
            let mut sp = 0.0f32;
            let mut sv = 0.0f32;
            for k in 0..cnt {
                let typical = (high[start + k] + low[start + k] + close[start + k]) / 3.0f32;
                let vol = volume[start + k];
                sp = sp + typical * vol;
                sv = sv + vol;
            }
            if sv > 0.0f32 {
                y = sp / sv;
            }
            output[i] = y;
        }
    } else if formula == 65u32 {
        if n > 0 {
            output[0] = 0.0f32;
            let mut prev = fld(open, high, low, close, volume, 0, lane);
            let mut s1 = 0.5f32;
            let mut s2 = 0.5f32;
            let mut s3 = 0.5f32;
            let mut avg_g = 0.0f32;
            let mut avg_l = 0.0f32;
            let alpha = 1.0f32 / (p as f32);
            for i in 1..n {
                let px = fld(open, high, low, close, volume, i, lane);
                let diff = px - prev;
                prev = px;
                let mut gain = 0.0f32;
                let mut loss = 0.0f32;
                if diff > 0.0f32 {
                    gain = diff;
                }
                if diff < 0.0f32 {
                    loss = -diff;
                }
                let count = i + 1;
                if count <= p {
                    avg_g = avg_g + gain;
                    avg_l = avg_l + loss;
                    if count == p {
                        avg_g = avg_g / (p as f32);
                        avg_l = avg_l / (p as f32);
                    }
                } else {
                    avg_g = avg_g + alpha * (gain - avg_g);
                    avg_l = avg_l + alpha * (loss - avg_l);
                }
                let mut rsi = 1.0f32;
                if avg_l >= 1.0e-12 {
                    let rs = avg_g / avg_l;
                    rsi = 1.0f32 - 1.0f32 / (1.0f32 + rs);
                }
                s1 = s1 + 0.0625f32 * (rsi - s1);
                s2 = s2 + 0.0625f32 * (s1 - s2);
                s3 = s3 + 0.0625f32 * (s2 - s3);
                output[i] = s3;
            }
        }
    } else if formula == 66u32 {
        if n > 0 {
            output[0] = 0.0f32;
            let mut po = open[0];
            let mut pc = close[0];
            let mut y = 0.0f32;
            for i in 1..n {
                let o = open[i];
                let h = high[i];
                let l = low[i];
                let c = close[i];
                let cy = pc;
                let oy = po;
                let mut hcy = h - cy;
                if hcy < 0.0f32 {
                    hcy = -hcy;
                }
                let mut lcy = l - cy;
                if lcy < 0.0f32 {
                    lcy = -lcy;
                }
                let mut kmax = hcy;
                if lcy > kmax {
                    kmax = lcy;
                }
                let hl = h - l;
                let mut cyoy = cy - oy;
                if cyoy < 0.0f32 {
                    cyoy = -cyoy;
                }
                let mut r = hl + 0.25f32 * cyoy;
                if hcy >= lcy {
                    if hcy >= hl {
                        r = hcy + 0.5f32 * lcy + 0.25f32 * cyoy;
                    }
                } else if lcy >= hl {
                    r = lcy + 0.5f32 * hcy + 0.25f32 * cyoy;
                }
                let mut t = c * 0.03f32;
                if t < 1.0e-10 {
                    t = 1.0e-10;
                }
                let mut si = 0.0f32;
                if r > 1.0e-10 {
                    let numer = (cy - c) + 0.5f32 * (cy - oy) + 0.25f32 * (c - o);
                    si = 50.0f32 * numer / r * kmax / t;
                    if si > 100.0f32 {
                        si = 100.0f32;
                    }
                    if si < -100.0f32 {
                        si = -100.0f32;
                    }
                }
                y = y + si;
                po = o;
                pc = c;
                output[i] = y;
            }
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map_e(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    formula: u32,
) {
    bar_scan_e(open, high, low, close, volume, output, lane, period, formula);
}

/// Bar formulas 67 and 68. `a` is the VaR confidence.
#[cube]
fn bar_scan_f(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    a: f32,
    formula: u32,
) {
    let n = open.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    if formula == 67u32 {
        let mut win = p;
        if win < 2 {
            win = 2;
        }
        let mut conf = a;
        if conf < 0.5f32 {
            conf = 0.5f32;
        }
        if conf > 0.9999f32 {
            conf = 0.9999f32;
        }
        for i in 0..n {
            let nret = count_log_returns(open, high, low, close, volume, i, lane);
            if nret < win {
                output[i] = 0.0f32;
            } else {
                let mut q = (1.0f32 - conf) * (win as f32);
                if q < 0.0f32 {
                    q = 0.0f32;
                }
                let mut idx = 0usize;
                let mut acc = 0.0f32;
                for _step in 0..win {
                    if acc + 1.0f32 <= q {
                        acc = acc + 1.0f32;
                        idx = idx + 1;
                    }
                }
                if idx >= win {
                    idx = win - 1;
                }
                let base = nret - win;
                let picked = log_return_rank(
                    open, high, low, close, volume, i, base, win, idx, lane,
                );
                output[i] = -picked;
            }
        }
    } else if formula == 68u32 {
        let mut y = 50.0f32;
        let ten = 10.0f32;
        let ln10 = ten.ln();
        for i in 0..n {
            if i + 1 >= p {
                let start = i + 1 - p;
                let mut hh = high[start];
                let mut ll = low[start];
                let mut atr_sum = 0.0f32;
                for k in 0..p {
                    let idx = start + k;
                    if high[idx] > hh {
                        hh = high[idx];
                    }
                    if low[idx] < ll {
                        ll = low[idx];
                    }
                    atr_sum = atr_sum + wilder_tr(high, low, close, idx);
                }
                let range = hh - ll;
                if range > 1.0e-12 {
                    if atr_sum > 1.0e-12 {
                        let log_period = (p as f32).ln() / ln10;
                        let mut alp = log_period;
                        if alp < 0.0f32 {
                            alp = -alp;
                        }
                        if alp > 1.0e-12 {
                            let ratio = atr_sum / range;
                            let mut v = 100.0f32 * (ratio.ln() / ln10) / log_period;
                            if v < 0.0f32 {
                                v = 0.0f32;
                            }
                            if v > 100.0f32 {
                                v = 100.0f32;
                            }
                            y = v;
                        }
                    }
                }
            }
            output[i] = y;
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map_f(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    a: f32,
    formula: u32,
) {
    bar_scan_f(
        open, high, low, close, volume, output, lane, period, a, formula,
    );
}

/// Bar formulas 69 through 71. `a` is the envelope percent.
#[cube]
fn bar_scan_g(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    a: f32,
    formula: u32,
) {
    let n = open.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    if formula == 69u32 {
        for i in 0..n {
            let mut c5 = i + 1;
            if c5 > 5 {
                c5 = 5;
            }
            let mut c34 = i + 1;
            if c34 > 34 {
                c34 = 34;
            }
            let s5 = i + 1 - c5;
            let s34 = i + 1 - c34;
            let mut a5 = 0.0f32;
            let mut a34 = 0.0f32;
            for k in 0..c5 {
                a5 = a5 + 0.5f32 * (high[s5 + k] + low[s5 + k]);
            }
            for k in 0..c34 {
                a34 = a34 + 0.5f32 * (high[s34 + k] + low[s34 + k]);
            }
            output[i] = a5 / (c5 as f32) - a34 / (c34 as f32);
        }
    } else if formula == 70u32 {
        if p < 2 {
            p = 2;
        }
        let mut half = 0usize;
        let mut left = p;
        for _step in 0..p {
            if left >= 2 {
                left = left - 2;
                half = half + 1;
            }
        }
        let lookback = half + 1;
        for i in 0..n {
            let len = i + 1;
            if len < p + lookback {
                output[i] = 0.0f32;
            } else {
                let hist = len - lookback - 1;
                let mut sma = 0.0f32;
                if hist >= half {
                    if hist + half < len {
                        let start = hist - half;
                        let mut sum = 0.0f32;
                        for k in 0..p {
                            sum = sum + fld(open, high, low, close, volume, start + k, lane);
                        }
                        sma = sum / (p as f32);
                    }
                }
                let hist_px = fld(open, high, low, close, volume, hist, lane);
                let price = fld(open, high, low, close, volume, i, lane);
                let dpo = hist_px - sma;
                let mut ap = price;
                if ap < 0.0f32 {
                    ap = -ap;
                }
                if ap > 1.0e-12 {
                    output[i] = dpo / price;
                } else {
                    output[i] = 0.0f32;
                }
            }
        }
    } else if formula == 71u32 {
        let mut pct = a;
        if pct < 0.01f32 {
            pct = 0.01f32;
        }
        for i in 0..n {
            let mut cnt = i + 1;
            if cnt > p {
                cnt = p;
            }
            let start = i + 1 - cnt;
            let mut sum = 0.0f32;
            for k in 0..cnt {
                sum = sum + fld(open, high, low, close, volume, start + k, lane);
            }
            if cnt < p {
                output[i] = 0.0f32;
            } else {
                let middle = sum / (cnt as f32);
                let mut am = middle;
                if am < 0.0f32 {
                    am = -am;
                }
                if am > 1.0e-12 {
                    output[i] = 2.0f32 * pct / 100.0f32;
                } else {
                    output[i] = 0.0f32;
                }
            }
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map_g(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    a: f32,
    formula: u32,
) {
    bar_scan_g(
        open, high, low, close, volume, output, lane, period, a, formula,
    );
}

/// Awesome oscillator at bar `i`. Partial SMA(5) minus partial SMA(34) of the median.
#[cube]
fn ao_at(high: &[f32], low: &[f32], i: usize) -> f32 {
    let mut c5 = i + 1;
    if c5 > 5 {
        c5 = 5;
    }
    let mut c34 = i + 1;
    if c34 > 34 {
        c34 = 34;
    }
    let s5 = i + 1 - c5;
    let s34 = i + 1 - c34;
    let mut a5 = 0.0f32;
    let mut a34 = 0.0f32;
    for k in 0..c5 {
        a5 = a5 + 0.5f32 * (high[s5 + k] + low[s5 + k]);
    }
    for k in 0..c34 {
        a34 = a34 + 0.5f32 * (high[s34 + k] + low[s34 + k]);
    }
    a5 / (c5 as f32) - a34 / (c34 as f32)
}

/// Bar formulas 72 and 73.
#[cube]
fn bar_scan_h(
    high: &[f32],
    low: &[f32],
    volume: &[f32],
    output: &mut [f32],
    formula: u32,
) {
    let n = high.len();
    if formula == 72u32 {
        for i in 0..n {
            let mut cnt = i + 1;
            if cnt > 5 {
                cnt = 5;
            }
            let start = i + 1 - cnt;
            let mut sum = 0.0f32;
            for k in 0..cnt {
                sum = sum + ao_at(high, low, start + k);
            }
            output[i] = ao_at(high, low, i) - sum / (cnt as f32);
        }
    } else if formula == 73u32 {
        for i in 0..n {
            if volume[i] > 0.0f32 {
                output[i] = (high[i] - low[i]) / volume[i];
            } else {
                output[i] = 0.0f32;
            }
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map_h(
    high: &[f32],
    low: &[f32],
    volume: &[f32],
    output: &mut [f32],
    formula: u32,
) {
    bar_scan_h(high, low, volume, output, formula);
}

/// Bar formulas 74 through 79. High, low, close, volume only.
/// 74 Vfi, 75 Vzo, 76 intraday intensity percent, 77 intraday intensity ratio,
/// 78 Donchian position, 79 Donchian width. `period` is the window.
#[cube]
fn bar_scan_i(
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    period: u32,
    formula: u32,
) {
    let n = high.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    if formula == 74u32 {
        let pf = p as f32;
        let mut sum_flow = 0.0f32;
        let mut sum_vol = 0.0f32;
        for i in 0..n {
            let tp = (high[i] + low[i] + close[i]) / 3.0f32;
            sum_flow = sum_flow + (tp * volume[i] - sum_flow / pf);
            sum_vol = sum_vol + (volume[i] - sum_vol / pf);
            let mut av = sum_vol;
            if av < 0.0f32 {
                av = -av;
            }
            if av > 1.0e-12f32 {
                output[i] = sum_flow / sum_vol;
            } else {
                output[i] = 0.0f32;
            }
        }
    } else if formula == 75u32 {
        if p < 2 {
            p = 2;
        }
        if p > 1024 {
            p = 1024;
        }
        for i in 0..n {
            let mut cnt = i + 1;
            if cnt > p {
                cnt = p;
            }
            let start = i + 1 - cnt;
            let mut sp = 0.0f32;
            let mut sn = 0.0f32;
            for k in 0..cnt {
                let idx = start + k;
                let mut upf = 1.0f32;
                if idx > 0 {
                    if close[idx] < close[idx - 1] {
                        upf = 0.0f32;
                    }
                }
                if upf > 0.5f32 {
                    sp = sp + volume[idx];
                } else {
                    sn = sn + volume[idx];
                }
            }
            let mut denom = sp + sn;
            if denom < 0.0f32 {
                denom = -denom;
            }
            if denom < 1.0e-9f32 {
                denom = 1.0e-9f32;
            }
            output[i] = 100.0f32 * (sp - sn) / denom;
        }
    } else if formula == 76u32 {
        for i in 0..n {
            let mut denom = high[i] - low[i];
            if denom < 0.0f32 {
                denom = -denom;
            }
            if denom < 1.0e-9f32 {
                denom = 1.0e-9f32;
            }
            output[i] = ((2.0f32 * close[i] - high[i] - low[i]) / denom) * volume[i];
        }
    } else if formula == 77u32 {
        let alpha = 1.0f32 / (p as f32);
        let mut acc = 0.0f32;
        for i in 0..n {
            let mut denom = high[i] - low[i];
            if denom < 0.0f32 {
                denom = -denom;
            }
            if denom < 1.0e-9f32 {
                denom = 1.0e-9f32;
            }
            let iip = ((2.0f32 * close[i] - high[i] - low[i]) / denom) * volume[i];
            acc = (1.0f32 - alpha) * acc + alpha * iip;
            if acc > 1.0e9f32 {
                acc = 1.0e9f32;
            }
            if acc < -1.0e9f32 {
                acc = -1.0e9f32;
            }
            output[i] = acc;
        }
    } else if formula == 78u32 || formula == 79u32 {
        if p < 2 {
            p = 2;
        }
        for i in 0..n {
            let mut res = 0.0f32;
            if formula == 78u32 {
                res = 0.5f32;
            }
            if i + 1 >= p {
                let start = i + 1 - p;
                let mut hh = high[start];
                let mut ll = low[start];
                for k in 0..p {
                    if high[start + k] > hh {
                        hh = high[start + k];
                    }
                    if low[start + k] < ll {
                        ll = low[start + k];
                    }
                }
                let mut width = hh - ll;
                if formula == 79u32 {
                    res = width;
                } else {
                    if width < 0.0f32 {
                        width = 0.0f32;
                    }
                    if width > 0.0f32 {
                        res = (close[i] - ll) / width;
                    }
                }
            }
            output[i] = res;
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map_i(
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    period: u32,
    formula: u32,
) {
    bar_scan_i(high, low, close, volume, output, period, formula);
}

/// Bar formulas 80 through 85. 80 price channel oscillator, 81 price channel
/// width, 82 efficiency ratio over the full history, 83 efficiency ratio over a
/// ring window, 84 R-squared (ring-slot x axis, as the feed), 85 VWAP distance.
#[cube]
fn bar_scan_j(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    formula: u32,
) {
    let n = open.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    if formula == 80u32 || formula == 81u32 {
        if p < 2 {
            p = 2;
        }
        if p > 512 {
            p = 512;
        }
        for i in 0..n {
            let mut cnt = i + 1;
            if cnt > p {
                cnt = p;
            }
            let start = i + 1 - cnt;
            let mut hh = high[start];
            let mut ll = low[start];
            for k in 0..cnt {
                if high[start + k] > hh {
                    hh = high[start + k];
                }
                if low[start + k] < ll {
                    ll = low[start + k];
                }
            }
            if formula == 81u32 {
                output[i] = hh - ll;
            } else {
                let mut pos = 0.5f32;
                if i + 1 >= p {
                    if hh != ll {
                        pos = (close[i] - ll) / (hh - ll);
                        if pos < 0.0f32 {
                            pos = 0.0f32;
                        }
                        if pos > 1.0f32 {
                            pos = 1.0f32;
                        }
                    }
                }
                output[i] = 2.0f32 * pos - 1.0f32;
            }
        }
    } else if formula == 82u32 {
        let mut sum = 0.0f32;
        let first = fld(open, high, low, close, volume, 0, lane);
        let mut prev = first;
        for i in 0..n {
            let x = fld(open, high, low, close, volume, i, lane);
            if i == 0 {
                output[i] = 0.0f32;
            } else {
                let mut d = x - prev;
                if d < 0.0f32 {
                    d = -d;
                }
                sum = sum + d;
                let mut net = x - first;
                if net < 0.0f32 {
                    net = -net;
                }
                if sum == 0.0f32 {
                    output[i] = 0.0f32;
                } else {
                    output[i] = net / sum;
                }
            }
            prev = x;
        }
    } else if formula == 83u32 {
        if p < 2 {
            p = 2;
        }
        for i in 0..n {
            if i == 0 {
                output[i] = 0.0f32;
            } else {
                let mut cnt = i + 1;
                if cnt > p {
                    cnt = p;
                }
                let start = i + 1 - cnt;
                let mut sum = 0.0f32;
                for k in 1..cnt {
                    let mut d = fld(open, high, low, close, volume, start + k, lane)
                        - fld(open, high, low, close, volume, start + k - 1, lane);
                    if d < 0.0f32 {
                        d = -d;
                    }
                    sum = sum + d;
                }
                let mut net = fld(open, high, low, close, volume, i, lane)
                    - fld(open, high, low, close, volume, start, lane);
                if net < 0.0f32 {
                    net = -net;
                }
                if sum == 0.0f32 {
                    output[i] = 0.0f32;
                } else {
                    output[i] = net / sum;
                }
            }
        }
    } else if formula == 84u32 {
        if p < 5 {
            p = 5;
        }
        if p > 1024 {
            p = 1024;
        }
        let wf = p as f32;
        let mx = (wf - 1.0f32) / 2.0f32;
        let mut sxx = 0.0f32;
        for j in 0..p {
            let dx = (j as f32) - mx;
            sxx = sxx + dx * dx;
        }
        let mut r = 0usize;
        let mut last = 0.0f32;
        for i in 0..n {
            if i + 1 >= p {
                let mut my = 0.0f32;
                for j in 0..p {
                    let mut t = i;
                    if j <= r {
                        t = i - (r - j);
                    } else {
                        t = i - (r + p - j);
                    }
                    my = my + fld(open, high, low, close, volume, t, lane);
                }
                my = my / wf;
                let mut syy = 0.0f32;
                let mut sxy = 0.0f32;
                for j in 0..p {
                    let mut t = i;
                    if j <= r {
                        t = i - (r - j);
                    } else {
                        t = i - (r + p - j);
                    }
                    let dy = fld(open, high, low, close, volume, t, lane) - my;
                    syy = syy + dy * dy;
                    sxy = sxy + ((j as f32) - mx) * dy;
                }
                if wf * wf * sxx * syy > 1.0e-12f32 {
                    last = sxy * sxy / (sxx * syy);
                } else {
                    last = 0.0f32;
                }
            }
            output[i] = last;
            r = r + 1;
            if r == p {
                r = 0;
            }
        }
    } else if formula == 85u32 {
        let mut y = 0.0f32;
        for i in 0..n {
            let mut cnt = i + 1;
            if cnt > p {
                cnt = p;
            }
            let start = i + 1 - cnt;
            let mut sp = 0.0f32;
            let mut sv = 0.0f32;
            for k in 0..cnt {
                let typical = (high[start + k] + low[start + k] + close[start + k]) / 3.0f32;
                sp = sp + typical * volume[start + k];
                sv = sv + volume[start + k];
            }
            if sv > 0.0f32 {
                y = sp / sv;
            }
            let mut ay = y;
            if ay < 0.0f32 {
                ay = -ay;
            }
            if ay > 1.0e-12f32 {
                output[i] = (close[i] - y) / y;
            } else {
                output[i] = 0.0f32;
            }
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map_j(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    formula: u32,
) {
    bar_scan_j(open, high, low, close, volume, output, lane, period, formula);
}

/// Bar formulas 86 through 88. 86 Ehlers cyber cycle (`a` is alpha), 87 Kaufman-style
/// AMA over an ER ring window (`period` ER window, `fast` / `slow` periods),
/// 88 volatility break flag (`a` is alpha, `b` is the sigma threshold).
#[cube]
fn bar_scan_k(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    fast: u32,
    slow: u32,
    a: f32,
    b: f32,
    formula: u32,
) {
    let n = open.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    if formula == 86u32 {
        let mut al = a;
        if al < 0.0f32 {
            al = 0.0f32;
        }
        if al > 1.0f32 {
            al = 1.0f32;
        }
        let k1 = (1.0f32 - al / 2.0f32) * (1.0f32 - al / 2.0f32);
        let k2 = 2.0f32 * (1.0f32 - al);
        let k3 = (1.0f32 - al) * (1.0f32 - al);
        let mut prev1 = 0.0f32;
        let mut prev2 = 0.0f32;
        for i in 0..n {
            let c = fld(open, high, low, close, volume, i, lane);
            let val = k1 * (c - 2.0f32 * prev1 + prev2) + k2 * prev1 - k3 * prev2;
            prev2 = prev1;
            prev1 = c;
            output[i] = val;
        }
    } else if formula == 87u32 {
        if p < 2 {
            p = 2;
        }
        let af = 2.0f32 / ((fast as f32) + 1.0f32);
        let asl = 2.0f32 / ((slow as f32) + 1.0f32);
        let mut prior = fld(open, high, low, close, volume, 0, lane);
        output[0] = prior;
        for i in 1..n {
            let mut cnt = i + 1;
            if cnt > p {
                cnt = p;
            }
            let start = i + 1 - cnt;
            let mut sum = 0.0f32;
            for k in 1..cnt {
                let mut d = fld(open, high, low, close, volume, start + k, lane)
                    - fld(open, high, low, close, volume, start + k - 1, lane);
                if d < 0.0f32 {
                    d = -d;
                }
                sum = sum + d;
            }
            let x = fld(open, high, low, close, volume, i, lane);
            let mut net = x - fld(open, high, low, close, volume, start, lane);
            if net < 0.0f32 {
                net = -net;
            }
            let mut er = 0.0f32;
            if sum != 0.0f32 {
                er = net / sum;
            }
            let base = er * (af - asl) + asl;
            prior = prior + base * base * (x - prior);
            output[i] = prior;
        }
    } else if formula == 88u32 {
        let mut al = a;
        if al < 0.01f32 {
            al = 0.01f32;
        }
        if al > 1.0f32 {
            al = 1.0f32;
        }
        let mut thr = b;
        if thr < 0.5f32 {
            thr = 0.5f32;
        }
        let mut ema = fld(open, high, low, close, volume, 0, lane);
        output[0] = 0.0f32;
        let mut last = 0.0f32;
        for i in 1..n {
            let x = fld(open, high, low, close, volume, i, lane);
            let prev = ema;
            ema = al * x + (1.0f32 - al) * ema;
            let mut sigma = x - prev;
            if sigma < 0.0f32 {
                sigma = -sigma;
            }
            if sigma < 1.0e-9f32 {
                sigma = 1.0e-9f32;
            }
            let mut dev = x - ema;
            if dev < 0.0f32 {
                dev = -dev;
            }
            if dev > thr * sigma {
                last = 1.0f32;
            } else {
                last = 0.0f32;
            }
            output[i] = last;
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map_k(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    fast: u32,
    slow: u32,
    a: f32,
    b: f32,
    formula: u32,
) {
    bar_scan_k(
        open, high, low, close, volume, output, lane, period, fast, slow, a, b, formula,
    );
}

/// Log return `ln(close[t] / close[t - 1])` of the lane, `t >= 1`.
#[cube]
fn lane_lr(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    t: usize,
    lane: u32,
) -> f32 {
    let c1 = fld(open, high, low, close, volume, t - 1, lane);
    let c2 = fld(open, high, low, close, volume, t, lane);
    (c2 / c1).ln()
}

/// Bar formulas 89 and 90. 89 return autocorrelation (`period` window, `fast` lag),
/// 90 variance ratio (`period` window, at least 20; `fast` aggregation m, 2..=window/2).
#[cube]
fn bar_scan_l(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    fast: u32,
    formula: u32,
) {
    let n = open.len();
    if formula == 89u32 {
        let mut w = period as usize;
        if w < 2 {
            w = 2;
        }
        let mut lag = fast as usize;
        if lag < 1 {
            lag = 1;
        }
        let mut last = 0.0f32;
        for i in 0..n {
            if i >= w + lag {
                let mut sx = 0.0f32;
                let mut sy = 0.0f32;
                let mut sx2 = 0.0f32;
                let mut sy2 = 0.0f32;
                let mut sxy = 0.0f32;
                for k in 0..w {
                    let t = i - k;
                    let rt = lane_lr(open, high, low, close, volume, t, lane);
                    let rl = lane_lr(open, high, low, close, volume, t - lag, lane);
                    sx = sx + rt;
                    sy = sy + rl;
                    sx2 = sx2 + rt * rt;
                    sy2 = sy2 + rl * rl;
                    sxy = sxy + rt * rl;
                }
                let nf = w as f32;
                let mx = sx / nf;
                let my = sy / nf;
                let cov = sxy / nf - mx * my;
                let mut vx = sx2 / nf - mx * mx;
                let mut vy = sy2 / nf - my * my;
                if vx < 0.0f32 {
                    vx = 0.0f32;
                }
                if vy < 0.0f32 {
                    vy = 0.0f32;
                }
                let denom = sqrt_f(vx * vy);
                if denom > 1.0e-12f32 {
                    last = cov / denom;
                } else {
                    last = 0.0f32;
                }
            }
            output[i] = last;
        }
    } else if formula == 90u32 {
        let mut w = period as usize;
        if w < 20 {
            w = 20;
        }
        let mut half = 0usize;
        let mut left = w;
        for _step in 0..w {
            if left >= 2 {
                left = left - 2;
                half = half + 1;
            }
        }
        let mut m = fast as usize;
        if m < 2 {
            m = 2;
        }
        if m > half {
            m = half;
        }
        let wf = w as f32;
        let mf = m as f32;
        let mut last = 1.0f32;
        for i in 0..n {
            if i >= w {
                let first = i + 1 - w;
                let mut mean = 0.0f32;
                for k in 0..w {
                    mean = mean + lane_lr(open, high, low, close, volume, first + k, lane);
                }
                mean = mean / wf;
                let mut var1 = 0.0f32;
                for k in 0..w {
                    let d = lane_lr(open, high, low, close, volume, first + k, lane) - mean;
                    var1 = var1 + d * d;
                }
                var1 = var1 / wf;
                if var1 <= 1.0e-12f32 {
                    last = 1.0f32;
                } else {
                    let count = w - m + 1;
                    let cf = count as f32;
                    let mut smean = 0.0f32;
                    for s in 0..count {
                        let mut acc = 0.0f32;
                        for k in 0..m {
                            acc = acc + lane_lr(open, high, low, close, volume, first + s + k, lane);
                        }
                        smean = smean + acc;
                    }
                    smean = smean / cf;
                    let mut varm = 0.0f32;
                    for s in 0..count {
                        let mut acc = 0.0f32;
                        for k in 0..m {
                            acc = acc + lane_lr(open, high, low, close, volume, first + s + k, lane);
                        }
                        let d = acc - smean;
                        varm = varm + d * d;
                    }
                    varm = varm / cf;
                    last = (varm / var1) / mf;
                }
            }
            output[i] = last;
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map_l(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    fast: u32,
    formula: u32,
) {
    bar_scan_l(open, high, low, close, volume, output, lane, period, fast, formula);
}

/// Multi-column bar formulas, codes 100 and up. One launch writes
/// `CubeFormula::output_count()` columns into one buffer, column-major:
/// column `c` of bar `i` is `output[c * n + i]`. Column order is the manifest
/// brace order of the indicator.
/// 100 Donchian bands (upper middle lower), 101 Donchian metrics (width position),
/// 102 Aroon (up down oscillator), 103 central pivot range (bc pivot tc),
/// 104 Heikin Ashi (open high low close),
/// 105 candle anatomy (body upper_wick lower_wick long_upper long_lower; flags are 0/1).
#[cube]
fn bar_scan_m(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    output: &mut [f32],
    period: u32,
    a: f32,
    formula: u32,
) {
    let n = open.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    if formula == 100u32 || formula == 101u32 {
        for i in 0..n {
            let mut up = 0.0f32;
            let mut lo = 0.0f32;
            if i + 1 >= p {
                let start = i + 1 - p;
                up = high[start];
                lo = low[start];
                for k in 0..p {
                    if high[start + k] > up {
                        up = high[start + k];
                    }
                    if low[start + k] < lo {
                        lo = low[start + k];
                    }
                }
            }
            if formula == 100u32 {
                output[i] = up;
                output[n + i] = (up + lo) / 2.0f32;
                output[2 * n + i] = lo;
            } else {
                let width = up - lo;
                output[i] = width;
                if width > 0.0f32 {
                    output[n + i] = (close[i] - lo) / width;
                } else {
                    output[n + i] = 0.5f32;
                }
            }
        }
    } else if formula == 102u32 {
        let pf = p as f32;
        for i in 0..n {
            let mut au = 0.0f32;
            let mut ad = 0.0f32;
            if i + 1 >= p {
                let mut hi_v = high[i];
                let mut lo_v = low[i];
                let mut hi_k = 0usize;
                let mut lo_k = 0usize;
                for k in 0..p {
                    let t = i - k;
                    if high[t] > hi_v {
                        hi_v = high[t];
                        hi_k = k;
                    }
                    if low[t] < lo_v {
                        lo_v = low[t];
                        lo_k = k;
                    }
                }
                au = 100.0f32 * ((p - hi_k) as f32) / pf;
                ad = 100.0f32 * ((p - lo_k) as f32) / pf;
            }
            output[i] = au;
            output[n + i] = ad;
            output[2 * n + i] = au - ad;
        }
    } else if formula == 103u32 {
        for i in 0..n {
            let pivot = (high[i] + low[i] + close[i]) / 3.0f32;
            let mut tc = pivot - low[i];
            if tc < 0.0f32 {
                tc = -tc;
            }
            let mut bc = high[i] - pivot;
            if bc < 0.0f32 {
                bc = -bc;
            }
            let mut m = tc;
            if bc > m {
                m = bc;
            }
            output[i] = pivot - m;
            output[n + i] = pivot;
            output[2 * n + i] = pivot + m;
        }
    } else if formula == 104u32 {
        let mut ha_o = (open[0] + close[0]) / 2.0f32;
        let mut ha_c = (open[0] + high[0] + low[0] + close[0]) / 4.0f32;
        for i in 0..n {
            let c_new = (open[i] + high[i] + low[i] + close[i]) / 4.0f32;
            let mut o_new = (open[i] + close[i]) / 2.0f32;
            if i > 0 {
                o_new = (ha_o + ha_c) / 2.0f32;
            }
            let mut hh = c_new;
            if o_new > hh {
                hh = o_new;
            }
            if high[i] > hh {
                hh = high[i];
            }
            let mut ll = c_new;
            if o_new < ll {
                ll = o_new;
            }
            if low[i] < ll {
                ll = low[i];
            }
            ha_o = o_new;
            ha_c = c_new;
            output[i] = o_new;
            output[n + i] = hh;
            output[2 * n + i] = ll;
            output[3 * n + i] = c_new;
        }
    } else if formula == 105u32 {
        for i in 0..n {
            let mut range = high[i] - low[i];
            if range < 0.0f32 {
                range = -range;
            }
            let mut body = 0.0f32;
            let mut upper = 0.0f32;
            let mut lower = 0.0f32;
            let mut lu = 0.0f32;
            let mut ll = 0.0f32;
            if range > 1.0e-12f32 {
                let mut hi_oc = open[i];
                let mut lo_oc = close[i];
                if close[i] > hi_oc {
                    hi_oc = close[i];
                    lo_oc = open[i];
                }
                let mut bd = close[i] - open[i];
                if bd < 0.0f32 {
                    bd = -bd;
                }
                body = bd / range;
                let mut uw = high[i] - hi_oc;
                if uw < 0.0f32 {
                    uw = 0.0f32;
                }
                let mut lw = lo_oc - low[i];
                if lw < 0.0f32 {
                    lw = 0.0f32;
                }
                upper = uw / range;
                lower = lw / range;
                if upper >= a {
                    lu = 1.0f32;
                }
                if lower >= a {
                    ll = 1.0f32;
                }
            }
            output[i] = body;
            output[n + i] = upper;
            output[2 * n + i] = lower;
            output[3 * n + i] = lu;
            output[4 * n + i] = ll;
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map_m(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    output: &mut [f32],
    period: u32,
    a: f32,
    formula: u32,
) {
    bar_scan_m(open, high, low, close, output, period, a, formula);
}

/// Linear-weighted mean of `src[off + i + 1 - wp ..= off + i]`, or the sample itself
/// while the window is short. `i` counts from `off`.
#[cube]
fn sm_wma_at(src: &[f32], off: usize, i: usize, wp: usize) -> f32 {
    let mut out = src[off + i];
    if i + 1 >= wp {
        let start = i + 1 - wp;
        let mut acc = 0.0f32;
        let mut wsum = 0.0f32;
        for k in 0..wp {
            let w = (k + 1) as f32;
            acc = acc + src[off + start + k] * w;
            wsum = wsum + w;
        }
        out = acc / wsum;
    }
    out
}

/// Mean of the (partial) window of `p` samples ending at `i`, counted from `off`.
#[cube]
fn sm_mean_at(src: &[f32], off: usize, i: usize, p: usize) -> f32 {
    let mut window = p;
    if window > i + 1 {
        window = i + 1;
    }
    let start = i + 1 - window;
    let mut acc = 0.0f32;
    for k in 0..window {
        acc = acc + src[off + start + k];
    }
    acc / (window as f32)
}

/// Smoother pass A: codes 0 SMA, 1 EMA, 2 WMA, 3 RMA, 4 DEMA, 5 TEMA.
/// The series is `src[skip..]`; bars before `skip` write 0. Seeds and warm-up
/// follow the matching `CubeFormula` arm, so a smoother choice here is the same
/// math as the single-lane formula on that series.
#[cube]
fn smooth_scan_q(
    src: &[f32],
    dst: &mut [f32],
    code: u32,
    period: u32,
    skip: u32,
) {
    let n = src.len();
    let off = skip as usize;
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    let pf = p as f32;
    let alpha = 2.0f32 / (pf + 1.0f32);
    for z in 0..off {
        dst[z] = 0.0f32;
    }
    if n > off {
        let m = n - off;
        if code == 0u32 {
            for t in 0..m {
                dst[off + t] = sm_mean_at(src, off, t, p);
            }
        } else if code == 1u32 {
            let mut y = src[off];
            dst[off] = y;
            for t in 1..m {
                y = alpha * src[off + t] + (1.0f32 - alpha) * y;
                dst[off + t] = y;
            }
        } else if code == 2u32 {
            for t in 0..m {
                dst[off + t] = sm_wma_at(src, off, t, p);
            }
        } else if code == 3u32 {
            let mut y = src[off];
            dst[off] = y;
            for t in 1..m {
                y = (y * (pf - 1.0f32) + src[off + t]) / pf;
                dst[off + t] = y;
            }
        } else if code == 4u32 {
            let mut e1 = src[off];
            let mut e2 = e1;
            dst[off] = 2.0f32 * e1 - e2;
            for t in 1..m {
                e1 = alpha * src[off + t] + (1.0f32 - alpha) * e1;
                e2 = alpha * e1 + (1.0f32 - alpha) * e2;
                dst[off + t] = 2.0f32 * e1 - e2;
            }
        } else if code == 5u32 {
            let mut e1 = src[off];
            let mut e2 = e1;
            let mut e3 = e2;
            dst[off] = 3.0f32 * e1 - 3.0f32 * e2 + e3;
            for t in 1..m {
                e1 = alpha * src[off + t] + (1.0f32 - alpha) * e1;
                e2 = alpha * e1 + (1.0f32 - alpha) * e2;
                e3 = alpha * e2 + (1.0f32 - alpha) * e3;
                dst[off + t] = 3.0f32 * e1 - 3.0f32 * e2 + e3;
            }
        }
    }
}

#[cube(launch_unchecked)]
fn smooth_map_q(
    src: &[f32],
    dst: &mut [f32],
    code: u32,
    period: u32,
    skip: u32,
) {
    smooth_scan_q(src, dst, code, period, skip);
}

/// Smoother pass B: codes 6 TMA / Trima, 7 HMA, 8 ALMA (`a` offset, `b` sigma;
/// zero until the window is full). Own entry: these are the O(window^2) ones.
#[cube]
fn smooth_scan_r(
    src: &[f32],
    dst: &mut [f32],
    code: u32,
    period: u32,
    skip: u32,
    a: f32,
    b: f32,
) {
    let n = src.len();
    let off = skip as usize;
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    let pf = p as f32;
    for z in 0..off {
        dst[z] = 0.0f32;
    }
    if n > off {
        let m = n - off;
        if code == 6u32 {
            for t in 0..m {
                let mut window = p;
                if window > t + 1 {
                    window = t + 1;
                }
                let start = t + 1 - window;
                let mut acc = 0.0f32;
                for j in 0..window {
                    acc = acc + sm_mean_at(src, off, start + j, p);
                }
                dst[off + t] = acc / (window as f32);
            }
        } else if code == 7u32 {
            let mut half = 0usize;
            let mut left = p;
            for _step in 0..p {
                if left >= 2 {
                    left = left - 2;
                    half = half + 1;
                }
            }
            if half < 1 {
                half = 1;
            }
            let mut sq = 1usize;
            for cand in 1..p + 1 {
                if cand * cand <= p {
                    sq = cand;
                }
            }
            for t in 0..m {
                if t + 1 < sq {
                    let w1 = sm_wma_at(src, off, t, half);
                    let w2 = sm_wma_at(src, off, t, p);
                    dst[off + t] = 2.0f32 * w1 - w2;
                } else {
                    let start = t + 1 - sq;
                    let mut acc = 0.0f32;
                    let mut wsum = 0.0f32;
                    for k in 0..sq {
                        let w1 = sm_wma_at(src, off, start + k, half);
                        let w2 = sm_wma_at(src, off, start + k, p);
                        let w = (k + 1) as f32;
                        acc = acc + (2.0f32 * w1 - w2) * w;
                        wsum = wsum + w;
                    }
                    dst[off + t] = acc / wsum;
                }
            }
        } else if code == 8u32 {
            let mo = a * (pf - 1.0f32);
            let mut s = pf / b;
            if s < 1.0e-9f32 {
                s = 1.0e-9f32;
            }
            for t in 0..m {
                if t + 1 < p {
                    dst[off + t] = 0.0f32;
                } else {
                    let start = t + 1 - p;
                    let mut acc = 0.0f32;
                    let mut wsum = 0.0f32;
                    for k in 0..p {
                        let x = ((k as f32) - mo) / s;
                        let wi = (-0.5f32 * x * x).exp();
                        acc = acc + src[off + start + k] * wi;
                        wsum = wsum + wi;
                    }
                    dst[off + t] = acc / wsum;
                }
            }
        }
    }
}

#[cube(launch_unchecked)]
fn smooth_map_r(
    src: &[f32],
    dst: &mut [f32],
    code: u32,
    period: u32,
    skip: u32,
    a: f32,
    b: f32,
) {
    smooth_scan_r(src, dst, code, period, skip, a, b);
}

/// Pre-smoother series for the smoothed formulas (codes 120..=127).
/// `raw0` is the series the smoother reads; `raw1` is a second series for 125.
/// 120 Qstick `close - open`, 121 force index `volume * dclose`, 122 Coppock ROC sum
/// (`period`, `fast` lengths), 123 volume, 124 Chaikin ADL, 125 intraday intensity
/// (`raw0`) and volume (`raw1`), 126 ease of movement raw (`a` is the scale),
/// 127 true range (first bar `high - low`).
#[cube]
fn prep_scan_p(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    raw0: &mut [f32],
    raw1: &mut [f32],
    period: u32,
    fast: u32,
    a: f32,
    formula: u32,
) {
    let n = open.len();
    let mut adl = 0.0f32;
    for i in 0..n {
        raw1[i] = volume[i];
        if formula == 120u32 {
            raw0[i] = close[i] - open[i];
        } else if formula == 121u32 {
            if i == 0 {
                raw0[i] = 0.0f32;
            } else {
                raw0[i] = volume[i] * (close[i] - close[i - 1]);
            }
        } else if formula == 122u32 {
            let mut l1 = period as usize;
            if l1 < 1 {
                l1 = 1;
            }
            let mut l2 = fast as usize;
            if l2 < 1 {
                l2 = 1;
            }
            let mut r1 = 0.0f32;
            let mut r2 = 0.0f32;
            if i >= l1 {
                let base = close[i - l1];
                let mut ab = base;
                if ab < 0.0f32 {
                    ab = -ab;
                }
                if ab >= 1.0e-12f32 {
                    r1 = 100.0f32 * (close[i] - base) / base;
                }
            }
            if i >= l2 {
                let base = close[i - l2];
                let mut ab = base;
                if ab < 0.0f32 {
                    ab = -ab;
                }
                if ab >= 1.0e-12f32 {
                    r2 = 100.0f32 * (close[i] - base) / base;
                }
            }
            raw0[i] = r1 + r2;
        } else if formula == 123u32 {
            raw0[i] = volume[i];
        } else if formula == 124u32 || formula == 125u32 {
            let mut hl = high[i] - low[i];
            if hl < 0.0f32 {
                hl = -hl;
            }
            if hl < 1.0e-12f32 {
                hl = 1.0e-12f32;
            }
            if formula == 124u32 {
                adl = adl + (((close[i] - low[i]) - (high[i] - close[i])) / hl) * volume[i];
                raw0[i] = adl;
            } else {
                raw0[i] = ((2.0f32 * close[i] - high[i] - low[i]) / hl) * volume[i];
            }
        } else if formula == 126u32 {
            if i == 0 {
                raw0[i] = 0.0f32;
            } else {
                let dist = (high[i] + low[i]) / 2.0f32 - (high[i - 1] + low[i - 1]) / 2.0f32;
                let range = high[i] - low[i];
                let mut ar = range;
                if ar < 0.0f32 {
                    ar = -ar;
                }
                let mut av = volume[i];
                if av < 0.0f32 {
                    av = -av;
                }
                let mut boxr = 0.0f32;
                if ar >= 1.0e-12f32 && av >= 1.0e-12f32 {
                    boxr = volume[i] / a / range;
                }
                let mut ab = boxr;
                if ab < 0.0f32 {
                    ab = -ab;
                }
                if ab < 1.0e-12f32 {
                    raw0[i] = 0.0f32;
                } else {
                    raw0[i] = dist / boxr;
                }
            }
        } else if formula == 127u32 {
            let mut tr = high[i] - low[i];
            if i > 0 {
                let mut d1 = high[i] - close[i - 1];
                if d1 < 0.0f32 {
                    d1 = -d1;
                }
                let mut d2 = low[i] - close[i - 1];
                if d2 < 0.0f32 {
                    d2 = -d2;
                }
                if d1 > tr {
                    tr = d1;
                }
                if d2 > tr {
                    tr = d2;
                }
            }
            raw0[i] = tr;
        }
    }
}

#[cube(launch_unchecked)]
fn prep_map_p(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    raw0: &mut [f32],
    raw1: &mut [f32],
    period: u32,
    fast: u32,
    a: f32,
    formula: u32,
) {
    prep_scan_p(open, high, low, close, volume, raw0, raw1, period, fast, a, formula);
}

/// Final combine of the smoothed series. 123 and 124 are `sm0 - sm1`;
/// 125 is `100 * sm0 / sm1` (0 when `|sm1| < 1e-12`); 127 is `100 * sm0 / |close|`
/// (0 when `|close| < 1e-12`); every other code is `sm0`.
#[cube]
fn comb_scan_s(
    close: &[f32],
    sm0: &[f32],
    sm1: &[f32],
    output: &mut [f32],
    formula: u32,
) {
    let n = close.len();
    for i in 0..n {
        if formula == 123u32 || formula == 124u32 {
            output[i] = sm0[i] - sm1[i];
        } else if formula == 125u32 {
            let mut den = sm1[i];
            if den < 0.0f32 {
                den = -den;
            }
            if den < 1.0e-12f32 {
                output[i] = 0.0f32;
            } else {
                output[i] = 100.0f32 * sm0[i] / sm1[i];
            }
        } else if formula == 127u32 {
            let mut ac = close[i];
            if ac < 0.0f32 {
                ac = -ac;
            }
            if ac < 1.0e-12f32 {
                output[i] = 0.0f32;
            } else {
                output[i] = 100.0f32 * sm0[i] / ac;
            }
        } else {
            output[i] = sm0[i];
        }
    }
}

#[cube(launch_unchecked)]
fn comb_map_s(
    close: &[f32],
    sm0: &[f32],
    sm1: &[f32],
    output: &mut [f32],
    formula: u32,
) {
    comb_scan_s(close, sm0, sm1, output, formula);
}

/// Bar formulas 91 through 93. Signal outputs are packed as f32: `i8` -1 / 0 / +1.
/// 91 volume rate of change (percent, `lane` is volume, `period` is the lookback),
/// 92 Donchian breakout (`period` at least 2; the channel is 0 / 0 until it is full,
/// as the feed), 93 Heikin Ashi trend.
#[cube]
fn bar_scan_n(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    formula: u32,
) {
    let n = open.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    if formula == 91u32 {
        for i in 0..n {
            let mut res = 0.0f32;
            if i >= p {
                let past = fld(open, high, low, close, volume, i - p, lane);
                let cur = fld(open, high, low, close, volume, i, lane);
                let mut ap = past;
                if ap < 0.0f32 {
                    ap = -ap;
                }
                if ap > 1.0e-12f32 {
                    res = (cur - past) / past * 100.0f32;
                }
            }
            output[i] = res;
        }
    } else if formula == 92u32 {
        if p < 2 {
            p = 2;
        }
        for i in 0..n {
            let mut up = 0.0f32;
            let mut lo = 0.0f32;
            if i + 1 >= p {
                let start = i + 1 - p;
                up = high[start];
                lo = low[start];
                for k in 0..p {
                    if high[start + k] > up {
                        up = high[start + k];
                    }
                    if low[start + k] < lo {
                        lo = low[start + k];
                    }
                }
            }
            let mut sig = 0.0f32;
            if close[i] > up {
                sig = 1.0f32;
            } else if close[i] < lo {
                sig = -1.0f32;
            }
            output[i] = sig;
        }
    } else if formula == 93u32 {
        let mut ha_o = (open[0] + close[0]) / 2.0f32;
        let mut ha_c = (open[0] + high[0] + low[0] + close[0]) / 4.0f32;
        for i in 0..n {
            let c_new = (open[i] + high[i] + low[i] + close[i]) / 4.0f32;
            let mut o_new = (open[i] + close[i]) / 2.0f32;
            if i > 0 {
                o_new = (ha_o + ha_c) / 2.0f32;
            }
            ha_o = o_new;
            ha_c = c_new;
            let mut sig = 0.0f32;
            if c_new > o_new {
                sig = 1.0f32;
            } else if c_new < o_new {
                sig = -1.0f32;
            }
            output[i] = sig;
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map_n(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    formula: u32,
) {
    bar_scan_n(open, high, low, close, volume, output, lane, period, formula);
}

/// Calendar-effect formulas (codes 140..=143). The time adapter supplies the
/// calendar columns of each bar (`GpuTimes`); `state` is 128 zeroed f32 of bucket
/// counts, sums and means. Each output is the feed's mean-return readout.
/// 140 weekday effect, 141 session effect, 142 month/quarter effect (month mean),
/// 143 day-of-month / week-of-quarter effect (day-of-month mean).
#[cube]
fn bar_scan_t(
    close: &[f32],
    weekday: &[f32],
    hour: &[f32],
    month: &[f32],
    dom: &[f32],
    state: &mut [f32],
    output: &mut [f32],
    formula: u32,
) {
    let n = close.len();
    for i in 0..n {
        if i == 0 {
            output[i] = 0.0f32;
        } else {
            let r = (close[i] / close[i - 1]).ln();
            if formula == 140u32 {
                let b = weekday[i] as usize;
                state[b] = state[b] + 1.0f32;
                state[8 + b] = state[8 + b] + r;
                output[i] = state[8 + b] / state[b];
            } else if formula == 141u32 {
                let h = hour[i];
                let mut b = 2usize;
                if h < 6.0f32 {
                    b = 3usize;
                } else if h < 12.0f32 {
                    b = 0usize;
                } else if h < 18.0f32 {
                    b = 1usize;
                }
                state[b] = state[b] + 1.0f32;
                state[8 + b] = state[8 + b] + r;
                output[i] = state[8 + b] / state[b];
            } else if formula == 142u32 {
                let mi = (month[i] as usize) - 1;
                state[mi] = state[mi] + 1.0f32;
                state[16 + mi] = state[16 + mi] + r;
                state[32 + mi] = state[16 + mi] / state[mi];
                let mut sum = 0.0f32;
                let mut cnt = 0.0f32;
                for m in 0..12 {
                    if state[32 + m] != 0.0f32 {
                        sum = sum + state[32 + m];
                        cnt = cnt + 1.0f32;
                    }
                }
                if cnt < 1.0f32 {
                    cnt = 1.0f32;
                }
                output[i] = sum / cnt;
            } else if formula == 143u32 {
                let di = (dom[i] as usize) - 1;
                state[di] = state[di] + 1.0f32;
                state[32 + di] = state[32 + di] + r;
                state[64 + di] = state[32 + di] / state[di];
                let mut sum = 0.0f32;
                let mut cnt = 0.0f32;
                for d in 0..31 {
                    if state[64 + d] != 0.0f32 {
                        sum = sum + state[64 + d];
                        cnt = cnt + 1.0f32;
                    }
                }
                if cnt < 1.0f32 {
                    cnt = 1.0f32;
                }
                output[i] = sum / cnt;
            }
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map_t(
    close: &[f32],
    weekday: &[f32],
    hour: &[f32],
    month: &[f32],
    dom: &[f32],
    state: &mut [f32],
    output: &mut [f32],
    formula: u32,
) {
    bar_scan_t(close, weekday, hour, month, dom, state, output, formula);
}

/// Bar formula 94, Hampel filter. Own entry: the median and MAD are rank scans,
/// O(window^2) per bar. `period` is the window (3..=512), `a` is k (3 when not positive).
/// The feed uses the upper median (`len / 2`) for both the median and the MAD.
#[cube]
fn bar_scan_o(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    a: f32,
    formula: u32,
) {
    let n = open.len();
    let mut win = period as usize;
    if win < 3 {
        win = 3;
    }
    if win > 512 {
        win = 512;
    }
    let mut k = a;
    if k <= 0.0f32 {
        k = 3.0f32;
    }
    let mut half = 0usize;
    let mut left = win;
    for _step in 0..win {
        if left >= 2 {
            left = left - 2;
            half = half + 1;
        }
    }
    if formula == 94u32 {
        for i in 0..n {
            if i + 1 < win {
                output[i] = 0.0f32;
            } else {
                let start = i + 1 - win;
                let x = fld(open, high, low, close, volume, i, lane);
                let med = window_rank(open, high, low, close, volume, start, win, half, lane);
                let mad = window_dev_rank(
                    open, high, low, close, volume, start, win, half, lane, med,
                );
                let sigma = 1.4826f32 * mad;
                let mut denom = sigma;
                if denom < 1.0e-9f32 {
                    denom = 1.0e-9f32;
                }
                let z = (x - med) / denom;
                let mut az = z;
                if az < 0.0f32 {
                    az = -az;
                }
                if az > k {
                    if z > 0.0f32 {
                        output[i] = med + k * sigma;
                    } else {
                        output[i] = med - k * sigma;
                    }
                } else {
                    output[i] = x;
                }
            }
        }
    }
}

#[cube(launch_unchecked)]
fn bar_map_o(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    a: f32,
    formula: u32,
) {
    bar_scan_o(open, high, low, close, volume, output, lane, period, a, formula);
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
    assert!(
        !formula.needs_time(),
        "calendar formulas need the time adapter: use launch_cube_timed"
    );
    if formula.is_smoothed() {
        return launch_cube_smoothed(formula, samples, params);
    }
    if formula.code() >= 1200 && formula.code() < 1400 {
        return super::kernels_bar::launch_cube_bar(formula, samples, params).swap_remove(0);
    }
    if formula.code() >= 200 && formula.code() < 300 {
        return launch_cube_gx(formula, samples, params);
    }
    if formula.code() >= 400 && formula.code() < 500 && formula.output_count() == 1 {
        return launch_cube_smoothed_gx(formula, samples, params);
    }
    if formula.code() >= 1000 && formula.code() < 1100 {
        return super::kernels_comp::launch_cube_comp(formula, samples, params).swap_remove(0);
    }
    if formula.code() >= 1120 && formula.code() < 1140 {
        return if formula == CubeFormula::StftComp {
            super::kernels_spec::launch_cube_stft(samples, params).swap_remove(0)
        } else {
            super::kernels_spec::launch_cube_spectral_post(formula, samples, params).swap_remove(0)
        };
    }
    if formula.code() >= 1100 && formula.code() < 1200 {
        return super::kernels_spec::launch_cube_spectral(formula, samples, params).swap_remove(0);
    }
    if formula.code() >= 830 && formula.code() < 850 {
        return super::kernels_post::launch_cube_barsig(formula, samples, params).swap_remove(0);
    }
    if formula.code() >= 810 && formula.code() < 830 {
        return super::kernels_post::launch_cube_signal2(formula, samples, params);
    }
    if formula.code() >= 800 && formula.code() < 900 {
        return super::kernels_post::launch_cube_signal(formula, samples, params);
    }
    if formula.code() >= 500 && formula.code() < 700 && formula.output_count() == 1 {
        return super::kernels_post::launch_cube_post(formula, samples, params);
    }
    if formula.output_count() > 1 {
        // Multi-column formulas return their first column here; use `launch_cube_columns` for all.
        return launch_cube_columns(formula, samples, params).swap_remove(0);
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
        if formula.code() >= 94 && formula.code() < 100 {
            bar_map_o::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(open_b, n),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(volume_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                params.lane.code(),
                params.period,
                params.a,
                formula.code(),
            );
        } else if formula.code() >= 91 {
            bar_map_n::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(open_b, n),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(volume_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                params.lane.code(),
                params.period,
                formula.code(),
            );
        } else if formula.code() >= 89 {
            bar_map_l::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(open_b, n),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(volume_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                params.lane.code(),
                params.period,
                params.fast,
                formula.code(),
            );
        } else if formula.code() >= 86 {
            bar_map_k::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(open_b, n),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(volume_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                params.lane.code(),
                params.period,
                params.fast,
                params.slow,
                params.a,
                params.b,
                formula.code(),
            );
        } else if formula.code() >= 80 {
            bar_map_j::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(open_b, n),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(volume_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                params.lane.code(),
                params.period,
                formula.code(),
            );
        } else if formula.code() >= 74 {
            bar_map_i::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(volume_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                params.period,
                formula.code(),
            );
        } else if formula.code() >= 72 {
            bar_map_h::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(volume_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                formula.code(),
            );
        } else if formula.code() >= 69 {
            bar_map_g::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(open_b, n),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(volume_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                params.lane.code(),
                params.period,
                params.a,
                formula.code(),
            );
        } else if formula.code() >= 67 {
            bar_map_f::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(open_b, n),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(volume_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                params.lane.code(),
                params.period,
                params.a,
                formula.code(),
            );
        } else if formula.code() >= 63 {
            bar_map_e::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(open_b, n),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(volume_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                params.lane.code(),
                params.period,
                formula.code(),
            );
        } else if formula.code() >= 60 {
            bar_map_d::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(open_b, n),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(volume_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                params.lane.code(),
                params.period,
                formula.code(),
            );
        } else if formula.code() >= 54 {
            bar_map_c::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(open_b, n),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(volume_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                params.lane.code(),
                params.period,
                params.fast,
                params.a,
                formula.code(),
            );
        } else if formula.code() >= 45 {
            bar_map_b::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(open_b, n),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(volume_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                params.lane.code(),
                params.period,
                formula.code(),
            );
        } else if formula.code() >= 36 {
            bar_map::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(open_b, n),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(volume_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                params.lane.code(),
                params.period,
                params.a,
                formula.code(),
            );
        } else {
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
    }
    let bytes = client.read_one_unchecked(output);
    f32::from_bytes(&bytes).to_vec()
}

/// Run a multi-column cube formula. Returns `formula.output_count()` columns of
/// `samples.len()` values each, in the manifest brace order of the indicator.
/// A single-output formula returns one column, the same buffer [`launch_cube`] returns.
/// An empty slice returns no columns and does not create a device.
pub fn launch_cube_columns(
    formula: CubeFormula,
    samples: &[GpuSample],
    params: CubeParams,
) -> Vec<Vec<f32>> {
    if samples.is_empty() {
        return Vec::new();
    }
    if formula.output_count() <= 1 {
        return vec![launch_cube(formula, samples, params)];
    }
    if formula.code() >= 300 && formula.code() < 400 {
        return launch_cube_gx_columns(formula, samples, params);
    }
    if formula.code() >= 600 && formula.code() < 700 {
        return super::kernels_post::launch_cube_post_columns(formula, samples, params);
    }
    if formula.code() >= 1200 && formula.code() < 1400 {
        return super::kernels_bar::launch_cube_bar(formula, samples, params);
    }
    if formula.code() >= 1000 && formula.code() < 1100 {
        return super::kernels_comp::launch_cube_comp(formula, samples, params);
    }
    if formula.code() >= 1120 && formula.code() < 1140 {
        return if formula == CubeFormula::StftComp {
            super::kernels_spec::launch_cube_stft(samples, params)
        } else {
            super::kernels_spec::launch_cube_spectral_post(formula, samples, params)
        };
    }
    if formula.code() >= 1100 && formula.code() < 1200 {
        return super::kernels_spec::launch_cube_spectral(formula, samples, params);
    }
    if formula.code() >= 830 && formula.code() < 850 {
        return super::kernels_post::launch_cube_barsig(formula, samples, params);
    }
    assert!(
        !formula.needs_time(),
        "calendar formulas need the time adapter: use kernels_cal::launch_cube_calendar"
    );
    let n = samples.len();
    let cols = formula.output_count() as usize;
    let c = GpuSample::columns(samples);
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let open_b = client.create_from_slice(f32::as_bytes(&c.open));
    let high_b = client.create_from_slice(f32::as_bytes(&c.high));
    let low_b = client.create_from_slice(f32::as_bytes(&c.low));
    let close_b = client.create_from_slice(f32::as_bytes(&c.close));
    let output = client.empty(n * cols * core::mem::size_of::<f32>());
    unsafe {
        bar_map_m::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(open_b, n),
            BufferArg::from_raw_parts(high_b, n),
            BufferArg::from_raw_parts(low_b, n),
            BufferArg::from_raw_parts(close_b, n),
            BufferArg::from_raw_parts(output.clone(), n * cols),
            params.period,
            params.a,
            formula.code(),
        );
    }
    let bytes = client.read_one_unchecked(output);
    let flat = f32::from_bytes(&bytes).to_vec();
    flat.chunks(n).map(|ch| ch.to_vec()).collect()
}

/// Run a smoothed formula (code 120..=127): pre-smoother series, one or two
/// smoother passes chosen by [`CubeParams::smoother`] / [`CubeParams::smoother2`],
/// then the combine step. Each stage is its own launch on the same client, so no
/// shader holds more than one stage's arms.
fn launch_cube_smoothed(formula: CubeFormula, samples: &[GpuSample], params: CubeParams) -> Vec<f32> {
    let n = samples.len();
    let c = GpuSample::columns(samples);
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let open_b = client.create_from_slice(f32::as_bytes(&c.open));
    let high_b = client.create_from_slice(f32::as_bytes(&c.high));
    let low_b = client.create_from_slice(f32::as_bytes(&c.low));
    let close_b = client.create_from_slice(f32::as_bytes(&c.close));
    let volume_b = client.create_from_slice(f32::as_bytes(&c.volume));
    let bytes_n = n * core::mem::size_of::<f32>();
    let raw0 = client.empty(bytes_n);
    let raw1 = client.empty(bytes_n);
    let sm0 = client.empty(bytes_n);
    let sm1 = client.empty(bytes_n);
    let output = client.empty(bytes_n);
    let skip = formula.smooth_skip();
    let code = formula.code();
    macro_rules! smooth {
        ($src:expr, $dst:expr, $which:expr, $per:expr) => {
            if $which >= 6u32 {
                smooth_map_r::launch_unchecked(
                    &client,
                    CubeCount::new_1d(1),
                    CubeDim::new_1d(1),
                    BufferArg::from_raw_parts($src.clone(), n),
                    BufferArg::from_raw_parts($dst.clone(), n),
                    $which,
                    $per,
                    skip,
                    params.a,
                    params.b,
                );
            } else {
                smooth_map_q::launch_unchecked(
                    &client,
                    CubeCount::new_1d(1),
                    CubeDim::new_1d(1),
                    BufferArg::from_raw_parts($src.clone(), n),
                    BufferArg::from_raw_parts($dst.clone(), n),
                    $which,
                    $per,
                    skip,
                );
            }
        };
    }
    unsafe {
        prep_map_p::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(open_b, n),
            BufferArg::from_raw_parts(high_b, n),
            BufferArg::from_raw_parts(low_b, n),
            BufferArg::from_raw_parts(close_b.clone(), n),
            BufferArg::from_raw_parts(volume_b, n),
            BufferArg::from_raw_parts(raw0.clone(), n),
            BufferArg::from_raw_parts(raw1.clone(), n),
            params.period,
            params.fast,
            params.a,
            code,
        );
        let first = params.smoother.code();
        smooth!(raw0, sm0, first, params.smooth_period);
        if formula.smoother_stages() >= 2 {
            if code == 125 {
                smooth!(raw1, sm1, first, params.smooth_period);
            } else {
                smooth!(raw0, sm1, params.smoother2.code(), params.smooth_period2);
            }
        }
        comb_map_s::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(close_b, n),
            BufferArg::from_raw_parts(sm0, n),
            BufferArg::from_raw_parts(sm1, n),
            BufferArg::from_raw_parts(output.clone(), n),
            code,
        );
    }
    let bytes = client.read_one_unchecked(output);
    f32::from_bytes(&bytes).to_vec()
}

/// Run a calendar formula (code 140..=143). `times` is the per-bar calendar
/// adapter built from the bar timestamps; it must have one row per sample.
pub fn launch_cube_timed(
    formula: CubeFormula,
    samples: &[GpuSample],
    times: &GpuTimes,
    _params: CubeParams,
) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    assert!(formula.needs_time(), "launch_cube_timed needs a calendar formula (code 140..=149)");
    assert_eq!(times.len(), samples.len(), "one GpuTimes row per sample");
    if formula.code() >= 700 {
        return super::kernels_cal::launch_cube_calendar(formula, times, _params).swap_remove(0);
    }
    let n = samples.len();
    let c = GpuSample::columns(samples);
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let close_b = client.create_from_slice(f32::as_bytes(&c.close));
    let weekday_b = client.create_from_slice(f32::as_bytes(&times.weekday));
    let hour_b = client.create_from_slice(f32::as_bytes(&times.hour));
    let month_b = client.create_from_slice(f32::as_bytes(&times.month));
    let dom_b = client.create_from_slice(f32::as_bytes(&times.dom));
    let state_b = client.create_from_slice(f32::as_bytes(&[0.0f32; 128]));
    let output = client.empty(n * core::mem::size_of::<f32>());
    unsafe {
        bar_map_t::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(close_b, n),
            BufferArg::from_raw_parts(weekday_b, n),
            BufferArg::from_raw_parts(hour_b, n),
            BufferArg::from_raw_parts(month_b, n),
            BufferArg::from_raw_parts(dom_b, n),
            BufferArg::from_raw_parts(state_b, 128),
            BufferArg::from_raw_parts(output.clone(), n),
            formula.code(),
        );
    }
    let bytes = client.read_one_unchecked(output);
    f32::from_bytes(&bytes).to_vec()
}

/// Run one smoother over an arbitrary series on the device (the smoother passes of the smoothed
/// formulas, exposed for the composite oscillators). The first `skip` samples are not seen and
/// their output is 0; `a` / `b` are the ALMA offset and sigma. UNTESTED on GPU.
pub(crate) fn smooth_series(
    series: &[f32],
    smoother: super::gpu::CubeSmoother,
    period: u32,
    skip: u32,
    a: f32,
    b: f32,
) -> Vec<f32> {
    let n = series.len();
    if n == 0 {
        return Vec::new();
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let src = client.create_from_slice(f32::as_bytes(series));
    let dst = client.empty(n * core::mem::size_of::<f32>());
    let which = smoother.code();
    unsafe {
        if which >= 6u32 {
            smooth_map_r::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(src, n),
                BufferArg::from_raw_parts(dst.clone(), n),
                which,
                period,
                skip,
                a,
                b,
            );
        } else {
            smooth_map_q::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(src, n),
                BufferArg::from_raw_parts(dst.clone(), n),
                which,
                period,
                skip,
            );
        }
    }
    let bytes = client.read_one_unchecked(dst);
    f32::from_bytes(&bytes).to_vec()
}


// ---------------------------------------------------------------------------
// gx entries: formulas 200..=299. UNTESTED on GPU (no GPU on the authoring box).
// A new batch is a new entry (`gx_scan_c`, ...), never a new arm in an old one.
// ---------------------------------------------------------------------------

/// `(high + low) / 2` smoothed `(p0 + 2 p1 + 2 p2 + p3) / 6` at bar `j >= 3`.
#[cube]
fn hl2_fir(high: &[f32], low: &[f32], j: usize) -> f32 {
    let p3 = (high[j] + low[j]) / 2.0f32;
    let p2 = (high[j - 1] + low[j - 1]) / 2.0f32;
    let p1 = (high[j - 2] + low[j - 2]) / 2.0f32;
    let p0 = (high[j - 3] + low[j - 3]) / 2.0f32;
    (p3 + 2.0f32 * p2 + 2.0f32 * p1 + p0) / 6.0f32
}

/// `RealizedVol` of the lane at bar `i` over `win` returns. Zero at bar 0.
/// `a > 0` multiplies the result.
#[cube]
fn rv_at(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    i: usize,
    win: usize,
    lane: u32,
    a: f32,
) -> f32 {
    let mut out = 0.0f32;
    if i > 0 {
        let mut use_n = i;
        if use_n > win {
            use_n = win;
        }
        let mut sum = 0.0f32;
        for t in 0..use_n {
            let j = i - use_n + 1 + t;
            let mut pv = fld(open, high, low, close, volume, j - 1, lane);
            if pv < 1.0e-12f32 {
                pv = 1.0e-12f32;
            }
            let cx = fld(open, high, low, close, volume, j, lane);
            let r = (cx / pv).ln();
            sum = sum + r * r;
        }
        let vol = sqrt_f(sum / (use_n as f32));
        if a > 0.0f32 {
            out = vol * a;
        } else {
            out = vol;
        }
    }
    out
}

/// `BipowerVariance` of the lane at bar `k` (ring-slot products as the feed). Zero at bar 0.
#[cube]
fn bpv_at(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    k: usize,
    win: usize,
    lane: u32,
) -> f32 {
    let pi = 3.14159274f32;
    let mut res = 0.0f32;
    if k > 0 {
        let mut len = k;
        if len > win {
            len = win;
        }
        let mut newest = k - 1;
        for _step in 0..k {
            if newest >= win {
                newest = newest - win;
            }
        }
        let mut s = 0.0f32;
        for t in 1..len {
            let mut age_hi = newest + win - t;
            if t <= newest {
                age_hi = newest - t;
            }
            let mut age_lo = newest + win - (t - 1);
            if t - 1 <= newest {
                age_lo = newest - (t - 1);
            }
            let r_hi = k - age_hi;
            let r_lo = k - age_lo;
            let mut pv_hi = fld(open, high, low, close, volume, r_hi - 1, lane);
            if pv_hi < 1.0e-12f32 {
                pv_hi = 1.0e-12f32;
            }
            let mut ar_hi = (fld(open, high, low, close, volume, r_hi, lane) / pv_hi).ln();
            if ar_hi < 0.0f32 {
                ar_hi = -ar_hi;
            }
            let mut pv_lo = fld(open, high, low, close, volume, r_lo - 1, lane);
            if pv_lo < 1.0e-12f32 {
                pv_lo = 1.0e-12f32;
            }
            let mut ar_lo = (fld(open, high, low, close, volume, r_lo, lane) / pv_lo).ln();
            if ar_lo < 0.0f32 {
                ar_lo = -ar_lo;
            }
            s = s + ar_hi * ar_lo;
        }
        let mut denom = (len as f32) - 1.0f32;
        if denom < 1.0f32 {
            denom = 1.0f32;
        }
        res = pi * 0.5f32 * (s / denom) * 252.0f32 * 10000.0f32;
    }
    res
}

/// gx formulas 200 through 204: scalar-state recursions over bars.
#[cube]
fn gx_scan_a(
    high: &[f32],
    low: &[f32],
    close: &[f32],
    output: &mut [f32],
    period: u32,
    a: f32,
    b: f32,
    c: f32,
    formula: u32,
) {
    let n = high.len();
    if formula == 200u32 {
        // Parabolic SAR. a = af start, b = af step, c = af cap.
        let mut sar = 0.0f32;
        let mut up = 1u32;
        let mut af = a;
        let mut ep = 0.0f32;
        let mut last_high = 0.0f32;
        let mut last_low = 0.0f32;
        for i in 0..n {
            let h = high[i];
            let l = low[i];
            let prev_high = last_high;
            let prev_low = last_low;
            last_high = h;
            last_low = l;
            if i == 0 {
                sar = l;
                ep = h;
                up = 1u32;
            } else if i == 1 {
                if h > prev_high {
                    up = 1u32;
                    sar = prev_low;
                    if l < prev_low {
                        sar = l;
                    }
                    ep = h;
                } else {
                    up = 0u32;
                    sar = prev_high;
                    if h > prev_high {
                        sar = h;
                    }
                    ep = l;
                }
                af = a;
            } else {
                let prev_sar = sar;
                let new_sar = prev_sar + af * (ep - prev_sar);
                sar = new_sar;
                if up == 1u32 {
                    let mut min_low = prev_low;
                    if last_low < min_low {
                        min_low = last_low;
                    }
                    if sar > min_low {
                        sar = min_low;
                    }
                    if h > ep {
                        ep = h;
                        af = af + b;
                        if af > c {
                            af = c;
                        }
                    }
                    if l <= sar {
                        up = 0u32;
                        sar = ep;
                        ep = l;
                        af = a;
                    }
                } else {
                    let mut max_high = prev_high;
                    if last_high > max_high {
                        max_high = last_high;
                    }
                    if sar < max_high {
                        sar = max_high;
                    }
                    if l < ep {
                        ep = l;
                        af = af + b;
                        if af > c {
                            af = c;
                        }
                    }
                    if h >= sar {
                        up = 1u32;
                        sar = ep;
                        ep = h;
                        af = a;
                    }
                }
            }
            output[i] = sar;
        }
    } else if formula == 201u32 {
        // Supertrend with Wilder ATR. period = ATR period, a = multiplier.
        let mut p = period as usize;
        if p < 1 {
            p = 1;
        }
        let pf = p as f32;
        let mut atr = 0.0f32;
        let mut prev_super = 0.0f32;
        let mut prev_up = 1u32;
        for i in 0..n {
            if i == 0 {
                atr = high[0] - low[0];
            } else {
                atr = (atr * (pf - 1.0f32) + wilder_tr(high, low, close, i)) / pf;
            }
            let hl2 = (high[i] + low[i]) / 2.0f32;
            let ub = hl2 + a * atr;
            let lb = hl2 - a * atr;
            let cl = close[i];
            let mut fub = ub;
            let mut flb = lb;
            if i > 0 {
                let mut pfu = ub;
                if prev_up == 1u32 {
                    pfu = prev_super;
                }
                if ub < pfu || cl > pfu {
                    fub = ub;
                } else {
                    fub = pfu;
                }
                let mut pfl = lb;
                if prev_up == 0u32 {
                    pfl = prev_super;
                }
                if lb > pfl || cl < pfl {
                    flb = lb;
                } else {
                    flb = pfl;
                }
            }
            let mut cur_up = prev_up;
            if i == 0 {
                if cl <= fub {
                    cur_up = 0u32;
                } else {
                    cur_up = 1u32;
                }
            } else if prev_up == 1u32 && cl <= flb {
                cur_up = 0u32;
            } else if prev_up == 0u32 && cl >= fub {
                cur_up = 1u32;
            }
            let mut sv = fub;
            if cur_up == 1u32 {
                sv = flb;
            }
            prev_super = sv;
            prev_up = cur_up;
            output[i] = sv;
        }
    } else if formula == 202u32 || formula == 203u32 {
        // ADX (202) and its slope (203). Wilder ATR value feeds the DM sums, as Adx::feed.
        let mut p = period as usize;
        if p < 1 {
            p = 1;
        }
        if formula == 203u32 {
            if p < 2 {
                p = 2;
            }
        }
        let pf = p as f32;
        let sf = 1.0f32 / pf;
        let mut atr = 0.0f32;
        let mut tr_sum = 0.0f32;
        let mut pdm_sum = 0.0f32;
        let mut mdm_sum = 0.0f32;
        let mut adx = 0.0f32;
        let mut prev_adx = 0.0f32;
        let mut slope = 0.0f32;
        for i in 0..n {
            if i > 0 {
                if i == 1 {
                    atr = high[1] - low[1];
                } else {
                    atr = (atr * (pf - 1.0f32) + wilder_tr(high, low, close, i)) / pf;
                }
                let up_move = high[i] - high[i - 1];
                let down_move = low[i - 1] - low[i];
                let mut pdm = 0.0f32;
                let mut mdm = 0.0f32;
                if up_move > down_move {
                    if up_move > 0.0f32 {
                        pdm = up_move;
                    }
                }
                if down_move > up_move {
                    if down_move > 0.0f32 {
                        mdm = down_move;
                    }
                }
                let mut do_calc = false;
                if i <= p {
                    tr_sum = tr_sum + atr;
                    pdm_sum = pdm_sum + pdm;
                    mdm_sum = mdm_sum + mdm;
                    if i == p {
                        do_calc = true;
                    }
                } else {
                    tr_sum = tr_sum - tr_sum * sf + atr;
                    pdm_sum = pdm_sum - pdm_sum * sf + pdm;
                    mdm_sum = mdm_sum - mdm_sum * sf + mdm;
                    do_calc = true;
                }
                if do_calc {
                    let mut abs_tr = tr_sum;
                    if abs_tr < 0.0f32 {
                        abs_tr = -abs_tr;
                    }
                    if abs_tr >= 1.0e-12f32 {
                        let pdi = (pdm_sum / tr_sum) * 100.0f32;
                        let mdi = (mdm_sum / tr_sum) * 100.0f32;
                        let di_sum = pdi + mdi;
                        let mut abs_sum = di_sum;
                        if abs_sum < 0.0f32 {
                            abs_sum = -abs_sum;
                        }
                        let mut dx = 0.0f32;
                        if abs_sum >= 1.0e-12f32 {
                            let mut dd = pdi - mdi;
                            if dd < 0.0f32 {
                                dd = -dd;
                            }
                            dx = (dd / di_sum) * 100.0f32;
                        }
                        if i <= p {
                            if p == 1 {
                                adx = dx;
                            }
                        } else {
                            adx = adx - adx * sf + dx * sf;
                        }
                    }
                }
            }
            if i + 1 > p + p {
                if i >= p {
                    slope = adx - prev_adx;
                    prev_adx = adx;
                }
            }
            if formula == 203u32 {
                output[i] = slope;
            } else {
                output[i] = adx;
            }
        }
    } else if formula == 204u32 {
        // CusumBreakDetector. a = threshold, b = kappa.
        let mut pos = 0.0f32;
        let mut neg = 0.0f32;
        let mut value = 0.0f32;
        for i in 0..n {
            if i > 0 {
                let r = (close[i] / close[i - 1]).ln();
                pos = b * (pos + r);
                if pos < 0.0f32 {
                    pos = 0.0f32;
                }
                neg = b * (neg - r);
                if neg < 0.0f32 {
                    neg = 0.0f32;
                }
                let mut hit_pos = pos - a;
                if hit_pos < 0.0f32 {
                    hit_pos = 0.0f32;
                }
                let mut hit_neg = neg - a;
                if hit_neg < 0.0f32 {
                    hit_neg = 0.0f32;
                }
                value = hit_pos;
                if hit_neg > value {
                    value = hit_neg;
                }
                if hit_pos > 0.0f32 {
                    pos = 0.0f32;
                }
                if hit_neg > 0.0f32 {
                    neg = 0.0f32;
                }
            }
            output[i] = value;
        }
    }
}

#[cube(launch_unchecked)]
fn gx_map_a(
    high: &[f32],
    low: &[f32],
    close: &[f32],
    output: &mut [f32],
    period: u32,
    a: f32,
    b: f32,
    c: f32,
    formula: u32,
) {
    gx_scan_a(high, low, close, output, period, a, b, c, formula);
}

/// gx formulas 205 through 208: windowed volatility and the hl2 cycle.
#[cube]
fn gx_scan_b(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    fast: u32,
    slow: u32,
    a: f32,
    formula: u32,
) {
    let n = open.len();
    if formula == 205u32 {
        // HAR-RV: windows period / fast / slow.
        let mut wd = period as usize;
        if wd < 1 {
            wd = 1;
        }
        let mut ww = fast as usize;
        if ww < 1 {
            ww = 1;
        }
        let mut wm = slow as usize;
        if wm < 1 {
            wm = 1;
        }
        for i in 0..n {
            let rd = rv_at(open, high, low, close, volume, i, wd, lane, a);
            let rw = rv_at(open, high, low, close, volume, i, ww, lane, a);
            let rm = rv_at(open, high, low, close, volume, i, wm, lane, a);
            output[i] = 0.6f32 * rd + 0.3f32 * rw + 0.1f32 * rm;
        }
    } else if formula == 206u32 {
        // RbvJumpTest.
        let mut win = period as usize;
        if win < 2 {
            win = 2;
        }
        for i in 0..n {
            let bv = bpv_at(open, high, low, close, volume, i, win, lane);
            let rv = rv_at(open, high, low, close, volume, i, win, lane, a);
            let mut v = 0.0f32;
            if bv > 0.0f32 {
                let mut dm = rv * rv - bv;
                if dm < 0.0f32 {
                    dm = 0.0f32;
                }
                v = dm / (bv + 1.0e-9f32);
            }
            output[i] = v;
        }
    } else if formula == 207u32 {
        // VolOfVol, AbsReturn source: std of the last `win` |ln(c / c_prev)|.
        let mut win = period as usize;
        if win < 2 {
            win = 2;
        }
        let mut value = 0.0f32;
        for i in 0..n {
            if i >= 1 {
                let mut cnt = i;
                if cnt > win {
                    cnt = win;
                }
                if cnt >= 2 {
                    let first = i - cnt + 1;
                    let mut sum = 0.0f32;
                    for t in 0..cnt {
                        let j = first + t;
                        let mut pv = close[j - 1];
                        if pv < 1.0e-12f32 {
                            pv = 1.0e-12f32;
                        }
                        let mut x = (close[j] / pv).ln();
                        if x < 0.0f32 {
                            x = -x;
                        }
                        sum = sum + x;
                    }
                    let mean = sum / (cnt as f32);
                    let mut ss = 0.0f32;
                    for t in 0..cnt {
                        let j = first + t;
                        let mut pv = close[j - 1];
                        if pv < 1.0e-12f32 {
                            pv = 1.0e-12f32;
                        }
                        let mut x = (close[j] / pv).ln();
                        if x < 0.0f32 {
                            x = -x;
                        }
                        let d = x - mean;
                        ss = ss + d * d;
                    }
                    value = sqrt_f(ss / (cnt as f32));
                }
            }
            output[i] = value;
        }
    } else if formula == 208u32 {
        // EhlersCyberCycle of (high + low) / 2. a = alpha.
        let mut alpha = a;
        if alpha < 0.01f32 {
            alpha = 0.01f32;
        }
        if alpha > 0.99f32 {
            alpha = 0.99f32;
        }
        let omh = 1.0f32 - 0.5f32 * alpha;
        let c1 = omh * omh;
        let c2 = 2.0f32 * (1.0f32 - alpha);
        let c3 = (1.0f32 - alpha) * (1.0f32 - alpha);
        let mut cyc1 = 0.0f32;
        let mut cyc2 = 0.0f32;
        let mut ncyc = 0u32;
        let mut value = 0.0f32;
        for i in 0..n {
            if i >= 5 {
                let s2 = hl2_fir(high, low, i);
                let s1 = hl2_fir(high, low, i - 1);
                let s0 = hl2_fir(high, low, i - 2);
                let sd = s2 - 2.0f32 * s1 + s0;
                let mut cycle = c1 * sd;
                if ncyc >= 1u32 {
                    cycle = cycle + c2 * cyc1;
                }
                if ncyc >= 2u32 {
                    cycle = cycle - c3 * cyc2;
                }
                cyc2 = cyc1;
                cyc1 = cycle;
                ncyc = ncyc + 1u32;
                value = cycle;
            }
            output[i] = value;
        }
    }
}

#[cube(launch_unchecked)]
fn gx_map_b(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    output: &mut [f32],
    lane: u32,
    period: u32,
    fast: u32,
    slow: u32,
    a: f32,
    formula: u32,
) {
    gx_scan_b(open, high, low, close, volume, output, lane, period, fast, slow, a, formula);
}

/// Launch for the gx single-output formulas (code 200..=299).
fn launch_cube_gx(formula: CubeFormula, samples: &[GpuSample], params: CubeParams) -> Vec<f32> {
    let n = samples.len();
    let c = GpuSample::columns(samples);
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let open_b = client.create_from_slice(f32::as_bytes(&c.open));
    let high_b = client.create_from_slice(f32::as_bytes(&c.high));
    let low_b = client.create_from_slice(f32::as_bytes(&c.low));
    let close_b = client.create_from_slice(f32::as_bytes(&c.close));
    let volume_b = client.create_from_slice(f32::as_bytes(&c.volume));
    let output = client.empty(n * core::mem::size_of::<f32>());
    unsafe {
        if formula.code() < 205 {
            gx_map_a::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                params.period,
                params.a,
                params.b,
                params.c,
                formula.code(),
            );
        } else {
            gx_map_b::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(open_b, n),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(volume_b, n),
                BufferArg::from_raw_parts(output.clone(), n),
                params.lane.code(),
                params.period,
                params.fast,
                params.slow,
                params.a,
                formula.code(),
            );
        }
    }
    let bytes = client.read_one_unchecked(output);
    f32::from_bytes(&bytes).to_vec()
}

/// Sum over the Dm ring window ending at bar `i` (`which` 0 true range, 1 +DM, 2 -DM).
/// The window is the last `min(i, p)` bars of `1..=i`, as `Dm::feed`.
#[cube]
fn dm_sums_at(high: &[f32], low: &[f32], close: &[f32], i: usize, p: usize, which: u32) -> f32 {
    let mut start = 1usize;
    if i >= p {
        start = i + 1 - p;
    }
    let cnt = i + 1 - start;
    let mut s = 0.0f32;
    for t in 0..cnt {
        let j = start + t;
        if which == 0u32 {
            s = s + wilder_tr(high, low, close, j);
        } else {
            let up_move = high[j] - high[j - 1];
            let down_move = low[j - 1] - low[j];
            if which == 1u32 {
                if up_move > down_move {
                    if up_move > 0.0f32 {
                        s = s + up_move;
                    }
                }
            } else {
                if down_move > up_move {
                    if down_move > 0.0f32 {
                        s = s + down_move;
                    }
                }
            }
        }
    }
    s
}

/// DX of `Dm::feed` at bar `j` (`j + 1 >= p`).
#[cube]
fn dm_dx_at(high: &[f32], low: &[f32], close: &[f32], j: usize, p: usize) -> f32 {
    let ts = dm_sums_at(high, low, close, j, p, 0u32);
    let ps = dm_sums_at(high, low, close, j, p, 1u32);
    let ms = dm_sums_at(high, low, close, j, p, 2u32);
    let mut abs_ts = ts;
    if abs_ts < 0.0f32 {
        abs_ts = -abs_ts;
    }
    let mut dx = 0.0f32;
    if abs_ts >= 1.0e-12f32 {
        let pdi = 100.0f32 * ps / ts;
        let mdi = 100.0f32 * ms / ts;
        let mut diff = pdi - mdi;
        if diff < 0.0f32 {
            diff = -diff;
        }
        let sum = pdi + mdi;
        let mut abs_sum = sum;
        if abs_sum < 0.0f32 {
            abs_sum = -abs_sum;
        }
        if abs_sum >= 1.0e-12f32 {
            dx = 100.0f32 * diff / sum;
        }
    }
    dx
}

/// gx columns, formulas 300 through 305. Output holds `cols` columns of `n` values,
/// then one scratch column of `n` values (vortex ATR).
#[cube]
fn gx_cols_a(
    high: &[f32],
    low: &[f32],
    close: &[f32],
    output: &mut [f32],
    period: u32,
    formula: u32,
) {
    let n = high.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    if formula == 300u32 {
        // Vortex: columns plus, minus. Scratch column 2 holds the Wilder ATR value per bar.
        let pf = p as f32;
        let mut atr = 0.0f32;
        let mut vip = 1.0f32;
        let mut vim = 1.0f32;
        for i in 0..n {
            if i > 0 {
                if i == 1 {
                    atr = high[1] - low[1];
                } else {
                    atr = (atr * (pf - 1.0f32) + wilder_tr(high, low, close, i)) / pf;
                }
                output[2 * n + i] = atr;
                let mut start = 1usize;
                if i >= p {
                    start = i + 1 - p;
                }
                let cnt = i + 1 - start;
                let mut ps = 0.0f32;
                let mut ns = 0.0f32;
                let mut ts = 0.0f32;
                for t in 0..cnt {
                    let j = start + t;
                    let mut pv = high[j] - low[j - 1];
                    if pv < 0.0f32 {
                        pv = -pv;
                    }
                    let mut nv = low[j] - high[j - 1];
                    if nv < 0.0f32 {
                        nv = -nv;
                    }
                    ps = ps + pv;
                    ns = ns + nv;
                    ts = ts + output[2 * n + j];
                }
                let mut abs_ts = ts;
                if abs_ts < 0.0f32 {
                    abs_ts = -abs_ts;
                }
                if abs_ts > 1.0e-12f32 {
                    vip = ps / ts;
                    vim = ns / ts;
                }
            }
            output[i] = vip;
            output[n + i] = vim;
        }
    } else if formula == 302u32 {
        // DiPlusMinus: +DI / -DI of the Wilder-smoothed Adx state.
        let pf = p as f32;
        let sf = 1.0f32 / pf;
        let mut atr = 0.0f32;
        let mut tr_sum = 0.0f32;
        let mut pdm_sum = 0.0f32;
        let mut mdm_sum = 0.0f32;
        let mut pdi_v = 0.0f32;
        let mut mdi_v = 0.0f32;
        for i in 0..n {
            if i > 0 {
                if i == 1 {
                    atr = high[1] - low[1];
                } else {
                    atr = (atr * (pf - 1.0f32) + wilder_tr(high, low, close, i)) / pf;
                }
                let up_move = high[i] - high[i - 1];
                let down_move = low[i - 1] - low[i];
                let mut pdm = 0.0f32;
                let mut mdm = 0.0f32;
                if up_move > down_move {
                    if up_move > 0.0f32 {
                        pdm = up_move;
                    }
                }
                if down_move > up_move {
                    if down_move > 0.0f32 {
                        mdm = down_move;
                    }
                }
                let mut do_calc = false;
                if i <= p {
                    tr_sum = tr_sum + atr;
                    pdm_sum = pdm_sum + pdm;
                    mdm_sum = mdm_sum + mdm;
                    if i == p {
                        do_calc = true;
                    }
                } else {
                    tr_sum = tr_sum - tr_sum * sf + atr;
                    pdm_sum = pdm_sum - pdm_sum * sf + pdm;
                    mdm_sum = mdm_sum - mdm_sum * sf + mdm;
                    do_calc = true;
                }
                if do_calc {
                    let mut abs_tr = tr_sum;
                    if abs_tr < 0.0f32 {
                        abs_tr = -abs_tr;
                    }
                    if abs_tr < 1.0e-12f32 {
                        pdi_v = 0.0f32;
                        mdi_v = 0.0f32;
                    } else {
                        pdi_v = (pdm_sum / tr_sum) * 100.0f32;
                        mdi_v = (mdm_sum / tr_sum) * 100.0f32;
                    }
                }
            }
            output[i] = pdi_v;
            output[n + i] = mdi_v;
        }
    } else if formula == 303u32 {
        // Rwi: columns up (high), down (low). period at least 2.
        if p < 2 {
            p = 2;
        }
        let pf = p as f32;
        let sq = sqrt_f(pf);
        let mut atr = 0.0f32;
        let mut up = 0.0f32;
        let mut down = 0.0f32;
        for i in 0..n {
            if i == 0 {
                atr = high[0] - low[0];
            } else {
                atr = (atr * (pf - 1.0f32) + wilder_tr(high, low, close, i)) / pf;
            }
            let mut denom = atr * sq;
            if denom < 1.0e-12f32 {
                denom = 1.0e-12f32;
            }
            if i > 0 {
                let mut u = high[i] - low[i - 1];
                if u < 0.0f32 {
                    u = 0.0f32;
                }
                let mut d = high[i - 1] - low[i];
                if d < 0.0f32 {
                    d = 0.0f32;
                }
                up = u / denom;
                down = d / denom;
            }
            output[i] = up;
            output[n + i] = down;
        }
    } else if formula == 304u32 {
        // HigherMoments: skew and kurtosis of the last `win` log returns. Holds 0 until win + 1 closes.
        let mut win = p;
        if win < 3 {
            win = 3;
        }
        let mut skew = 0.0f32;
        let mut kurt = 0.0f32;
        for i in 0..n {
            if i >= win {
                let mut sum = 0.0f32;
                for k in 0..win {
                    let mut c1 = close[i - k - 1];
                    if c1 < 1.0e-12f32 {
                        c1 = 1.0e-12f32;
                    }
                    let mut c2 = close[i - k];
                    if c2 < 1.0e-12f32 {
                        c2 = 1.0e-12f32;
                    }
                    sum = sum + (c2 / c1).ln();
                }
                let wf = win as f32;
                let mean = sum / wf;
                let mut m2 = 0.0f32;
                let mut m3 = 0.0f32;
                let mut m4 = 0.0f32;
                for k in 0..win {
                    let mut c1 = close[i - k - 1];
                    if c1 < 1.0e-12f32 {
                        c1 = 1.0e-12f32;
                    }
                    let mut c2 = close[i - k];
                    if c2 < 1.0e-12f32 {
                        c2 = 1.0e-12f32;
                    }
                    let d = (c2 / c1).ln() - mean;
                    let d2 = d * d;
                    m2 = m2 + d2;
                    m3 = m3 + d2 * d;
                    m4 = m4 + d2 * d2;
                }
                m2 = m2 / wf;
                m3 = m3 / wf;
                m4 = m4 / wf;
                let mut s2 = m2;
                if s2 < 1.0e-12f32 {
                    s2 = 1.0e-12f32;
                }
                skew = m3 / (s2 * sqrt_f(s2));
                kurt = m4 / (s2 * s2);
            }
            output[i] = skew;
            output[n + i] = kurt;
        }
    } else if formula == 305u32 {
        // SwingAge: bars since the bar set a new window high / low. lookback at least 2.
        let mut len = p;
        if len < 2 {
            len = 2;
        }
        let mut ah = 0.0f32;
        let mut al = 0.0f32;
        for i in 0..n {
            if i + 1 >= len {
                let mut prev_max = high[i - 1];
                let mut prev_min = low[i - 1];
                for k in 1..len {
                    if high[i - k] > prev_max {
                        prev_max = high[i - k];
                    }
                    if low[i - k] < prev_min {
                        prev_min = low[i - k];
                    }
                }
                if high[i] >= prev_max {
                    ah = 0.0f32;
                } else {
                    ah = ah + 1.0f32;
                }
                if low[i] <= prev_min {
                    al = 0.0f32;
                } else {
                    al = al + 1.0f32;
                }
            }
            output[i] = ah;
            output[n + i] = al;
        }
    }
}

#[cube(launch_unchecked)]
fn gx_cols_map_a(
    high: &[f32],
    low: &[f32],
    close: &[f32],
    output: &mut [f32],
    period: u32,
    formula: u32,
) {
    gx_cols_a(high, low, close, output, period, formula);
}

/// gx columns, formula 301 (Dm). Own entry: the ADX ring recomputes DX over the window.
#[cube]
fn gx_cols_b(
    high: &[f32],
    low: &[f32],
    close: &[f32],
    output: &mut [f32],
    period: u32,
    formula: u32,
) {
    let n = high.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    if formula == 301u32 {
        for i in 0..n {
            let mut pdi = 0.0f32;
            let mut mdi = 0.0f32;
            let mut adx = 0.0f32;
            if i > 0 {
                if i + 1 >= p {
                    let ts = dm_sums_at(high, low, close, i, p, 0u32);
                    let ps = dm_sums_at(high, low, close, i, p, 1u32);
                    let ms = dm_sums_at(high, low, close, i, p, 2u32);
                    let mut abs_ts = ts;
                    if abs_ts < 0.0f32 {
                        abs_ts = -abs_ts;
                    }
                    if abs_ts >= 1.0e-12f32 {
                        pdi = 100.0f32 * ps / ts;
                        mdi = 100.0f32 * ms / ts;
                    }
                    if i + 2 >= p + p {
                        let start = i + 1 - p;
                        let mut dsum = 0.0f32;
                        for t in 0..p {
                            dsum = dsum + dm_dx_at(high, low, close, start + t, p);
                        }
                        adx = dsum / (p as f32);
                    }
                }
            }
            output[i] = pdi;
            output[n + i] = mdi;
            output[2 * n + i] = adx;
        }
    }
}

#[cube(launch_unchecked)]
fn gx_cols_map_b(
    high: &[f32],
    low: &[f32],
    close: &[f32],
    output: &mut [f32],
    period: u32,
    formula: u32,
) {
    gx_cols_b(high, low, close, output, period, formula);
}

/// Launch for the gx multi-column formulas (code 300..=399): `output_count` columns.
fn launch_cube_gx_columns(
    formula: CubeFormula,
    samples: &[GpuSample],
    params: CubeParams,
) -> Vec<Vec<f32>> {
    let n = samples.len();
    let cols = formula.output_count() as usize;
    let c = GpuSample::columns(samples);
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let high_b = client.create_from_slice(f32::as_bytes(&c.high));
    let low_b = client.create_from_slice(f32::as_bytes(&c.low));
    let close_b = client.create_from_slice(f32::as_bytes(&c.close));
    // One scratch column after the output columns.
    let total = n * (cols + 1);
    let output = client.empty(total * core::mem::size_of::<f32>());
    unsafe {
        if formula.code() == 301 {
            gx_cols_map_b::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(output.clone(), total),
                params.period,
                formula.code(),
            );
        } else {
            gx_cols_map_a::launch_unchecked(
                &client,
                CubeCount::new_1d(1),
                CubeDim::new_1d(1),
                BufferArg::from_raw_parts(high_b, n),
                BufferArg::from_raw_parts(low_b, n),
                BufferArg::from_raw_parts(close_b, n),
                BufferArg::from_raw_parts(output.clone(), total),
                params.period,
                formula.code(),
            );
        }
    }
    let bytes = client.read_one_unchecked(output);
    let flat = f32::from_bytes(&bytes).to_vec();
    flat.chunks(n).take(cols).map(|ch| ch.to_vec()).collect()
}

/// gx smoothed formulas, prep stage (codes 410..=415). `raw0` feeds the first smoother,
/// `raw1` the second (or the combine step).
/// 410 Ewmac / 411 Gator / 412 Ravi: the lane. 413 Twiggs money flow: `mf * volume` and volume.
/// 416..=418 Keltner width / distance / position: the lane and the Wilder true range.
/// 414 volatility ratio / 415 range over ATR: the Wilder true range (`raw1` is `max(high - low, 0)`).
#[cube]
fn gx_prep_scan(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    raw0: &mut [f32],
    raw1: &mut [f32],
    lane: u32,
    formula: u32,
) {
    let n = open.len();
    for i in 0..n {
        if formula == 413u32 {
            let mut hl = high[i] - low[i];
            if hl < 0.0f32 {
                hl = -hl;
            }
            if hl < 1.0e-12f32 {
                hl = 1.0e-12f32;
            }
            let mf = ((close[i] - low[i]) - (high[i] - close[i])) / hl;
            raw0[i] = mf * volume[i];
            raw1[i] = volume[i];
        } else if formula == 414u32 || formula == 415u32 || formula == 419u32 {
            raw0[i] = wilder_tr(high, low, close, i);
            let mut r = high[i] - low[i];
            if r < 0.0f32 {
                r = 0.0f32;
            }
            raw1[i] = r;
        } else if formula == 420u32 {
            // High-low range, never negative.
            let mut r = high[i] - low[i];
            if r < 0.0f32 {
                r = 0.0f32;
            }
            raw0[i] = r;
            raw1[i] = r;
        } else if formula == 421u32 {
            // |ln(close / previous close)|, 0 on the first bar (previous close floored at 1e-12).
            let mut r = 0.0f32;
            if i > 0 {
                let mut pv = close[i - 1];
                if pv < 1.0e-12f32 {
                    pv = 1.0e-12f32;
                }
                r = (close[i] / pv).ln();
                if r < 0.0f32 {
                    r = -r;
                }
            }
            raw0[i] = r;
            raw1[i] = r;
        } else if formula == 422u32 {
            let m = (high[i] + low[i]) / 2.0f32;
            raw0[i] = m;
            raw1[i] = m;
        } else if formula >= 416u32 && formula <= 418u32 {
            raw0[i] = fld(open, high, low, close, volume, i, lane);
            raw1[i] = wilder_tr(high, low, close, i);
        } else {
            let v = fld(open, high, low, close, volume, i, lane);
            raw0[i] = v;
            raw1[i] = v;
        }
    }
}

#[cube(launch_unchecked)]
fn gx_prep_map(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
    raw0: &mut [f32],
    raw1: &mut [f32],
    lane: u32,
    formula: u32,
) {
    gx_prep_scan(open, high, low, close, volume, raw0, raw1, lane, formula);
}

/// gx smoothed formulas, combine stage. `sm0` / `sm1` are the smoother outputs.
#[cube]
fn gx_comb_scan(
    sm0: &[f32],
    sm1: &[f32],
    raw1: &[f32],
    close: &[f32],
    output: &mut [f32],
    formula: u32,
    period: u32,
    mult: f32,
) {
    let n = sm0.len();
    for i in 0..n {
        let f = sm0[i];
        let s = sm1[i];
        let mut v = 0.0f32;
        if formula == 410u32 || formula == 411u32 {
            v = f - s;
        } else if formula == 412u32 {
            let mut abs_s = s;
            if abs_s < 0.0f32 {
                abs_s = -abs_s;
            }
            if abs_s > 1.0e-12f32 {
                let mut d = f - s;
                if d < 0.0f32 {
                    d = -d;
                }
                v = (d / s) * 100.0f32;
            }
        } else if formula == 413u32 {
            let mut abs_d = s;
            if abs_d < 0.0f32 {
                abs_d = -abs_d;
            }
            if abs_d >= 1.0e-12f32 {
                v = f / s;
            }
        } else if formula == 414u32 {
            if f > 0.0f32 {
                v = s / f;
            }
        } else if formula == 415u32 {
            if f > 1.0e-12f32 {
                v = raw1[i] / f;
            }
        } else if formula == 419u32 || formula == 423u32 {
            v = f;
        } else if formula >= 420u32 && formula <= 422u32 {
            v = raw1[i];
        } else if formula >= 416u32 && formula <= 418u32 {
            // Keltner family: `f` is the centre line, `s` the ATR, ready once `period` bars are in.
            let mut p = period as usize;
            if p < 1 {
                p = 1;
            }
            let ready = i + 1 >= p;
            let up = f + mult * s;
            let lo = f - mult * s;
            if formula == 416u32 {
                let mut am = f;
                if am < 0.0f32 {
                    am = -am;
                }
                if ready && am > 1.0e-12f32 {
                    v = (up - lo) / am;
                }
            } else if formula == 417u32 {
                if ready && s > 0.0f32 {
                    v = (close[i] - f) / s;
                }
            } else {
                v = 0.5f32;
                if ready {
                    let mut width = up - lo;
                    if width < 0.0f32 {
                        width = 0.0f32;
                    }
                    if width > 0.0f32 {
                        v = (close[i] - lo) / width;
                    }
                }
            }
        }
        output[i] = v;
    }
}

#[cube(launch_unchecked)]
fn gx_comb_map(
    sm0: &[f32],
    sm1: &[f32],
    raw1: &[f32],
    close: &[f32],
    output: &mut [f32],
    formula: u32,
    period: u32,
    mult: f32,
) {
    gx_comb_scan(sm0, sm1, raw1, close, output, formula, period, mult);
}

/// Run a gx smoothed formula (code 410..=419): prep -> smoother -> (second smoother) -> combine.
/// Each stage is its own launch, so no shader holds more than one stage's arms.
fn launch_cube_smoothed_gx(
    formula: CubeFormula,
    samples: &[GpuSample],
    params: CubeParams,
) -> Vec<f32> {
    let n = samples.len();
    let code = formula.code();
    let c = GpuSample::columns(samples);
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let open_b = client.create_from_slice(f32::as_bytes(&c.open));
    let high_b = client.create_from_slice(f32::as_bytes(&c.high));
    let low_b = client.create_from_slice(f32::as_bytes(&c.low));
    let close_b = client.create_from_slice(f32::as_bytes(&c.close));
    let volume_b = client.create_from_slice(f32::as_bytes(&c.volume));
    let bytes_n = n * core::mem::size_of::<f32>();
    let raw0 = client.empty(bytes_n);
    let raw1 = client.empty(bytes_n);
    let sm0 = client.empty(bytes_n);
    let sm1 = client.empty(bytes_n);
    let output = client.empty(bytes_n);
    macro_rules! smooth {
        ($src:expr, $dst:expr, $which:expr, $per:expr) => {
            if $which >= 6u32 {
                smooth_map_r::launch_unchecked(
                    &client,
                    CubeCount::new_1d(1),
                    CubeDim::new_1d(1),
                    BufferArg::from_raw_parts($src.clone(), n),
                    BufferArg::from_raw_parts($dst.clone(), n),
                    $which,
                    $per,
                    0u32,
                    params.a,
                    params.b,
                );
            } else {
                smooth_map_q::launch_unchecked(
                    &client,
                    CubeCount::new_1d(1),
                    CubeDim::new_1d(1),
                    BufferArg::from_raw_parts($src.clone(), n),
                    BufferArg::from_raw_parts($dst.clone(), n),
                    $which,
                    $per,
                    0u32,
                );
            }
        };
    }
    // Second-leg period: Ravi and Gator keep the slow period at 2 or more.
    let mut per2 = params.smooth_period2;
    if (code == 411 || code == 412) && per2 < 2 {
        per2 = 2;
    }
    unsafe {
        gx_prep_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(open_b, n),
            BufferArg::from_raw_parts(high_b, n),
            BufferArg::from_raw_parts(low_b, n),
            BufferArg::from_raw_parts(close_b.clone(), n),
            BufferArg::from_raw_parts(volume_b, n),
            BufferArg::from_raw_parts(raw0.clone(), n),
            BufferArg::from_raw_parts(raw1.clone(), n),
            params.lane.code(),
            code,
        );
        let first = params.smoother.code();
        if code < 420 || code == 423 {
            smooth!(raw0, sm0, first, params.smooth_period);
        }
        if code >= 419 {
            // Single smoother (419) or no smoother at all (420..): nothing more to run.
        } else if code == 413 {
            // Twiggs money flow: the same smoother over volume.
            smooth!(raw1, sm1, first, params.smooth_period);
        } else if code == 414 {
            // Volatility ratio: the same smoother, slow period.
            smooth!(raw0, sm1, first, per2);
        } else if code >= 416 && code <= 418 {
            // Keltner: centre line with the first smoother, ATR (true range) with the second.
            smooth!(raw1, sm1, params.smoother2.code(), params.smooth_period);
        } else if code != 415 {
            smooth!(raw0, sm1, params.smoother2.code(), per2);
        }
        gx_comb_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(sm0, n),
            BufferArg::from_raw_parts(sm1, n),
            BufferArg::from_raw_parts(raw1, n),
            BufferArg::from_raw_parts(close_b, n),
            BufferArg::from_raw_parts(output.clone(), n),
            code,
            params.smooth_period,
            params.a,
        );
    }
    let bytes = client.read_one_unchecked(output);
    f32::from_bytes(&bytes).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ohlcv_field::OhlcvField;
    use crate::indicators::accumulation::accumulation_distribution::AccumulationDistribution;
    use crate::indicators::signal_processing::hampel_filter::HampelFilter;
    use crate::contract::gpu_sample::GpuTimes;
    use crate::indicators::volume::vroc::VolumeRateOfChange;
    use crate::indicators::channels::donchian_breakout::DonchianBreakout;
    use crate::indicators::trend::heikin_ashi_trend::HeikinAshiTrend;
    use crate::indicators::calendar::weekday_effect::WeekdayEffect;
    use crate::indicators::calendar::session_effect::SessionEffect;
    use crate::indicators::calendar::month_quarter_effect::MonthQuarterEffect;
    use crate::indicators::calendar::dayofmonth_weekofquarter_effect::DayOfMonthWeekOfQuarterEffect;
    use crate::contract::gpu::CubeSmoother;
    use crate::engine::contract_engine::SmootherId;
    use crate::indicators::momentum::qstick::Qstick;
    use crate::indicators::accumulation::force_index::ForceIndex;
    use crate::indicators::momentum::coppock::CoppockCurve;
    use crate::indicators::volume::volume_oscillator::VolumeOscillator;
    use crate::indicators::accumulation::chaikin_oscillator::ChaikinOscillator;
    use crate::indicators::accumulation::intraday_intensity::IntradayIntensity;
    use crate::indicators::accumulation::ease_of_movement::EaseOfMovement;
    use crate::indicators::volatility::natr::Natr;
    use crate::indicators::channels::donchian_channel::DonchianChannel;
    use crate::indicators::channels::donchian_channel_metrics::DonchianMetrics;
    use crate::indicators::momentum::aroon::Aroon;
    use crate::indicators::levels::central_pivot_range::CentralPivotRange;
    use crate::indicators::candles::heikin_ashi::HeikinAshi;
    use crate::indicators::candles::candle_anatomy::CandleAnatomy;
    use crate::indicators::signal_processing::cyber_cycle::CyberCycle;
    use crate::indicators::average::ama::Ama;
    use crate::indicators::volatility::volatility_break_exp::VolatilityBreakExp;
    use crate::indicators::signal_processing::autocorr::Autocorr;
    use crate::indicators::statistics::variance_ratio::VarianceRatio;
    use crate::indicators::channels::price_channel_oscillator::PriceChannelOscillator;
    use crate::indicators::channels::price_channel_width::PriceChannelWidth;
    use crate::indicators::ratio::efficiency_ratio::EfficiencyRatioFullHistory;
    use crate::indicators::ratio::efficiency_ratio_ring::EfficiencyRatioRingWindow;
    use crate::indicators::regression::r_squared::RSquared;
    use crate::indicators::levels::vwap_distance::VwapDistance;
    use crate::indicators::volume::vfi::Vfi;
    use crate::indicators::volume::vzo::Vzo;
    use crate::indicators::accumulation::intraday_intensity_percent::IntradayIntensityPercent;
    use crate::indicators::accumulation::intraday_intensity_ratio::IntradayIntensityRatio;
    use crate::indicators::channels::donchian_position::DonchianPosition;
    use crate::indicators::channels::donchian_width::DonchianWidth;
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
    use crate::indicators::momentum::center_of_gravity::CenterOfGravity;
    use crate::indicators::momentum::demarker::Demarker;
    use crate::indicators::momentum::intraday_momentum_index::IntradayMomentumIndex;
    use crate::indicators::momentum::psl::Psl;
    use crate::indicators::channels::percent_b::PercentB;
    use crate::indicators::accumulation::accumulative_swing_index::AccumulativeSwingIndex;
    use crate::indicators::accumulation::chaikin_money_flow::ChaikinMoneyFlow;
    use crate::indicators::accumulation::williams_ad::WilliamsAd;
    use crate::indicators::average::vwap::Vwap;
    use crate::indicators::channels::envelope_bandwidth::EnvelopeBandwidth;
    use crate::indicators::chaos::williams_indicators::AccelerationDeceleration;
    use crate::indicators::chaos::williams_indicators::AwesomeOscillator;
    use crate::indicators::chaos::williams_indicators::MarketFacilitationIndex;
    use crate::indicators::momentum::dpo_percent::DpoPercent;
    use crate::indicators::momentum::rsx::Rsx;
    use crate::indicators::regime::choppiness_index::ChoppinessIndex;
    use crate::indicators::regression::var::Var;
    use crate::indicators::levels::rolling_midline::RollingMidline;
    use crate::indicators::momentum::cfo::Cfo;
    use crate::indicators::statistics::zscore_price_mad::PriceMadZscore;
    use crate::indicators::momentum::momentum_zscore::MomentumZscore;
    use crate::indicators::momentum::pfe::Pfe;
    use crate::indicators::momentum::pzo::Pzo;
    use crate::indicators::regime::vhf::Vhf;
    use crate::indicators::volume::volume_zscore::VolumeZscore;
    use crate::indicators::momentum::williams_r::WilliamsR;
    use crate::indicators::momentum::bias::Bias;
    use crate::indicators::momentum::bop::Bop;
    use crate::indicators::momentum::cmo::Cmo;
    use crate::indicators::momentum::macd::Macd;
    use crate::indicators::momentum::roc::Roc;
    use crate::indicators::momentum::rsi::Rsi;
    use crate::indicators::book::book_pressure::BookPressure;
    use crate::indicators::book::order_book_slope::OrderBookSlope;
    use crate::indicators::book::imbalance::BookImbalanceRatio;
    use crate::indicators::book::microprice::Microprice;
    use crate::indicators::book_advanced::bid_ask_asymmetry::BidAskAsymmetry;
    use crate::indicators::swing::highest::Highest;
    use crate::indicators::swing::lowest::Lowest;
    use crate::indicators::trend::efficiency_ratio::EfficiencyRatio;
    use crate::indicators::volatility::atr::Atr;
    use crate::indicators::volatility::bipower_variance::BipowerVariance;
    use crate::indicators::volatility::gapo::Gapo;
    use crate::indicators::volatility::hv_c2c::HistoricalVolatilityC2C;
    use crate::indicators::volatility::realized_quarticity::RealizedQuarticity;
    use crate::indicators::volatility::realized_vol::RealizedVol;
    use crate::indicators::volatility::ulcer_index::UlcerIndex;
    use crate::indicators::volatility::wvf::Wvf;
    use crate::indicators::volume::mfi::Mfi;
    use crate::indicators::volume::obv::Obv;
    use crate::indicators::volume::pvt::PriceVolumeTrend;
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

    /// Absolute `1e-3` covers f32 running sums. A large scaled series is also
    /// accepted within a relative `1e-5` of the f64 core.
    #[track_caller]
    fn assert_close(gpu: &[f32], cpu: &[f64]) {
        assert_eq!(gpu.len(), cpu.len());
        for (g, c) in gpu.iter().zip(cpu) {
            let delta = (*g as f64 - *c).abs();
            assert!(
                delta < 1.0e-3 || delta < c.abs() * 1.0e-5,
                "{g} vs {c} (delta {delta})"
            );
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

    fn run_cols(formula: CubeFormula, bars: &[ResearchBar], params: CubeParams) -> Vec<Vec<f32>> {
        let samples: Vec<GpuSample> = bars.iter().map(GpuSample::from).collect();
        launch_cube_columns(formula, &samples, params)
    }

    /// One `assert_close` per column; the column count must match the formula.
    #[track_caller]
    fn assert_cols(gpu: &[Vec<f32>], cpu: &[&Vec<f64>]) {
        assert_eq!(gpu.len(), cpu.len());
        for (g, c) in gpu.iter().zip(cpu) {
            assert_close(g, c);
        }
    }

    fn cpu(values: &[f64], mut step: impl FnMut(f64) -> f64) -> Vec<f64> {
        values.iter().copied().map(|v| step(v)).collect()
    }

    /// UNTESTED on GPU (no GPU on the authoring box): gx batch 1, codes 200..=208.
    #[test]
    fn lane_matches_cpu_gx_batch1() {
        use crate::indicators::statistics::cusum_break_detector::CusumBreakDetector;
        use crate::indicators::trend::adx::Adx;
        use crate::indicators::trend::adx_slope::AdxSlope;
        use crate::indicators::trend::supertrend::Supertrend;
        use crate::indicators::trend_stop::parabolic_sar::ParabolicSAR;
        use crate::indicators::trend_stop::psar_stop::PSARStop;
        use crate::indicators::momentum::ehlers_cyber_cycle::EhlersCyberCycle;
        use crate::indicators::volatility::har_rv::HarRv;
        use crate::indicators::volatility::rbv_jump_test::RbvJumpTest;
        use crate::indicators::volatility::vol_of_vol::{VoVSource, VolOfVol};

        let bars = bars(70);

        let mut sar = ParabolicSAR::with_params(0.02, 0.02, 0.2);
        let cpu_sar: Vec<f64> = bars.iter().map(|b| sar.feed(&[b.high, b.low])).collect();
        let mut sar_params = CubeParams::period(1);
        sar_params.a = 0.02;
        sar_params.b = 0.02;
        sar_params.c = 0.2;
        assert_close(&run(CubeFormula::Psar, &bars, sar_params), &cpu_sar);

        let mut psars = PSARStop::with_params(0.03, 0.01, 0.25);
        let cpu_psars: Vec<f64> = bars
            .iter()
            .map(|b| psars.feed(&[b.high, b.low, b.close]))
            .collect();
        let mut psars_params = CubeParams::period(1);
        psars_params.a = 0.03;
        psars_params.b = 0.01;
        psars_params.c = 0.25;
        assert_close(&run(CubeFormula::Psar, &bars, psars_params), &cpu_psars);

        let mut st = Supertrend::with_params(7, 3.0);
        let cpu_st: Vec<f64> = bars
            .iter()
            .map(|b| st.feed(&[b.high, b.low, b.close]))
            .collect();
        let mut st_params = CubeParams::period(7);
        st_params.a = 3.0;
        assert_close(&run(CubeFormula::Supertrend, &bars, st_params), &cpu_st);

        let mut adx = Adx::new(5);
        let cpu_adx: Vec<f64> = bars
            .iter()
            .map(|b| adx.feed(&[b.high, b.low, b.close]))
            .collect();
        assert_close(&run(CubeFormula::Adx, &bars, CubeParams::period(5)), &cpu_adx);

        let mut adx_slope = AdxSlope::new(5);
        let cpu_adx_slope: Vec<f64> = bars
            .iter()
            .map(|b| adx_slope.feed(&[b.high, b.low, b.close]))
            .collect();
        assert_close(
            &run(CubeFormula::AdxSlope, &bars, CubeParams::period(5)),
            &cpu_adx_slope,
        );

        let mut cusum = CusumBreakDetector::new(0.02, 0.9);
        let cpu_cusum: Vec<f64> = bars.iter().map(|b| cusum.feed(b.close)).collect();
        let mut cusum_params = CubeParams::period(1);
        cusum_params.a = 0.02;
        cusum_params.b = 0.9;
        assert_close(&run(CubeFormula::Cusum, &bars, cusum_params), &cpu_cusum);

        let mut har = HarRv::new(3, 7, 15, 252.0_f64.sqrt());
        let cpu_har: Vec<f64> = bars.iter().map(|b| har.feed(b.close)).collect();
        let mut har_params = CubeParams::period(3);
        har_params.fast = 7;
        har_params.slow = 15;
        har_params.a = 252.0_f64.sqrt() as f32;
        assert_close(&run(CubeFormula::Har, &bars, har_params), &cpu_har);

        let mut rbvj = RbvJumpTest::new(8, 252.0);
        let cpu_rbvj: Vec<f64> = bars.iter().map(|b| rbvj.feed(&[b.close])).collect();
        let mut rbvj_params = CubeParams::period(8);
        rbvj_params.a = 252.0;
        assert_close(&run(CubeFormula::Rbvj, &bars, rbvj_params), &cpu_rbvj);

        let mut vov = VolOfVol::new(VoVSource::AbsReturn, 8);
        let cpu_vov: Vec<f64> = bars
            .iter()
            .map(|b| vov.feed(&[b.high, b.low, b.close]))
            .collect();
        assert_close(&run(CubeFormula::VolOfVol, &bars, CubeParams::period(8)), &cpu_vov);

        let mut ecc = EhlersCyberCycle::new(0.07);
        let cpu_ecc: Vec<f64> = bars.iter().map(|b| ecc.feed(&[b.high, b.low])).collect();
        let mut ecc_params = CubeParams::period(1);
        ecc_params.a = 0.07;
        assert_close(&run(CubeFormula::EhlersCc, &bars, ecc_params), &cpu_ecc);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): gx batch 2, multi-column codes 300..=305
    /// and the Dc alias (VoDc on `DonchianBands`).
    #[test]
    fn lane_matches_cpu_gx_batch2() {
        use crate::indicators::regime::rwi::Rwi;
        use crate::indicators::statistics::higher_moments::HigherMoments;
        use crate::indicators::swing::swing_age::SwingAge;
        use crate::indicators::trend::di_plus_minus::DiPlusMinus;
        use crate::indicators::trend::dm::Dm;
        use crate::indicators::trend::vortex_indicator::VortexIndicator;
        use crate::indicators::volatility::dc::Dc;

        let bars = bars(70);

        let mut vx = VortexIndicator::with_period(5);
        let (mut vp, mut vm) = (Vec::new(), Vec::new());
        for b in &bars {
            let (p, m) = vx.feed(&[b.high, b.low, b.close]);
            vp.push(p);
            vm.push(m);
        }
        assert_cols(&run_cols(CubeFormula::Vortex, &bars, CubeParams::period(5)), &[&vp, &vm]);

        let mut dm = Dm::new(5);
        let (mut dp, mut dn, mut da) = (Vec::new(), Vec::new(), Vec::new());
        for b in &bars {
            let (p, m, a) = dm.feed(&[b.high, b.low, b.close]);
            dp.push(p);
            dn.push(m);
            da.push(a);
        }
        assert_cols(&run_cols(CubeFormula::Dm, &bars, CubeParams::period(5)), &[&dp, &dn, &da]);

        let mut dpm = DiPlusMinus::with_period(5);
        let (mut pp, mut pm) = (Vec::new(), Vec::new());
        for b in &bars {
            dpm.feed(&[b.high, b.low, b.close]);
            pp.push(dpm.plus_di());
            pm.push(dpm.minus_di());
        }
        assert_cols(&run_cols(CubeFormula::DiPlusMinus, &bars, CubeParams::period(5)), &[&pp, &pm]);

        let mut rwi = Rwi::new(5);
        let (mut ru, mut rd) = (Vec::new(), Vec::new());
        for b in &bars {
            let (u, d) = rwi.feed(&[b.high, b.low, b.close]);
            ru.push(u);
            rd.push(d);
        }
        assert_cols(&run_cols(CubeFormula::Rwi, &bars, CubeParams::period(5)), &[&ru, &rd]);

        let mut hm = HigherMoments::new(8);
        let (mut sk, mut ku) = (Vec::new(), Vec::new());
        for b in &bars {
            let (s, k) = hm.feed(b.close);
            sk.push(s);
            ku.push(k);
        }
        assert_cols(&run_cols(CubeFormula::HigherMoments, &bars, CubeParams::period(8)), &[&sk, &ku]);

        let mut sa = SwingAge::new(6);
        let (mut ah, mut al) = (Vec::new(), Vec::new());
        for b in &bars {
            let (h, l) = sa.feed(&[b.high, b.low]);
            ah.push(h as f64);
            al.push(l as f64);
        }
        assert_cols(&run_cols(CubeFormula::SwingAge, &bars, CubeParams::period(6)), &[&ah, &al]);

        let mut dc = Dc::new(5);
        let (mut du, mut dmid, mut dl) = (Vec::new(), Vec::new(), Vec::new());
        for b in &bars {
            let (u, m, l) = dc.feed(&[b.high, b.low]);
            du.push(u);
            dmid.push(m);
            dl.push(l);
        }
        assert_cols(
            &run_cols(CubeFormula::DonchianBands, &bars, CubeParams::period(5)),
            &[&du, &dmid, &dl],
        );
    }

    /// UNTESTED on GPU (no GPU on the authoring box): gx batch 3, smoothed codes 410..=415.
    #[test]
    fn lane_matches_cpu_gx_batch3() {
        use crate::indicators::accumulation::tmf::Tmf;
        use crate::indicators::momentum::gator_oscillator::GatorOscillator;
        use crate::indicators::momentum::ewmac::Ewmac;
        use crate::indicators::ratio::range_to_atr::RangeToAtr;
        use crate::indicators::trend::ravi::Ravi;
        use crate::indicators::volatility::vr::Vr;

        let bars = bars(70);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();

        let mut ew = Ewmac::from_smoothers(SmootherId::Ema, 4, SmootherId::Wma, 9);
        let mut ewp = CubeParams::period(4);
        ewp.smoother = CubeSmoother::Ema;
        ewp.smooth_period = 4;
        ewp.smoother2 = CubeSmoother::Wma;
        ewp.smooth_period2 = 9;
        assert_close(&run(CubeFormula::Ewmac, &bars, ewp), &cpu(&close, |v| ew.feed(v)));

        let mut ga = GatorOscillator::from_smoothers(SmootherId::Sma, 3, SmootherId::Rma, 8);
        let mut gap = CubeParams::period(3);
        gap.smoother = CubeSmoother::Sma;
        gap.smooth_period = 3;
        gap.smoother2 = CubeSmoother::Rma;
        gap.smooth_period2 = 8;
        assert_close(&run(CubeFormula::Gator, &bars, gap), &cpu(&close, |v| ga.feed(v)));

        let mut rv = Ravi::from_smoothers(SmootherId::Ema, 4, SmootherId::Sma, 12);
        let mut rvp = CubeParams::period(4);
        rvp.smoother = CubeSmoother::Ema;
        rvp.smooth_period = 4;
        rvp.smoother2 = CubeSmoother::Sma;
        rvp.smooth_period2 = 12;
        assert_close(&run(CubeFormula::Ravi, &bars, rvp), &cpu(&close, |v| rv.feed(v)));

        let mut tmf = Tmf::new(6);
        let cpu_tmf: Vec<f64> = bars
            .iter()
            .map(|b| tmf.feed(&[b.high, b.low, b.close, b.volume]))
            .collect();
        let mut tp = CubeParams::period(6);
        tp.smoother = CubeSmoother::Ema;
        tp.smooth_period = 6;
        assert_close(&run(CubeFormula::Tmf, &bars, tp), &cpu_tmf);

        let mut vr = Vr::from_smoothers(5, 12, SmootherId::Rma);
        let cpu_vr: Vec<f64> = bars
            .iter()
            .map(|b| vr.feed(&[b.high, b.low, b.close]))
            .collect();
        let mut vp = CubeParams::period(5);
        vp.smoother = CubeSmoother::Rma;
        vp.smooth_period = 5;
        vp.smooth_period2 = 12;
        assert_close(&run(CubeFormula::VolRatio, &bars, vp), &cpu_vr);

        let mut ra = RangeToAtr::new(7, SmootherId::Rma);
        let cpu_ra: Vec<f64> = bars
            .iter()
            .map(|b| ra.feed(&[b.high, b.low, b.close]))
            .collect();
        let mut rap = CubeParams::period(7);
        rap.smoother = CubeSmoother::Rma;
        rap.smooth_period = 7;
        assert_close(&run(CubeFormula::RangeAtr, &bars, rap), &cpu_ra);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): gx batch 4, Keltner family codes 416..=418 and Atrbw.
    #[test]
    fn lane_matches_cpu_gx_batch4() {
        use crate::indicators::channels::keltner_bandwidth::KeltnerBandwidth;
        use crate::indicators::channels::keltner_distance::KeltnerDistance;
        use crate::indicators::channels::keltner_position::KeltnerPosition;
        use crate::indicators::volatility::atr_bandwidth::AtrBandwidth;

        let bars = bars(70);
        let mut kp = CubeParams::period(8);
        kp.smoother = CubeSmoother::Sma;
        kp.smoother2 = CubeSmoother::Rma;
        kp.smooth_period = 8;
        kp.a = 1.5;
        let lanes = |b: &ResearchBar| [b.high, b.low, b.close];

        let mut bw = KeltnerBandwidth::with_smoothers(8, 1.5, SmootherId::Sma, SmootherId::Rma);
        let cpu_bw: Vec<f64> = bars.iter().map(|b| bw.feed(&lanes(b))).collect();
        assert_close(&run(CubeFormula::KeltBw, &bars, kp), &cpu_bw);

        let mut kd = KeltnerDistance::with_smoothers(8, 1.5, SmootherId::Sma, SmootherId::Rma);
        let cpu_kd: Vec<f64> = bars.iter().map(|b| kd.feed(&lanes(b))).collect();
        assert_close(&run(CubeFormula::KeltDist, &bars, kp), &cpu_kd);

        let mut kpos = KeltnerPosition::with_smoothers(8, 1.5, SmootherId::Sma, SmootherId::Rma);
        let cpu_kp: Vec<f64> = bars.iter().map(|b| kpos.feed(&lanes(b))).collect();
        assert_close(&run(CubeFormula::KeltPos, &bars, kp), &cpu_kp);

        let mut ab = AtrBandwidth::from_smoother(7, SmootherId::Rma);
        let cpu_ab: Vec<f64> = bars.iter().map(|b| ab.feed(&lanes(b))).collect();
        let mut ap = CubeParams::period(7);
        ap.smoother = CubeSmoother::Rma;
        ap.smooth_period = 7;
        assert_close(&run(CubeFormula::RangeAtr, &bars, ap), &cpu_ab);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): gx batch 5, composites 500..=504 and 419..=422.
    #[test]
    fn lane_matches_cpu_gx_batch5() {
        use crate::indicators::levels::hl_value_area::HlValueArea;
        use crate::indicators::volatility::atr_percentile::AtrPercentile;
        use crate::indicators::volatility::atr_percentile_trend::AtrPercentileTrend;
        use crate::indicators::volatility::atr_zscore::AtrZscore;
        use crate::indicators::volatility::close_to_close_vol_percentile::CloseVolPercentile;
        use crate::indicators::volatility::range_percentile::RangePercentile;
        use crate::indicators::volatility::vol_of_vol_percentile::VolOfVolPercentile;
        use crate::indicators::volatility::vol_of_vol_percentile_trend::VolOfVolPercentileTrend;

        let bars = bars(90);
        let hlc = |b: &ResearchBar| [b.high, b.low, b.close];
        let mut ap = CubeParams::period(7);
        ap.smoother = CubeSmoother::Rma;
        ap.smooth_period = 7;
        ap.slow = 12;
        ap.a = 0.3;

        let mut m = AtrPercentile::from_smoothers(7, SmootherId::Rma, 12);
        let c: Vec<f64> = bars.iter().map(|b| m.feed(&hlc(b))).collect();
        assert_close(&run(CubeFormula::AtrPct, &bars, ap), &c);

        let mut m = AtrPercentileTrend::new(7, 12, 0.3);
        let c: Vec<f64> = bars.iter().map(|b| m.feed(&hlc(b))).collect();
        assert_close(&run(CubeFormula::AtrPctTrend, &bars, ap), &c);

        let mut m = AtrZscore::from_smoother(7, SmootherId::Rma, 12);
        let c: Vec<f64> = bars.iter().map(|b| m.feed(&hlc(b))).collect();
        assert_close(&run(CubeFormula::AtrZ, &bars, ap), &c);

        let mut vp = CubeParams::period(8);
        vp.slow = 12;
        vp.a = 0.3;
        let mut m = VolOfVolPercentile::with_period(8, 12);
        let c: Vec<f64> = bars.iter().map(|b| m.feed(&hlc(b))).collect();
        assert_close(&run(CubeFormula::VovPct, &bars, vp), &c);

        let mut m = VolOfVolPercentileTrend::new(8, 12, 0.3);
        let c: Vec<f64> = bars.iter().map(|b| m.feed(&hlc(b))).collect();
        assert_close(&run(CubeFormula::VovPctTrend, &bars, vp), &c);

        let mut m = RangePercentile::new(20);
        let c: Vec<f64> = bars.iter().map(|b| m.feed(&[b.high, b.low]).0).collect();
        assert_close(&run(CubeFormula::HlRange, &bars, CubeParams::period(1)), &c);

        let mut m = CloseVolPercentile::new(10, 20);
        let c: Vec<f64> = bars.iter().map(|b| m.feed(b.close).0).collect();
        assert_close(&run(CubeFormula::AbsLogRet, &bars, CubeParams::period(1)), &c);

        let mut m = HlValueArea::new(10);
        let c: Vec<f64> = bars
            .iter()
            .map(|b| {
                m.feed(&[b.high, b.low]);
                m.value()
            })
            .collect();
        assert_close(&run(CubeFormula::Hl2, &bars, CubeParams::period(1)), &c);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): rank / quantile composites 505, 600..=602.
    #[test]
    fn lane_matches_cpu_gx_batch6() {
        use crate::indicators::channels::percentile_channels::PercentileChannels;
        use crate::indicators::levels::rolling_quartiles::RollingQuartiles;
        use crate::indicators::momentum::rsi_percentile_bands::RsiPercentileBands;
        use crate::indicators::momentum::rsi_percentile_rank::RsiPercentileRank;

        let bars = bars(90);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let mut p = CubeParams::period(5);
        p.slow = 12;

        let mut m = RsiPercentileRank::new(5, 12);
        assert_close(&run(CubeFormula::RsiPctRank, &bars, p), &cpu(&close, |v| m.feed(v)));

        let mut m = RollingQuartiles::new(12);
        let (mut q1, mut q2, mut q3) = (Vec::new(), Vec::new(), Vec::new());
        for c in &close {
            m.feed(*c);
            q1.push(m.q1());
            q2.push(m.q2());
            q3.push(m.q3());
        }
        assert_cols(&run_cols(CubeFormula::RollQuart, &bars, p), &[&q1, &q2, &q3]);

        let mut m = PercentileChannels::new(12, 0.2, 0.8);
        let (mut up, mut mid, mut lo) = (Vec::new(), Vec::new(), Vec::new());
        for c in &close {
            let (l, md, u) = m.feed(*c);
            up.push(u);
            mid.push(md);
            lo.push(l);
        }
        let mut pc = p;
        pc.a = 0.2;
        pc.b = 0.8;
        assert_cols(&run_cols(CubeFormula::PctChannels, &bars, pc), &[&up, &mid, &lo]);

        let mut m = RsiPercentileBands::new(5, 12);
        let (mut up, mut mid, mut lo) = (Vec::new(), Vec::new(), Vec::new());
        for c in &close {
            let (u, md, l) = m.feed(*c);
            up.push(u);
            mid.push(md);
            lo.push(l);
        }
        assert_cols(&run_cols(CubeFormula::RsiPctBands, &bars, p), &[&up, &mid, &lo]);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): smoother-chain composites 1000..=1009.
    #[test]
    fn lane_matches_cpu_comp_batch() {
        use crate::indicators::momentum::dpo::DetrendedPriceOscillator;
        use crate::indicators::momentum::elder_ray::ElderRay;
        use crate::indicators::momentum::kst::KnowSureThing;
        use crate::indicators::momentum::pmo::Pmo;
        use crate::indicators::momentum::ppo::Ppo;
        use crate::indicators::momentum::rsioma::RsiOma;
        use crate::indicators::momentum::trix::Trix;
        use crate::indicators::momentum::tsi::TrueStrengthIndex;
        use crate::indicators::volume::kvo::Kvo;
        use crate::indicators::volume::pvo::Pvo;

        let bars = bars(160);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let vol: Vec<f64> = bars.iter().map(|b| b.volume).collect();
        let lanes: Vec<[f64; 4]> = bars.iter().map(|b| [b.high, b.low, b.close, b.volume]).collect();
        let cols = |rows: Vec<Vec<f64>>| -> Vec<Vec<f64>> {
            (0..rows[0].len()).map(|k| rows.iter().map(|r| r[k]).collect()).collect()
        };
        let chk = |g: &[Vec<f32>], rows: Vec<Vec<f64>>| {
            let c = cols(rows);
            let r: Vec<&Vec<f64>> = c.iter().collect();
            assert_cols(g, &r);
        };
        let mut p = CubeParams::period(5);
        p.smoother = CubeSmoother::Ema;
        p.smoother2 = CubeSmoother::Ema;
        p.smoother3 = CubeSmoother::Ema;
        p.fast = 5;
        p.slow = 12;
        p.signal = 4;

        let mut m = Ppo::from_smoother(SmootherId::Ema, 5, 12, 4);
        chk(&run_cols(CubeFormula::PpoCols, &bars, p), close.iter().map(|c| { m.feed(*c); vec![m.line(), m.signal(), m.histogram()] }).collect());

        let mut m = Pvo::from_smoothers(SmootherId::Ema, 5, SmootherId::Ema, 12, SmootherId::Ema, 4);
        chk(&run_cols(CubeFormula::PvoCols, &bars, p), vol.iter().map(|c| { m.feed(*c); vec![m.line(), m.signal(), m.histogram()] }).collect());

        let mut m = Trix::from_smoother(SmootherId::Ema, 5, 4);
        chk(&run_cols(CubeFormula::TrixCols, &bars, p), close.iter().map(|c| { m.feed(*c); vec![m.line(), m.signal()] }).collect());

        let mut m = TrueStrengthIndex::from_smoother(SmootherId::Ema, 5, 12, 4);
        chk(&run_cols(CubeFormula::TsiCols, &bars, p), close.iter().map(|c| { m.feed(*c); vec![m.line(), m.signal(), m.histogram()] }).collect());

        let mut kp = p;
        kp.ext = [4, 6, 8, 10, 3, 3, 3, 4];
        let mut m = KnowSureThing::from_smoothers([4, 6, 8, 10], [3, 3, 3, 4], 4, [SmootherId::Ema; 4], SmootherId::Ema);
        chk(&run_cols(CubeFormula::KstCols, &bars, kp), close.iter().map(|c| { m.feed(*c); vec![m.kst(), m.signal()] }).collect());

        let mut pp = p;
        pp.smooth_period = 5;
        pp.smooth_period2 = 6;
        pp.smooth_period3 = 4;
        let mut m = Pmo::from_smoothers(1, 5, 6, 4, SmootherId::Ema);
        chk(&run_cols(CubeFormula::PmoCols, &bars, pp), close.iter().map(|c| { m.feed(*c); vec![m.pmo(), m.signal()] }).collect());

        let mut m = Kvo::from_smoothers(SmootherId::Ema, 5, 12, 4);
        chk(&run_cols(CubeFormula::KvoCols, &bars, p), lanes.iter().map(|l| { m.feed(l); vec![m.line(), m.signal()] }).collect());

        let mut rp = CubeParams::period(6);
        rp.smoother = CubeSmoother::Ema;
        rp.smooth_period = 5;
        let mut m = RsiOma::from_smoother(6, 5, SmootherId::Ema);
        assert_close(&run(CubeFormula::RsiOmaCols, &bars, rp), &cpu(&close, |v| m.feed(v)));

        let dp = CubeParams::period(10);
        let mut m = DetrendedPriceOscillator::from_smoother(SmootherId::Sma, 10);
        assert_close(&run(CubeFormula::DpoCols, &bars, dp), &cpu(&close, |v| m.feed(v)));

        let mut ep = CubeParams::period(7);
        ep.smoother = CubeSmoother::Ema;
        ep.smooth_period = 7;
        let mut m = ElderRay::from_smoother(7, SmootherId::Ema);
        chk(&run_cols(CubeFormula::ElderRayCols, &bars, ep), lanes.iter().map(|l| { let (a, b) = m.feed(l); vec![a, b] }).collect());
    }

    /// UNTESTED on GPU (no GPU on the authoring box): composites 1010..=1015.
    #[test]
    fn lane_matches_cpu_comp_batch2() {
        use crate::indicators::channels::bb_period::BbPeriod;
        use crate::indicators::channels::bollinger_bands::BollingerBands;
        use crate::indicators::channels::bollinger_metrics::BollingerMetrics;
        use crate::indicators::channels::envelope_channels::{EnvelopeChannels, EnvelopeMode};
        use crate::indicators::momentum::kdj::Kdj;
        use crate::indicators::momentum::stochastics::Stochastics;

        let bars = bars(140);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let lanes: Vec<[f64; 4]> = bars.iter().map(|b| [b.high, b.low, b.close, b.volume]).collect();
        let cols = |rows: Vec<Vec<f64>>| -> Vec<Vec<f64>> {
            (0..rows[0].len()).map(|k| rows.iter().map(|r| r[k]).collect()).collect()
        };
        let chk = |g: &[Vec<f32>], rows: Vec<Vec<f64>>| {
            let c = cols(rows);
            let r: Vec<&Vec<f64>> = c.iter().collect();
            assert_cols(g, &r);
        };
        for (sid, cs) in [(SmootherId::Sma, CubeSmoother::Sma), (SmootherId::Ema, CubeSmoother::Ema), (SmootherId::Wma, CubeSmoother::Wma)] {
            let mut p = CubeParams::period(9);
            p.smoother = cs;
            p.smooth_period = 5;
            p.a = 2.0;
            let mut m = BollingerBands::from_smoother(sid, 9, 2.0);
            chk(&run_cols(CubeFormula::BbCols, &bars, p), close.iter().map(|c| { m.feed(*c); vec![m.upper(), m.middle(), m.lower(), m.std_dev(), m.bandwidth(), m.percent_b()] }).collect());

            let mut m = BbPeriod::new(9, 2.0, sid);
            chk(&run_cols(CubeFormula::BbPeriodCols, &bars, p), lanes.iter().map(|l| { let (a, b, c) = m.feed(l); vec![a, b, c] }).collect());

            let mut m = EnvelopeChannels::new(9, 2.5, EnvelopeMode::Fixed, sid);
            let mut ep = p;
            ep.a = 2.5;
            chk(&run_cols(CubeFormula::EnvelopeCols, &bars, ep), close.iter().map(|c| { let (u, mi, l) = m.feed(*c); vec![u, mi, l] }).collect());

            let mut kp = p;
            kp.period = 6;
            kp.smooth_period = 4;
            let mut m = Stochastics::from_smoother(sid, 6, 4);
            chk(&run_cols(CubeFormula::StochCols, &bars, kp), lanes.iter().map(|l| { let (k, d) = m.feed(l); vec![k, d] }).collect());
            let mut m = Kdj::from_smoother(6, 4, sid);
            chk(&run_cols(CubeFormula::KdjCols, &bars, kp), lanes.iter().map(|l| { let (k, d, j) = m.feed(l); vec![k, d, j] }).collect());
        }
        let mut p = CubeParams::period(9);
        p.a = 2.0;
        let mut m = BollingerMetrics::new(9, 2.0);
        chk(&run_cols(CubeFormula::BbMetricsCols, &bars, p), close.iter().map(|c| { let (a, b) = m.feed(*c); vec![a, b] }).collect());
    }

    /// UNTESTED on GPU (no GPU on the authoring box): composites 1016..=1021.
    #[test]
    fn lane_matches_cpu_comp_batch3() {
        use crate::engine::ohlcv_field::OhlcvField;
        use crate::indicators::channels::atr_channels::{AtrChannelMode, AtrChannels as AtrChan};
        use crate::indicators::channels::keltner_channel::{KeltnerChannel, KeltnerMode};
        use crate::indicators::channels::keltner_channel_metrics::KeltnerMetrics;
        use crate::indicators::channels::starc_bands::StarcBands;
        use crate::indicators::volatility::atr_channels::AtrChannels;
        use crate::indicators::volatility::kc::Kc;

        let bars = bars(140);
        let lanes: Vec<[f64; 4]> = bars.iter().map(|b| [b.high, b.low, b.close, b.volume]).collect();
        let cols = |rows: Vec<Vec<f64>>| -> Vec<Vec<f64>> {
            (0..rows[0].len()).map(|k| rows.iter().map(|r| r[k]).collect()).collect()
        };
        let chk = |g: &[Vec<f32>], rows: Vec<Vec<f64>>| {
            let c = cols(rows);
            let r: Vec<&Vec<f64>> = c.iter().collect();
            assert_cols(g, &r);
        };
        let mut p = CubeParams::period(8);
        p.smoother = CubeSmoother::Sma;
        p.smoother2 = CubeSmoother::Rma;
        p.smooth_period = 6;
        p.a = 1.5;
        p.lane = OhlcvField::HLC3;
        let (sa, sb) = (SmootherId::Sma, SmootherId::Rma);

        let mut m = KeltnerChannel::from_smoothers(sa, sb, 8, 1.5, KeltnerMode::Classic, OhlcvField::HLC3);
        chk(&run_cols(CubeFormula::KcCols, &bars, p), lanes.iter().map(|l| { let (u, mi, lo) = m.feed(l); vec![u, mi, lo] }).collect());
        let mut m = KeltnerMetrics::with_smoothers(8, 1.5, sa, sb);
        let mut mp = p;
        mp.lane = OhlcvField::Close;
        chk(&run_cols(CubeFormula::KcMetricsCols, &bars, mp), lanes.iter().map(|l| { let (w, ps) = m.feed(l); vec![w, ps] }).collect());
        let mut m = AtrChannels::from_smoothers(8, sa, 6, sb, 1.5);
        chk(&run_cols(CubeFormula::AtrcCols, &bars, p), lanes.iter().map(|l| { let (u, mi, lo) = m.feed(l); vec![u, mi, lo] }).collect());
        let mut m = AtrChan::from_smoothers(sa, sb, 8, 1.5, AtrChannelMode::Close);
        chk(&run_cols(CubeFormula::AtrChanCols, &bars, p), lanes.iter().map(|l| { let (u, mi, lo) = m.feed(l); vec![u, mi, lo] }).collect());
        let mut m = StarcBands::from_smoothers(8, 6, 1.5, sa, sb, OhlcvField::HLC3);
        chk(&run_cols(CubeFormula::StarcCols, &bars, p), lanes.iter().map(|l| { let (u, mi, lo) = m.feed(l); vec![u, mi, lo] }).collect());
        let mut m = Kc::from_smoothers(8, 1.5, sb);
        let mut vp = p;
        vp.smoother = CubeSmoother::Rma;
        chk(&run_cols(CubeFormula::VoKcCols, &bars, vp), lanes.iter().map(|l| { let (u, mi, lo) = m.feed(l); vec![u, mi, lo] }).collect());
    }

    /// UNTESTED on GPU (no GPU on the authoring box): composites 1022..=1027.
    #[test]
    fn lane_matches_cpu_comp_batch4() {
        use crate::indicators::momentum::cci::Cci;
        use crate::indicators::momentum::detrended_synthetic_price::DetrendedSyntheticPrice;
        use crate::indicators::momentum::rmi::Rmi;
        use crate::indicators::volatility::chaikin_volatility::ChaikinVolatility;
        use crate::indicators::volatility::mass_index::MassIndex;
        use crate::indicators::volatility::rvi::Rvi;

        let bars = bars(150);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let lanes: Vec<[f64; 4]> = bars.iter().map(|b| [b.high, b.low, b.close, b.volume]).collect();
        for (sid, cs) in [(SmootherId::Sma, CubeSmoother::Sma), (SmootherId::Ema, CubeSmoother::Ema), (SmootherId::Rma, CubeSmoother::Rma)] {
            let mut p = CubeParams::period(9);
            p.smoother = cs;
            p.smooth_period = 6;
            p.a = 0.015;
            p.slow = 5;
            let mut m = Cci::from_smoother(9, 0.015, sid);
            assert_close(&run(CubeFormula::CciComp, &bars, p), &lanes.iter().map(|l| m.feed(l)).collect::<Vec<f64>>());
            let mut m = ChaikinVolatility::from_smoother(9, 5, sid);
            assert_close(&run(CubeFormula::CvComp, &bars, p), &lanes.iter().map(|l| m.feed(l)).collect::<Vec<f64>>());
            let mut m = Rmi::from_smoother(9, 6, sid);
            assert_close(&run(CubeFormula::RmiComp, &bars, p), &cpu(&close, |v| m.feed(v)));
            let mut m = Rvi::from_smoother(9, sid);
            assert_close(&run(CubeFormula::RviComp, &bars, p), &cpu(&close, |v| m.feed(v)));
            let mut m = DetrendedSyntheticPrice::from_smoother(9, sid);
            assert_close(&run(CubeFormula::DspComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        }
        let mut p = CubeParams::period(5);
        p.smoother = CubeSmoother::Ema;
        p.slow = 12;
        let mut m = MassIndex::with_params(5, 12);
        assert_close(&run(CubeFormula::MiComp, &bars, p), &lanes.iter().map(|l| m.feed(l)).collect::<Vec<f64>>());
    }

    /// UNTESTED on GPU (no GPU on the authoring box): composites 1028..=1034.
    #[test]
    fn lane_matches_cpu_comp_batch5() {
        use crate::indicators::momentum::ema_slope::EmaSlope;
        use crate::indicators::momentum::rvgi::Rvgi;
        use crate::indicators::statistics::price_zscore::PriceZScore;
        use crate::indicators::trend::didi_index::DidiIndex;
        use crate::indicators::trend::ssl_channel::SslChannel;
        use crate::indicators::trend::trend_intensity_index::TrendIntensityIndex;
        use crate::indicators::volume::vpt::VolumePriceTrend;

        let bars = bars(150);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let lanes: Vec<[f64; 4]> = bars.iter().map(|b| [b.high, b.low, b.close, b.volume]).collect();
        let cols = |rows: Vec<Vec<f64>>| -> Vec<Vec<f64>> {
            (0..rows[0].len()).map(|k| rows.iter().map(|r| r[k]).collect()).collect()
        };
        let chk = |g: &[Vec<f32>], rows: Vec<Vec<f64>>| {
            let c = cols(rows);
            let r: Vec<&Vec<f64>> = c.iter().collect();
            assert_cols(g, &r);
        };
        for (sid, cs) in [(SmootherId::Sma, CubeSmoother::Sma), (SmootherId::Ema, CubeSmoother::Ema), (SmootherId::Wma, CubeSmoother::Wma)] {
            let mut p = CubeParams::period(9);
            p.smoother = cs;
            p.fast = 3;
            p.slow = 6;
            p.signal = 12;
            let mut m = DidiIndex::from_smoothers(sid, 3, 6, 12);
            chk(&run_cols(CubeFormula::DidiCols, &bars, p), close.iter().map(|c| { m.feed(*c); vec![m.short(), m.long()] }).collect());
            let mut m = SslChannel::from_smoother(9, sid);
            chk(&run_cols(CubeFormula::SslCols, &bars, p), lanes.iter().map(|l| { m.feed(l); vec![m.up(), m.down()] }).collect());
            let mut m = EmaSlope::from_smoother(9, 6, sid);
            assert_close(&run(CubeFormula::EmaSlopeComp, &bars, p), &cpu(&close, |v| m.feed(v)));
            let mut m = PriceZScore::from_smoothers(sid, 9);
            assert_close(&run(CubeFormula::PriceZComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        }
        let mut p = CubeParams::period(9);
        p.signal = 4;
        let ohlc: Vec<[f64; 4]> = bars.iter().map(|b| [b.open, b.high, b.low, b.close]).collect();
        let mut m = Rvgi::new(9, 4);
        chk(&run_cols(CubeFormula::RvgiCols, &bars, p), ohlc.iter().map(|l| { let v = m.feed(l); vec![v, m.signal()] }).collect());
        let mut m = TrendIntensityIndex::new(9);
        assert_close(&run(CubeFormula::TiiComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        let mut m = VolumePriceTrend::new();
        let cv: Vec<[f64; 2]> = bars.iter().map(|b| [b.close, b.volume]).collect();
        assert_close(&run(CubeFormula::VptComp, &bars, p), &cv.iter().map(|l| m.feed(l)).collect::<Vec<f64>>());
    }

    /// UNTESTED on GPU (no GPU on the authoring box): composites 1035..=1039.
    #[test]
    fn lane_matches_cpu_comp_batch6() {
        use crate::indicators::average::lr::LinearRegressionMA;
        use crate::indicators::channels::regression_channel_width::RegressionChannelWidth;
        use crate::indicators::channels::regression_channels::{RegressionChannelMode, RegressionChannels};
        use crate::indicators::channels::standard_deviation_channels::{RegressionSource, StandardDeviationChannels, StandardDeviationMode};
        use crate::indicators::channels::stddev_channel_width::StdDevChannelWidth;

        let bars = bars(120);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let cols = |rows: Vec<Vec<f64>>| -> Vec<Vec<f64>> {
            (0..rows[0].len()).map(|k| rows.iter().map(|r| r[k]).collect()).collect()
        };
        let chk = |g: &[Vec<f32>], rows: Vec<Vec<f64>>| {
            let c = cols(rows);
            let r: Vec<&Vec<f64>> = c.iter().collect();
            assert_cols(g, &r);
        };
        let mut p = CubeParams::period(10);
        p.a = 2.0;
        for zl in [false, true] {
            let mut lp = p;
            lp.flag = zl as u32;
            let mut m = LinearRegressionMA::with_zero_lag(10, zl);
            chk(&run_cols(CubeFormula::LrCols, &bars, lp), close.iter().map(|c| { m.feed(*c); vec![m.line(), m.gradient(), m.intercept(), m.r2()] }).collect());
        }
        for (flag, mode) in [(0u32, RegressionChannelMode::Standard), (1, RegressionChannelMode::Percentage), (2, RegressionChannelMode::R2Weighted)] {
            let mut rp = p;
            rp.flag = flag;
            let mut m = RegressionChannels::new(10, 2.0, mode);
            chk(&run_cols(CubeFormula::RegChanCols, &bars, rp), close.iter().map(|c| { let (u, mi, l) = m.feed(*c); vec![u, mi, l] }).collect());
        }
        let mut m = RegressionChannelWidth::new(10, 2.0);
        assert_close(&run(CubeFormula::RegChanWidthComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        for (flag, mode) in [(0u32, StandardDeviationMode::Simple), (1, StandardDeviationMode::Population)] {
            let mut sp = p;
            sp.flag = flag;
            let mut m = StandardDeviationChannels::new_custom(10, 2.0, mode, RegressionSource::Close);
            chk(&run_cols(CubeFormula::StdDevChanCols, &bars, sp), close.iter().map(|c| { let (u, mi, l) = m.feed(*c); vec![u, mi, l] }).collect());
        }
        let mut m = StdDevChannelWidth::new(10, 2.0);
        assert_close(&run(CubeFormula::StdDevWidthComp, &bars, p), &cpu(&close, |v| m.feed(v)));
    }

    /// UNTESTED on GPU (no GPU on the authoring box): Kalman family and alpha-beta-gamma 1040..=1045.
    #[test]
    fn lane_matches_cpu_comp_batch7() {
        use crate::indicators::kalman::alpha_beta_gamma_filter::AlphaBetaGammaFilter;
        use crate::indicators::kalman::basic_kalman_filter::BasicKalmanFilter;
        use crate::indicators::kalman::kalman_regime_score::KalmanRegimeScore;
        use crate::indicators::kalman::kalman_slope_zscore::KalmanSlopeZscore;
        use crate::indicators::kalman::kalman_trend_slope::KalmanTrendSlope;
        use crate::indicators::kalman::rts_smoother::RtsSmoother;

        let bars = bars(200);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let mut p = CubeParams::period(12);
        p.a = 1.0;
        p.b = 0.05;
        p.c = 0.5;
        let mut m = BasicKalmanFilter::new(1.0, 0.05, 0.5);
        assert_close(&run(CubeFormula::KalmanComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        let mut ap = p;
        ap.flag = 1;
        let mut m = BasicKalmanFilter::new_adaptive(1.0, 0.05, 0.5);
        assert_close(&run(CubeFormula::KalmanComp, &bars, ap), &cpu(&close, |v| m.feed(v)));
        let mut m = RtsSmoother::new();
        assert_close(&run(CubeFormula::RtsComp, &bars, p), &cpu(&close, |v| m.feed(v)));

        let mut m = KalmanTrendSlope::new(1.0, 0.05, 0.5, 12);
        let (mut s, mut z) = (Vec::new(), Vec::new());
        for c in &close {
            let (a, b) = m.feed(*c);
            s.push(a);
            z.push(b);
        }
        assert_cols(&run_cols(CubeFormula::KslopeCols, &bars, p), &[&s, &z]);
        let mut m = KalmanRegimeScore::new(1.0, 0.05, 0.5, 12, 0.9);
        assert_close(&run(CubeFormula::KscrComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        let mut zp = p;
        zp.period = 24;
        let mut m = KalmanSlopeZscore::new(1.0, 0.05, 0.5, 24);
        assert_close(&run(CubeFormula::KslopezComp, &bars, zp), &cpu(&close, |v| m.feed(v)));

        let mut m = AlphaBetaGammaFilter::new(10);
        let (mut a, mut b, mut c) = (Vec::new(), Vec::new(), Vec::new());
        for v in &close {
            m.feed(*v);
            a.push(m.pos());
            b.push(m.vel());
            c.push(m.acc());
        }
        assert_cols(&run_cols(CubeFormula::AbgCols, &bars, CubeParams::period(10)), &[&a, &b, &c]);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): WGSL dispatcher, Decycler end to end.
    #[cfg(feature = "gpu-shader")]
    #[test]
    fn shader_decycler_matches_cpu() {
        use crate::indicators::signal_processing::decycler::Decycler;
        let bars = bars(120);
        let samples: Vec<GpuSample> = bars.iter().map(GpuSample::from).collect();
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        for period in [10.0f32, 30.0] {
            let gpu = super::super::shader_run::launch_shader(crate::engine::indicator_id::IndicatorId::Decyc, &samples, period)
                .expect("wgpu adapter");
            let mut m = Decycler::new(period as f64);
            assert_close(&gpu[0], &cpu(&close, |v| m.feed(v)));
        }
    }

    /// UNTESTED on GPU (no GPU on the authoring box): spectral family 1100..=1112.
    #[test]
    fn lane_matches_cpu_spectral_batch() {
        use crate::indicators::signal_processing::spectral_bandpower::SpectralBandpower;
        use crate::indicators::signal_processing::spectral_bandpower_ratio_hl::SpectralBandpowerRatioHL;
        use crate::indicators::signal_processing::spectral_bandwidth_feature::SpectralBandwidthFeature;
        use crate::indicators::signal_processing::spectral_centroid_feature::SpectralCentroidFeature;
        use crate::indicators::signal_processing::spectral_crest::SpectralCrest;
        use crate::indicators::signal_processing::spectral_energy_ratio::SpectralEnergyRatio;
        use crate::indicators::signal_processing::spectral_entropy::SpectralEntropy;
        use crate::indicators::signal_processing::spectral_flatness::SpectralFlatness;
        use crate::indicators::signal_processing::spectral_high_mid_power_ratio::SpectralHighMidPowerRatio;
        use crate::indicators::signal_processing::spectral_low_mid_power_ratio::SpectralLowMidPowerRatio;
        use crate::indicators::signal_processing::spectral_rolloff::SpectralRolloff;
        use crate::indicators::signal_processing::spectral_rolloff_95::SpectralRolloff95;
        use crate::indicators::signal_processing::spectral_slope::SpectralSlope;

        // 300 bars: enough for 2 * ws samples and the 8-spectrum average at window 32 and 64.
        let bars = bars(300);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        for w in [32usize, 64] {
            let mut p = CubeParams::period(w as u32);
            p.a = 0.2;
            p.b = 0.4;
            let mut m = SpectralFlatness::new(w);
            assert_close(&run(CubeFormula::SflatComp, &bars, p), &cpu(&close, |v| m.feed(v)));
            let mut m = SpectralSlope::new(w);
            assert_close(&run(CubeFormula::SslopeComp, &bars, p), &cpu(&close, |v| m.feed(v)));
            let mut m = SpectralBandpower::new(w, 0.2, 0.4);
            let (mut l, mut mi, mut h) = (Vec::new(), Vec::new(), Vec::new());
            for c in &close {
                let (a, b, d) = m.feed(*c);
                l.push(a);
                mi.push(b);
                h.push(d);
            }
            assert_cols(&run_cols(CubeFormula::SbpCols, &bars, p), &[&l, &mi, &h]);
            let mut m = SpectralBandpowerRatioHL::new(w, 0.2, 0.4);
            assert_close(&run(CubeFormula::SbprhlComp, &bars, p), &cpu(&close, |v| m.feed(v)));
            let mut m = SpectralBandwidthFeature::new(w);
            assert_close(&run(CubeFormula::SbwfComp, &bars, p), &cpu(&close, |v| m.feed(v)));
            let mut m = SpectralCentroidFeature::new(w);
            assert_close(&run(CubeFormula::ScfComp, &bars, p), &cpu(&close, |v| m.feed(v)));
            let mut m = SpectralCrest::new(w);
            assert_close(&run(CubeFormula::ScrestComp, &bars, p), &cpu(&close, |v| m.feed(v)));
            let mut m = SpectralEntropy::new(w);
            assert_close(&run(CubeFormula::SentComp, &bars, p), &cpu(&close, |v| m.feed(v)));
            let mut m = SpectralEnergyRatio::new(w, 0.2);
            assert_close(&run(CubeFormula::SerComp, &bars, p), &cpu(&close, |v| m.feed(v)));
            let mut m = SpectralHighMidPowerRatio::new(w, 0.2, 0.4);
            assert_close(&run(CubeFormula::ShmprComp, &bars, p), &cpu(&close, |v| m.feed(v)));
            let mut m = SpectralLowMidPowerRatio::new(w, 0.2, 0.4);
            assert_close(&run(CubeFormula::SlmprComp, &bars, p), &cpu(&close, |v| m.feed(v)));
            let mut rp = p;
            rp.a = 0.8;
            let mut m = SpectralRolloff::new(w, 0.8);
            assert_close(&run(CubeFormula::SrollComp, &bars, rp), &cpu(&close, |v| m.feed(v)));
            let mut m = SpectralRolloff95::new(w);
            assert_close(&run(CubeFormula::Sroll95Comp, &bars, p), &cpu(&close, |v| m.feed(v)));
        }
    }

    /// UNTESTED on GPU (no GPU on the authoring box): spectral post statistics 1120..=1136.
    #[test]
    fn lane_matches_cpu_spectral_post_batch() {
        use crate::indicators::signal_processing::spectral_crest_percentile::SpectralCrestPercentile;
        use crate::indicators::signal_processing::spectral_entropy_of_entropy::SpectralEntropyOfEntropy;
        use crate::indicators::signal_processing::spectral_entropy_rate::SpectralEntropyRate;
        use crate::indicators::signal_processing::spectral_flatness_percentile::SpectralFlatnessPercentile;
        use crate::indicators::signal_processing::spectral_flux_proxy::SpectralFluxProxy;
        use crate::indicators::signal_processing::spectral_rolloff_percentile::SpectralRolloffPercentile;
        use crate::indicators::signal_processing::spectral_rolloff_robust_percentile::SpectralRolloffRobustPercentile;
        use crate::indicators::signal_processing::spectral_slope_percentile::SpectralSlopePercentile;
        use crate::indicators::signal_processing::spectral_slope_robust_percentile::SpectralSlopeRobustPercentile;
        use crate::indicators::signal_processing::spectral_slope_zscore::SpectralSlopeZscore;
        use crate::indicators::signal_processing::stft_features::StftBandEnergyRatio;

        let bars = bars(400);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let mut p = CubeParams::period(32);
        p.slow = 50;
        p.a = 0.8;
        p.b = 0.3;
        let mut m = SpectralFlatnessPercentile::new(32, 50);
        assert_close(&run(CubeFormula::SflatpComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        let mut m = SpectralRolloffPercentile::new(32, 50, 0.8);
        assert_close(&run(CubeFormula::SrollpComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        let mut m = SpectralRolloffRobustPercentile::new(32, 50, 0.8);
        assert_close(&run(CubeFormula::SrollrpComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        let mut m = SpectralSlopePercentile::new(32, 50);
        assert_close(&run(CubeFormula::SslopepComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        let mut m = SpectralSlopeRobustPercentile::new(32, 50);
        assert_close(&run(CubeFormula::SsloperpComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        let mut m = SpectralSlopeZscore::new(32, 50);
        assert_close(&run(CubeFormula::SslopezComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        let mut m = SpectralCrestPercentile::new(32, 50);
        assert_close(&run(CubeFormula::ScrestpComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        let mut m = SpectralEntropyOfEntropy::new(32, 50);
        assert_close(&run(CubeFormula::SententComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        let mut ap = p;
        ap.a = 0.3;
        let mut m = SpectralEntropyRate::new(32, 0.3);
        assert_close(&run(CubeFormula::SentrComp, &bars, ap), &cpu(&close, |v| m.feed(v)));
        let mut m = SpectralFluxProxy::new(32, 0.8, 0.3);
        assert_close(&run(CubeFormula::SfluxComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        let mut sp = CubeParams::period(16);
        sp.slow = 4;
        let mut m = StftBandEnergyRatio::new(16, 4);
        assert_close(&run(CubeFormula::StftComp, &bars, sp), &cpu(&close, |v| m.feed(v)));
    }

    /// UNTESTED on GPU (no GPU on the authoring box): Butterworth, Savitzky-Golay, roofing 1050..=1052.
    #[test]
    fn lane_matches_cpu_filter_batch() {
        use crate::indicators::signal_processing::butterworth::{ButterworthFilter, FilterType};
        use crate::indicators::signal_processing::roofing_filter::RoofingFilter;
        use crate::indicators::signal_processing::savitzky_golay::{DerivativeOrder, SavitzkyGolayFilter};

        let bars = bars(160);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        for (code, ft) in [(0u32, FilterType::LowPass), (1, FilterType::HighPass)] {
            let mut p = CubeParams::period(1);
            p.ext[0] = code;
            p.ext[1] = 3;
            p.a = 0.1;
            p.b = 1.0;
            let mut m = ButterworthFilter::new(ft, 3, 0.1, 1.0);
            assert_close(&run(CubeFormula::ButterComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        }
        for (flag, d) in [(0u32, DerivativeOrder::Smoothing), (1, DerivativeOrder::FirstDerivative)] {
            let mut p = CubeParams::period(11);
            p.slow = 3;
            p.flag = flag;
            let mut m = SavitzkyGolayFilter::new(11, 3, d);
            assert_close(&run(CubeFormula::SgComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        }
        let mut p = CubeParams::period(1);
        p.a = 48.0;
        p.b = 10.0;
        let mut m = RoofingFilter::new_with_periods(48.0, 10.0);
        assert_close(&run(CubeFormula::RoofComp, &bars, p), &cpu(&close, |v| m.feed(v)));
        let mut ap = p;
        ap.flag = 1;
        ap.a = 0.05;
        ap.b = 0.3;
        let mut m = RoofingFilter::new(0.05, 0.3);
        assert_close(&run(CubeFormula::RoofComp, &bars, ap), &cpu(&close, |v| m.feed(v)));
    }

    /// UNTESTED on GPU (no GPU on the authoring box): composites 1053..=1054.
    #[test]
    fn lane_matches_cpu_comp_batch8() {
        use crate::indicators::channels::trima_bands::TrimaBands;
        use crate::indicators::volatility::kp::Kp;

        let bars = bars(140);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let lanes: Vec<[f64; 4]> = bars.iter().map(|b| [b.high, b.low, b.close, b.volume]).collect();
        let mut p = CubeParams::period(10);
        p.a = 2.0;
        let mut m = TrimaBands::new(10, 2.0);
        let (mut u, mut mi, mut l) = (Vec::new(), Vec::new(), Vec::new());
        for c in &close {
            let (a, b, d) = m.feed(*c);
            u.push(a);
            mi.push(b);
            l.push(d);
        }
        assert_cols(&run_cols(CubeFormula::TrimaBandsCols, &bars, p), &[&u, &mi, &l]);
        let mut kp = p;
        kp.smoother = CubeSmoother::Rma;
        let mut m = Kp::new(10, 2.0);
        assert_close(&run(CubeFormula::KpComp, &bars, kp), &lanes.iter().map(|x| m.feed(x)).collect::<Vec<f64>>());
    }

    /// UNTESTED on GPU (no GPU on the authoring box): composites 1055..=1059.
    #[test]
    fn lane_matches_cpu_comp_batch9() {
        use crate::indicators::momentum::pressure::Pressure;
        use crate::indicators::trend::gann_hilo_activator::GannHiLoActivator;
        use crate::indicators::trend_stop::atr_trailing_stop::ATRTrailingStop;
        use crate::indicators::trend_stop::chande_kroll_stop::ChandeKrollStop;
        use crate::indicators::trend_stop::chandelier_stop::ChandelierStop;

        let bars = bars(140);
        let lanes: Vec<[f64; 4]> = bars.iter().map(|b| [b.high, b.low, b.close, b.volume]).collect();
        let mut p = CubeParams::period(10);
        p.a = 2.0;
        p.slow = 7;
        p.smoother = CubeSmoother::Sma;
        p.smoother2 = CubeSmoother::Rma;
        let mut m = ChandelierStop::with_params(10, 2.0);
        assert_close(&run(CubeFormula::ChandComp, &bars, p), &lanes.iter().map(|x| { m.feed(x); m.value() }).collect::<Vec<f64>>());
        let mut m = ChandeKrollStop::new(10, 2.0, 7, 7);
        assert_close(&run(CubeFormula::CksComp, &bars, p), &lanes.iter().map(|x| { m.feed(x); m.value() }).collect::<Vec<f64>>());
        let mut m = ATRTrailingStop::with_params(10, 2.0);
        assert_close(&run(CubeFormula::AtrtsComp, &bars, p), &lanes.iter().map(|x| { m.feed(x); m.value() }).collect::<Vec<f64>>());
        let mut m = GannHiLoActivator::from_smoother(10, SmootherId::Sma);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for x in &lanes {
            let (u, s) = m.feed(x);
            a.push(u);
            b.push(s);
        }
        assert_cols(&run_cols(CubeFormula::GannHiloCols, &bars, p), &[&a, &b]);
        let mut m = Pressure::from_smoothers(10, SmootherId::Sma, SmootherId::Rma);
        assert_close(&run(CubeFormula::PressureComp, &bars, p), &lanes.iter().map(|x| m.feed(x)).collect::<Vec<f64>>());
    }

    /// UNTESTED on GPU (no GPU on the authoring box): L3, tick-window and vol-index event rows 940..=948, MacdHistZ 1060.
    #[test]
    fn lane_matches_cpu_event_batch2() {
        use super::super::event_frame::GpuEventFrame;
        use super::super::kernels_ev::launch_cube_events;
        use crate::core::types::{L3Action, OrderBookSide, OrderbookL3Event, Tick, VolatilityIndex};
        use crate::engine::streams::orderbook_l3_consumer::OrderbookL3Consumer;
        use crate::engine::streams::tick_consumer::TickConsumer;
        use crate::engine::streams::volatility_index_consumer::VolatilityIndexConsumer;
        use crate::indicators::clusters::trade_cluster_detector::TradeClusterDetector;
        use crate::indicators::microstructure::l3_cancel_ratio::L3CancelRatio;
        use crate::indicators::microstructure::l3_large_order_tracker::L3LargeOrderTracker;
        use crate::indicators::microstructure::l3_order_rate::L3OrderRate;
        use crate::indicators::microstructure::l3_spoofer_score::L3SpooferScore;
        use crate::indicators::momentum::macd_hist_zscore::MacdHistZscore;
        use crate::indicators::tick_advanced::volume_imbalance_zone::VolumeImbalanceZone;
        use crate::indicators::tick_advanced::vwap_deviation::VwapDeviation;
        use crate::indicators::volatility_advanced::vol_idx_spike::VolIdxSpike;

        let n = 80usize;
        let w = |i: usize, k: f64| ((i as f64 * k).sin() * 0.5 + 0.5);
        let t0 = 1_700_000_000_000i64;
        let l3: Vec<OrderbookL3Event> = (0..n)
            .map(|i| OrderbookL3Event {
                side: if w(i, 0.9) > 0.5 { OrderBookSide::Bid } else { OrderBookSide::Ask },
                order_id: i.to_string(),
                price: 100.0 + w(i, 0.3),
                quantity: 0.5 + 3.0 * w(i, 0.7) * w(i, 0.11),
                action: match (w(i, 1.3) * 3.0) as u32 { 0 => L3Action::Add, 1 => L3Action::Modify, _ => L3Action::Delete },
                timestamp: t0 + i as i64 * 120,
            })
            .collect();
        let fr = GpuEventFrame::from_l3(&l3);
        let pp = |p: u32, a: f32, b: f32| { let mut c = CubeParams::period(p); c.a = a; c.b = b; c };
        let mut m = L3CancelRatio::new(12);
        let c: Vec<f64> = l3.iter().map(|e| { m.update_orderbook_l3(e); m.value() }).collect();
        assert_close(&launch_cube_events(CubeFormula::L3CancelRatioEv, &fr, pp(12, 0.0, 0.0))[0], &c);
        let mut m = L3OrderRate::new(1500);
        let c: Vec<f64> = l3.iter().map(|e| { m.update_orderbook_l3(e); m.value() }).collect();
        assert_close(&launch_cube_events(CubeFormula::L3OrderRateEv, &fr, pp(1, 1500.0, 0.0))[0], &c);
        let mut m = L3SpooferScore::new(10, 2.0);
        let c: Vec<f64> = l3.iter().map(|e| { m.update_orderbook_l3(e); m.value() }).collect();
        assert_close(&launch_cube_events(CubeFormula::L3SpooferScoreEv, &fr, pp(10, 2.0, 0.0))[0], &c);
        let mut m = L3LargeOrderTracker::new(10, 1.5);
        let (mut s, mut z, mut p) = (Vec::new(), Vec::new(), Vec::new());
        for e in &l3 {
            m.update_orderbook_l3(e);
            s.push(m.side());
            z.push(m.size());
            p.push(m.price());
        }
        assert_cols(&launch_cube_events(CubeFormula::L3LargeOrderEv, &fr, pp(10, 1.5, 0.0)), &[&s, &z, &p]);
        assert_eq!(launch_cube_events(CubeFormula::AuctionPriceDeviationEv, &fr, pp(1, 0.0, 0.0))[0], vec![0.0; n]);

        let ticks: Vec<Tick> = (0..n)
            .map(|i| Tick::new(t0 + i as i64 * 100, 100.0 + 3.0 * w(i, 0.3), 1.0 + w(i, 0.1), w(i, 1.3) > 0.45))
            .collect();
        let fr = GpuEventFrame::from_ticks(&ticks);
        let mut m = TradeClusterDetector::new(0.5, 3, 1000);
        let (mut a, mut b, mut cc) = (Vec::new(), Vec::new(), Vec::new());
        for t in &ticks {
            m.update_tick(t);
            a.push(m.signal());
            b.push(m.cluster_price());
            cc.push(m.cluster_size());
        }
        assert_cols(&launch_cube_events(CubeFormula::TradeClusterEv, &fr, pp(3, 0.5, 1000.0)), &[&a, &b, &cc]);
        let mut m = VolumeImbalanceZone::new(1000, 0.3);
        let (mut a, mut b, mut cc) = (Vec::new(), Vec::new(), Vec::new());
        for t in &ticks {
            m.update_tick(t);
            a.push(m.side());
            b.push(m.low());
            cc.push(m.high());
        }
        assert_cols(&launch_cube_events(CubeFormula::VolImbZoneEv, &fr, pp(1, 1000.0, 0.3)), &[&a, &b, &cc]);
        let mut m = VwapDeviation::new(1000);
        let (mut a, mut b, mut cc) = (Vec::new(), Vec::new(), Vec::new());
        for t in &ticks {
            m.update_tick(t);
            a.push(m.price());
            b.push(m.vwap());
            cc.push(m.deviation());
        }
        assert_cols(&launch_cube_events(CubeFormula::VwapDevEv, &fr, pp(1, 1000.0, 0.0)), &[&a, &b, &cc]);

        let vis: Vec<VolatilityIndex> = (0..n)
            .map(|i| { let mut v = VolatilityIndex::default(); v.value = 20.0 + 10.0 * w(i, 0.37); v.timestamp = t0 + i as i64 * 1000; v })
            .collect();
        let fr = GpuEventFrame::from_vol_index(&vis);
        let mut m = VolIdxSpike::new(15);
        let c: Vec<f64> = vis.iter().map(|v| { m.update_volatility_index(v); m.value() }).collect();
        assert_close(&launch_cube_events(CubeFormula::VolIdxSpikeEv, &fr, pp(15, 0.0, 0.0))[0], &c);

        let bars = bars(120);
        let mut mp = CubeParams::period(10);
        mp.fast = 4;
        mp.slow = 9;
        mp.signal = 3;
        mp.smoother = CubeSmoother::Ema;
        mp.smoother2 = CubeSmoother::Ema;
        mp.smoother3 = CubeSmoother::Ema;
        let mut m = MacdHistZscore::new(4, 9, 3, 10);
        let c: Vec<f64> = bars.iter().map(|b| m.feed(b.close)).collect();
        assert_close(&run(CubeFormula::MacdHistZComp, &bars, mp), &c);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): order-book snapshot rows 960..=966.
    #[test]
    fn lane_matches_cpu_book_batch() {
        use super::super::book_frame::GpuBookFrame;
        use super::super::kernels_book::launch_cube_book;
        use crate::core::types::OrderBook;
        use crate::engine::streams::order_book_consumer::OrderBookConsumer;
        use crate::indicators::book::book_depth_change::BookDepthChange;
        use crate::indicators::book::liquidity_sweep::LiquiditySweep;
        use crate::indicators::book::wall_detector::WallDetector;
        use crate::indicators::book_advanced::best_level_volatility::BestLevelVolatility;
        use crate::indicators::book_advanced::bid_ask_bounce_rate::BidAskBounceRate;
        use crate::indicators::book_advanced::mid_price_velocity::MidPriceVelocity;
        use crate::indicators::book_advanced::price_level_density::PriceLevelDensity;

        let n = 70usize;
        let w = |i: usize, k: f64| ((i as f64 * k).sin() * 0.5 + 0.5);
        let t0 = 1_700_000_000_000i64;
        let books: Vec<OrderBook> = (0..n)
            .map(|i| {
                let mid = 100.0 + 2.0 * w(i, 0.21);
                let kb = 4 + (w(i, 0.8) * 4.0) as usize;
                let ka = 4 + (w(i, 0.6) * 4.0) as usize;
                let bids = (0..kb).map(|l| (((mid - 0.1 * (l as f64 + 1.0)) * 10.0).round() / 10.0, 1.0 + 5.0 * w(i + l, 0.9))).collect();
                let asks = (0..ka).map(|l| (((mid + 0.1 * (l as f64 + 1.0)) * 10.0).round() / 10.0, 1.0 + 5.0 * w(i + l, 1.1))).collect();
                OrderBook::simple(bids, asks, t0 + i as i64 * 200)
            })
            .collect();
        let fr = GpuBookFrame::from_books(&books, 16);
        let mut p = CubeParams::period(8);
        p.levels = 3;
        p.a = 90.0;
        let mut m = BidAskBounceRate::new(8);
        let c: Vec<f64> = books.iter().map(|b| { m.update_orderbook(b); m.value() }).collect();
        assert_close(&launch_cube_book(CubeFormula::BidAskBounceBk, &fr, p)[0], &c);
        let mut m = MidPriceVelocity::new(8);
        let c: Vec<f64> = books.iter().map(|b| { m.update_orderbook(b); m.value() }).collect();
        assert_close(&launch_cube_book(CubeFormula::MidPriceVelBk, &fr, p)[0], &c);
        let mut m = BookDepthChange::new(8);
        let (mut a, mut b2) = (Vec::new(), Vec::new());
        for b in &books {
            m.update_orderbook(b);
            a.push(m.bid());
            b2.push(m.ask());
        }
        assert_cols(&launch_cube_book(CubeFormula::BookDepthChangeBk, &fr, p), &[&a, &b2]);
        let mut m = WallDetector::new(30, 90.0, 3);
        let (mut a, mut b2, mut c3) = (Vec::new(), Vec::new(), Vec::new());
        for b in &books {
            m.update_orderbook(b);
            a.push(m.bid_price());
            b2.push(m.ask_price());
            c3.push(m.total_size());
        }
        let mut wp = p;
        wp.period = 30;
        assert_cols(&launch_cube_book(CubeFormula::WallDetectorBk, &fr, wp), &[&a, &b2, &c3]);
        let mut m = BestLevelVolatility::new(8);
        let (mut a, mut b2, mut c3) = (Vec::new(), Vec::new(), Vec::new());
        for b in &books {
            m.update_orderbook(b);
            a.push(m.std_bid());
            b2.push(m.std_ask());
            c3.push(m.max());
        }
        assert_cols(&launch_cube_book(CubeFormula::BestLevelVolBk, &fr, p), &[&a, &b2, &c3]);
        let mut m = PriceLevelDensity::new(5);
        let (mut a, mut b2, mut c3) = (Vec::new(), Vec::new(), Vec::new());
        for b in &books {
            m.update_orderbook(b);
            a.push(m.density_bid());
            b2.push(m.density_ask());
            c3.push(m.avg());
        }
        let mut dp = p;
        dp.period = 5;
        assert_cols(&launch_cube_book(CubeFormula::PriceLevelDensityBk, &fr, dp), &[&a, &b2, &c3]);
        let mut m = LiquiditySweep::new();
        let (mut a, mut b2) = (Vec::new(), Vec::new());
        for b in &books {
            m.update_orderbook(b);
            a.push(m.direction());
            b2.push(m.magnitude());
        }
        assert_cols(&launch_cube_book(CubeFormula::LiquiditySweepBk, &fr, p), &[&a, &b2]);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): tick / funding / mark event rows 949..=956.
    #[test]
    fn lane_matches_cpu_event_batch3() {
        use super::super::event_frame::GpuEventFrame;
        use super::super::kernels_ev::launch_cube_events;
        use crate::core::types::{FundingRate, MarkPrice, Tick};
        use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
        use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
        use crate::engine::streams::tick_consumer::TickConsumer;
        use crate::indicators::clusters::tick_volume_analyzer::TickVolumeAnalyzer;
        use crate::indicators::composites::adaptive_threshold::AdaptiveThreshold;
        use crate::indicators::funding_advanced::funding_extreme_alert::FundingExtremeAlert;
        use crate::indicators::funding_advanced::funding_momentum::FundingMomentum;
        use crate::indicators::mark_price_advanced::index_price_momentum::IndexPriceMomentum;
        use crate::indicators::mark_price_advanced::mark_price_gap_detector::MarkPriceGapDetector;
        use crate::indicators::volume::trade_flow_imbalance::TradeFlowImbalance;
        use crate::indicators::volume::uptick_downtick_volume::UptickDowntickVolume;

        let n = 80usize;
        let w = |i: usize, k: f64| ((i as f64 * k).sin() * 0.5 + 0.5);
        let t0 = 1_700_000_000_000i64;
        let pp = |p: u32, a: f32| { let mut c = CubeParams::period(p); c.a = a; c };
        let ticks: Vec<Tick> = (0..n)
            .map(|i| Tick::new(t0 + i as i64 * 100, 100.0 + 3.0 * w(i, 0.3), 1.0 + w(i, 0.1), w(i, 1.3) > 0.45))
            .collect();
        let fr = GpuEventFrame::from_ticks(&ticks);
        let mut m = TickVolumeAnalyzer::new(10);
        let c: Vec<f64> = ticks.iter().map(|t| { m.update(t); m.value() }).collect();
        assert_close(&launch_cube_events(CubeFormula::TickVolumeEv, &fr, pp(10, 0.0))[0], &c);
        let mut m = TradeFlowImbalance::new(9);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for t in &ticks {
            m.update_tick(t);
            a.push(m.imbalance());
            b.push(m.volume());
        }
        assert_cols(&launch_cube_events(CubeFormula::TradeFlowImbEv, &fr, pp(9, 0.0)), &[&a, &b]);
        let mut m = UptickDowntickVolume::new(9);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for t in &ticks {
            m.update_tick(t);
            a.push(m.uptick());
            b.push(m.downtick());
        }
        assert_cols(&launch_cube_events(CubeFormula::UpDownTickVolEv, &fr, pp(9, 0.0)), &[&a, &b]);
        let mut m = AdaptiveThreshold::new(12, 1.5);
        let (mut a, mut b, mut c3) = (Vec::new(), Vec::new(), Vec::new());
        for t in &ticks {
            m.update_tick(t);
            a.push(m.mean());
            b.push(m.std());
            c3.push(m.threshold());
        }
        assert_cols(&launch_cube_events(CubeFormula::AdaptiveThresholdEv, &fr, pp(12, 1.5)), &[&a, &b, &c3]);

        let fund: Vec<FundingRate> = (0..n)
            .map(|i| FundingRate { rate: (w(i, 0.9) - 0.5) * 0.002, timestamp: t0 + i as i64 * 1000, ..Default::default() })
            .collect();
        let fr = GpuEventFrame::from_funding(&fund);
        let mut m = FundingExtremeAlert::new(10, 1.2);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for f in &fund {
            m.update_funding(f);
            a.push(m.signal());
            b.push(m.magnitude());
        }
        assert_cols(&launch_cube_events(CubeFormula::FundingExtremeEv, &fr, pp(10, 1.2)), &[&a, &b]);
        let mut m = FundingMomentum::new(6);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for f in &fund {
            m.update_funding(f);
            a.push(m.ema());
            b.push(m.slope());
        }
        assert_cols(&launch_cube_events(CubeFormula::FundingMomEv, &fr, pp(6, 0.0)), &[&a, &b]);

        let marks: Vec<MarkPrice> = (0..n)
            .map(|i| MarkPrice { mark_price: 100.0 + 3.0 * w(i, 0.5) + if i % 17 == 0 { 4.0 } else { 0.0 }, index_price: if i % 5 == 0 { None } else { Some(100.0 + 3.0 * w(i, 0.45)) }, timestamp: t0 + i as i64 * 1000, ..Default::default() })
            .collect();
        let fr = GpuEventFrame::from_mark(&marks);
        let mut m = IndexPriceMomentum::new(6);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for k in &marks {
            m.update_mark(k);
            a.push(m.ema());
            b.push(m.slope());
        }
        assert_cols(&launch_cube_events(CubeFormula::IndexPriceMomEv, &fr, pp(6, 0.0)), &[&a, &b]);
        let mut m = MarkPriceGapDetector::new(10, 2.0);
        let (mut a, mut b, mut c3) = (Vec::new(), Vec::new(), Vec::new());
        for k in &marks {
            m.update_mark(k);
            a.push(m.signal());
            b.push(m.jump_size());
            c3.push(m.sigma_ratio());
        }
        assert_cols(&launch_cube_events(CubeFormula::MarkGapEv, &fr, pp(10, 2.0)), &[&a, &b, &c3]);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): rows 957..=959 (events) and 967..=969 (books).
    #[test]
    fn lane_matches_cpu_event_book_batch4() {
        use super::super::book_frame::GpuBookFrame;
        use super::super::event_frame::GpuEventFrame;
        use super::super::kernels_book::launch_cube_book;
        use super::super::kernels_ev::launch_cube_events;
        use crate::core::types::{AggTrade, Liquidation, OrderBook, Tick};
        use crate::engine::streams::agg_trade_consumer::AggTradeConsumer;
        use crate::engine::streams::liquidation_consumer::LiquidationConsumer;
        use crate::engine::streams::order_book_consumer::OrderBookConsumer;
        use crate::engine::streams::tick_consumer::TickConsumer;
        use crate::indicators::book::order_book_velocity::OrderBookVelocity;
        use crate::indicators::book::spread_distribution::SpreadDistribution;
        use crate::indicators::book_advanced::layer_concentration::LayerConcentration;
        use crate::indicators::liquidations::liquidation_cluster_detector::LiquidationClusterDetector;
        use crate::indicators::sentiment::agg_trade_size_distribution::AggTradeSizeDistribution;
        use crate::indicators::tick_advanced::large_trade_filter::LargeTradeFilter;

        let n = 80usize;
        let w = |i: usize, k: f64| ((i as f64 * k).sin() * 0.5 + 0.5);
        let t0 = 1_700_000_000_000i64;
        let pp = |p: u32, a: f32, b: f32| { let mut c = CubeParams::period(p); c.a = a; c.b = b; c };
        let ticks: Vec<Tick> = (0..n)
            .map(|i| Tick::new(t0 + i as i64 * 100, 100.0 + 3.0 * w(i, 0.3), 0.5 + 3.0 * w(i, 0.7) * w(i, 0.13), w(i, 1.3) > 0.45))
            .collect();
        let fr = GpuEventFrame::from_ticks(&ticks);
        let mut m = LargeTradeFilter::new(10, 1.3);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for t in &ticks {
            m.update_tick(t);
            a.push(m.signal());
            b.push(m.ratio());
        }
        assert_cols(&launch_cube_events(CubeFormula::LargeTradeFilterEv, &fr, pp(10, 1.3, 0.0)), &[&a, &b]);

        let aggs: Vec<AggTrade> = (0..n)
            .map(|i| AggTrade { price: 100.0 + w(i, 0.3), quantity: 0.2 + 5.0 * w(i, 0.77), timestamp: t0 + i as i64 * 100, is_buy: w(i, 1.1) > 0.5, ..Default::default() })
            .collect();
        let fr = GpuEventFrame::from_agg_trades(&aggs);
        let mut m = AggTradeSizeDistribution::new(12);
        let (mut a, mut b, mut c3) = (Vec::new(), Vec::new(), Vec::new());
        for t in &aggs {
            m.update_agg_trade(t);
            a.push(m.median());
            b.push(m.p95());
            c3.push(m.current());
        }
        assert_cols(&launch_cube_events(CubeFormula::AggSizeDistEv, &fr, pp(12, 0.0, 0.0)), &[&a, &b, &c3]);

        let liqs: Vec<Liquidation> = (0..n)
            .map(|i| Liquidation { price: 100.0 + 2.0 * w(i, 0.3), quantity: 1.0 + w(i, 0.5), timestamp: t0 + i as i64 * 100, ..Default::default() })
            .collect();
        let fr = GpuEventFrame::from_liquidations(&liqs);
        let mut m = LiquidationClusterDetector::new(0.5, 1000, 3);
        let (mut a, mut b, mut c3) = (Vec::new(), Vec::new(), Vec::new());
        for t in &liqs {
            m.update_liquidation(t);
            a.push(m.price());
            b.push(m.count());
            c3.push(m.volume());
        }
        assert_cols(&launch_cube_events(CubeFormula::LiqClusterEv, &fr, pp(3, 1000.0, 0.5)), &[&a, &b, &c3]);

        let books: Vec<OrderBook> = (0..n)
            .map(|i| {
                let mid = 100.0 + 2.0 * w(i, 0.21);
                let kb = 4 + (w(i, 0.8) * 4.0) as usize;
                let ka = 4 + (w(i, 0.6) * 4.0) as usize;
                let bids = (0..kb).map(|l| (((mid - 0.1 * (l as f64 + 1.0)) * 10.0).round() / 10.0, 1.0 + 5.0 * w(i / 2 + l, 0.9))).collect();
                let asks = (0..ka).map(|l| (((mid + 0.1 * (l as f64 + 1.0)) * 10.0).round() / 10.0, 1.0 + 5.0 * w(i / 2 + l, 1.1))).collect();
                OrderBook::simple(bids, asks, t0 + i as i64 * 200)
            })
            .collect();
        let fr = GpuBookFrame::from_books(&books, 16);
        let mut m = SpreadDistribution::new(9);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for k in &books {
            m.update_orderbook(k);
            a.push(m.spread());
            b.push(m.percentile());
        }
        assert_cols(&launch_cube_book(CubeFormula::SpreadDistributionBk, &fr, CubeParams::period(9)), &[&a, &b]);
        let mut m = LayerConcentration::new(5);
        let (mut a, mut b, mut c3) = (Vec::new(), Vec::new(), Vec::new());
        for k in &books {
            m.update_orderbook(k);
            a.push(m.gini_bid());
            b.push(m.gini_ask());
            c3.push(m.max());
        }
        assert_cols(&launch_cube_book(CubeFormula::LayerConcentrationBk, &fr, CubeParams::period(5)), &[&a, &b, &c3]);
        let mut m = OrderBookVelocity::new(6);
        let c: Vec<f64> = books.iter().map(|k| { m.update_orderbook(k); m.value() }).collect();
        assert_close(&launch_cube_book(CubeFormula::OrderBookVelocityBk, &fr, CubeParams::period(6))[0], &c);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): delta and basis event rows 980..=983.
    #[test]
    fn lane_matches_cpu_event_batch5() {
        use super::super::event_frame::GpuEventFrame;
        use super::super::kernels_ev::launch_cube_events;
        use crate::core::types::{Basis, OrderBookLevel, OrderbookDelta};
        use crate::engine::streams::basis_consumer::BasisConsumer;
        use crate::engine::streams::orderbook_delta_consumer::OrderbookDeltaConsumer;
        use crate::indicators::book::book_churn_rate::BookChurnRate;
        use crate::indicators::book::level_replenishment_rate::LevelReplenishmentRate;
        use crate::indicators::index_basis::basis_extreme::BasisExtreme;
        use crate::indicators::microstructure::quote_stuffing_detector::QuoteStuffingDetector;

        let n = 80usize;
        let w = |i: usize, k: f64| ((i as f64 * k).sin() * 0.5 + 0.5);
        let t0 = 1_700_000_000_000i64;
        let pp = |p: u32, a: f32, b: f32| { let mut c = CubeParams::period(p); c.a = a; c.b = b; c };
        let deltas: Vec<OrderbookDelta> = (0..n)
            .map(|i| {
                let kb = (w(i, 0.9) * 4.0) as usize;
                let ka = (w(i, 0.5) * 4.0) as usize;
                OrderbookDelta {
                    bids: (0..kb).map(|l| OrderBookLevel::new(100.0 - l as f64, if (i + l) % 3 == 0 { 0.0 } else { 1.0 })).collect(),
                    asks: (0..ka).map(|l| OrderBookLevel::new(101.0 + l as f64, if (i + l) % 4 == 0 { 0.0 } else { 2.0 })).collect(),
                    timestamp: t0 + i as i64 * 150 + (i as i64 % 3) * 40,
                    ..Default::default()
                }
            })
            .collect();
        let fr = GpuEventFrame::from_deltas(&deltas);
        let mut m = BookChurnRate::new(7);
        let c: Vec<f64> = deltas.iter().map(|d| { m.update_delta(d); m.value() }).collect();
        assert_close(&launch_cube_events(CubeFormula::BookChurnEv, &fr, pp(7, 0.0, 0.0))[0], &c);
        let mut m = LevelReplenishmentRate::new(9);
        let c: Vec<f64> = deltas.iter().map(|d| { m.update_delta(d); m.value() }).collect();
        assert_close(&launch_cube_events(CubeFormula::LevelReplenishEv, &fr, pp(9, 0.0, 0.0))[0], &c);
        let mut m = QuoteStuffingDetector::new(1000, 5.0);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for d in &deltas {
            m.update_delta(d);
            a.push(m.rate());
            b.push(m.signal());
        }
        assert_cols(&launch_cube_events(CubeFormula::QuoteStuffingEv, &fr, pp(1, 1000.0, 5.0)), &[&a, &b]);

        let basis: Vec<Basis> = (0..n)
            .map(|i| Basis { basis: 10.0 * w(i, 0.37) - 5.0, timestamp: t0 + i as i64 * 1000, ..Default::default() })
            .collect();
        let fr = GpuEventFrame::from_basis(&basis);
        let mut m = BasisExtreme::new(15);
        let c: Vec<f64> = basis.iter().map(|b| { m.update_basis(b); m.value() }).collect();
        assert_close(&launch_cube_events(CubeFormula::BasisExtremeEv, &fr, pp(15, 0.0, 0.0))[0], &c);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): merged multi-stream rows 1070..=1077.
    #[test]
    fn lane_matches_cpu_merged_batch() {
        use super::super::event_frame::{GpuEventFrame, MergedStream};
        use super::super::kernels_ev::launch_cube_events;
        use crate::core::types::{
            CompositeIndex, FundingRate, HistoricalVolatility, IndexPrice, LongShortRatio, MarkPrice,
            OpenInterest, PredictedFunding, Ticker, VolatilityIndex,
        };
        use crate::engine::streams::composite_index_consumer::CompositeIndexConsumer;
        use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
        use crate::engine::streams::historical_volatility_consumer::HistoricalVolatilityConsumer;
        use crate::engine::streams::index_price_consumer::IndexPriceConsumer;
        use crate::engine::streams::long_short_ratio_consumer::LongShortRatioConsumer;
        use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
        use crate::engine::streams::open_interest_consumer::OpenInterestConsumer;
        use crate::engine::streams::predicted_funding_consumer::PredictedFundingConsumer;
        use crate::engine::streams::ticker_consumer::TickerConsumer;
        use crate::engine::streams::volatility_index_consumer::VolatilityIndexConsumer;
        use crate::indicators::composites::funding_oi_pressure::FundingOiPressure;
        use crate::indicators::composites::funding_sentiment_alignment::FundingSentimentAlignment;
        use crate::indicators::composites::index_tracking_error::IndexTrackingError;
        use crate::indicators::composites::iv_hv_spread::IvHvSpread;
        use crate::indicators::funding_advanced::funding_drift::FundingDrift;
        use crate::indicators::funding_advanced::funding_price_divergence::FundingPriceMomentumDivergence;
        use crate::indicators::mark_price_advanced::mark_price_vs_last::MarkPriceVsLast;
        use crate::indicators::open_interest::long_squeeze_detector::LongSqueezeDetector;

        let n = 40usize;
        let w = |i: usize, k: f64| ((i as f64 * k).sin() * 0.5 + 0.5);
        let t0 = 1_700_000_000_000i64;
        let pp = |p: u32, b: f32| { let mut c = CubeParams::period(p); c.b = b; c };
        let fund: Vec<FundingRate> = (0..n).map(|i| FundingRate { rate: (w(i, 0.9) - 0.5) * 0.002, timestamp: t0 + i as i64 * 1000, ..Default::default() }).collect();
        let pred: Vec<PredictedFunding> = (0..n).map(|i| PredictedFunding { predicted_rate: (w(i, 0.7) - 0.5) * 0.002, next_funding_time: 0, timestamp: t0 + i as i64 * 1300 + 50 }).collect();
        let oi: Vec<OpenInterest> = (0..n).map(|i| OpenInterest { open_interest: 1000.0 + 100.0 * w(i, 0.6), timestamp: t0 + i as i64 * 1700 + 20, ..Default::default() }).collect();
        let marks: Vec<MarkPrice> = (0..n).map(|i| MarkPrice { mark_price: 100.0 + 3.0 * w(i, 0.5), timestamp: t0 + i as i64 * 900 + 10, ..Default::default() }).collect();
        let lsr: Vec<LongShortRatio> = (0..n).map(|i| LongShortRatio { long_ratio: 0.3 + 0.4 * w(i, 0.8), timestamp: t0 + i as i64 * 2100 + 30, ..Default::default() }).collect();
        let sf = MergedStream::scalar(&fund, |f| f.timestamp, |f| f.rate);
        let sp = MergedStream::scalar(&pred, |f| f.timestamp, |f| f.predicted_rate);
        let so = MergedStream::scalar(&oi, |f| f.timestamp, |f| f.open_interest);
        let sm = MergedStream::scalar(&marks, |f| f.timestamp, |f| f.mark_price);
        let sl = MergedStream::scalar(&lsr, |f| f.timestamp, |f| f.long_ratio);

        // Replay helper: merged order = time, then stream order (the frame's own order).
        enum Ev<'a> { F(&'a FundingRate), P(&'a PredictedFunding), O(&'a OpenInterest), M(&'a MarkPrice), L(&'a LongShortRatio) }
        let order = |streams: Vec<(i64, usize, usize)>| { let mut v = streams; v.sort(); v };
        let mk = |lists: Vec<Vec<i64>>| -> Vec<(i64, usize, usize)> {
            order(lists.iter().enumerate().flat_map(|(s, l)| l.iter().enumerate().map(move |(k, t)| (*t, s, k))).collect())
        };
        let ts_f: Vec<i64> = fund.iter().map(|f| f.timestamp).collect();
        let ts_p: Vec<i64> = pred.iter().map(|f| f.timestamp).collect();
        let ts_o: Vec<i64> = oi.iter().map(|f| f.timestamp).collect();
        let ts_m: Vec<i64> = marks.iter().map(|f| f.timestamp).collect();
        let ts_l: Vec<i64> = lsr.iter().map(|f| f.timestamp).collect();

        // Funding drift: [predicted, funding].
        let fr = GpuEventFrame::merged(&[sp.clone(), sf.clone()]);
        let mut m = FundingDrift::new();
        let mut c = Vec::new();
        for (_, s, k) in mk(vec![ts_p.clone(), ts_f.clone()]) {
            if s == 0 { m.update_predicted_funding(&pred[k]); } else { m.update_funding(&fund[k]); }
            c.push(m.indicator_value());
        }
        assert_close(&launch_cube_events(CubeFormula::FundingDriftMg, &fr, pp(1, 0.0))[0], &c);

        // Funding x OI pressure: [funding, oi].
        let fr = GpuEventFrame::merged(&[sf.clone(), so.clone()]);
        let mut m = FundingOiPressure::new();
        let (mut a, mut b, mut c3) = (Vec::new(), Vec::new(), Vec::new());
        for (_, s, k) in mk(vec![ts_f.clone(), ts_o.clone()]) {
            if s == 0 { m.update_funding(&fund[k]); } else { m.update_oi(&oi[k]); }
            a.push(m.funding());
            b.push(m.oi_delta());
            c3.push(m.pressure());
        }
        assert_cols(&launch_cube_events(CubeFormula::FundingOiPressureMg, &fr, pp(1, 0.0)), &[&a, &b, &c3]);

        // Funding sentiment alignment: [funding, long ratio].
        let fr = GpuEventFrame::merged(&[sf.clone(), sl.clone()]);
        let mut m = FundingSentimentAlignment::new();
        let mut c = Vec::new();
        for (_, s, k) in mk(vec![ts_f.clone(), ts_l.clone()]) {
            if s == 0 { m.update_funding(&fund[k]); } else { m.update_long_short_ratio(&lsr[k]); }
            c.push(m.indicator_value());
        }
        assert_close(&launch_cube_events(CubeFormula::FundingSentimentMg, &fr, pp(1, 0.0))[0], &c);

        // Long squeeze: [open interest, mark].
        let fr = GpuEventFrame::merged(&[so.clone(), sm.clone()]);
        let mut m = LongSqueezeDetector::new();
        let mut c = Vec::new();
        for (_, s, k) in mk(vec![ts_o.clone(), ts_m.clone()]) {
            if s == 0 { m.update_oi(&oi[k]); } else { m.update_mark(&marks[k]); }
            c.push(m.indicator_value());
        }
        assert_close(&launch_cube_events(CubeFormula::LongSqueezeMg, &fr, pp(1, 0.0))[0], &c);

        // Funding price divergence: [funding, price (bar close)].
        let prices: Vec<(i64, f64)> = (0..n).map(|i| (t0 + i as i64 * 1100 + 5, 100.0 + 4.0 * w(i, 0.45))).collect();
        let spr = MergedStream::scalar(&prices, |p| p.0, |p| p.1);
        let fr = GpuEventFrame::merged(&[sf.clone(), spr]);
        let mut m = FundingPriceMomentumDivergence::new(4, 5);
        let (mut a, mut b, mut c3) = (Vec::new(), Vec::new(), Vec::new());
        let pt: Vec<i64> = prices.iter().map(|p| p.0).collect();
        for (_, s, k) in mk(vec![ts_f.clone(), pt]) {
            if s == 0 { m.update_funding(&fund[k]); } else { m.update_price(prices[k].1); }
            a.push(m.funding_slope());
            b.push(m.price_slope());
            c3.push(m.signal());
        }
        assert_cols(&launch_cube_events(CubeFormula::FundingPriceDivMg, &fr, pp(4, 5.0)), &[&a, &b, &c3]);

        // IV/HV spread: [historical vol, vol index].
        let hv: Vec<HistoricalVolatility> = (0..n).map(|i| HistoricalVolatility { volatility: 0.2 + 0.1 * w(i, 0.3), timestamp: t0 + i as i64 * 1000 }).collect();
        let vi: Vec<VolatilityIndex> = (0..n).map(|i| { let mut v = VolatilityIndex::default(); v.value = 0.3 + 0.1 * w(i, 0.35); v.timestamp = t0 + i as i64 * 1200 + 7; v }).collect();
        let fr = GpuEventFrame::merged(&[
            MergedStream::scalar(&hv, |f| f.timestamp, |f| f.volatility),
            MergedStream::scalar(&vi, |f| f.timestamp, |f| f.value),
        ]);
        let mut m = IvHvSpread::new();
        let (mut a, mut b, mut c3) = (Vec::new(), Vec::new(), Vec::new());
        let th: Vec<i64> = hv.iter().map(|f| f.timestamp).collect();
        let tv: Vec<i64> = vi.iter().map(|f| f.timestamp).collect();
        for (_, s, k) in mk(vec![th, tv]) {
            if s == 0 { m.update_historical_volatility(&hv[k]); } else { m.update_volatility_index(&vi[k]); }
            a.push(m.iv());
            b.push(m.hv());
            c3.push(m.spread());
        }
        assert_cols(&launch_cube_events(CubeFormula::IvHvSpreadMg, &fr, pp(1, 0.0)), &[&a, &b, &c3]);

        // Mark vs last: [mark, ticker].
        let tk: Vec<Ticker> = (0..n).map(|i| Ticker { last_price: 100.0 + 3.0 * w(i, 0.52), timestamp: t0 + i as i64 * 1500 + 3, ..Default::default() }).collect();
        let fr = GpuEventFrame::merged(&[sm.clone(), MergedStream::scalar(&tk, |f| f.timestamp, |f| f.last_price)]);
        let mut m = MarkPriceVsLast::new();
        let (mut a, mut b) = (Vec::new(), Vec::new());
        let tt: Vec<i64> = tk.iter().map(|f| f.timestamp).collect();
        for (_, s, k) in mk(vec![ts_m.clone(), tt]) {
            if s == 0 { m.update_mark(&marks[k]); } else { m.update_ticker(&tk[k]); }
            a.push(m.deviation());
            b.push(m.deviation_pct());
        }
        assert_cols(&launch_cube_events(CubeFormula::MarkVsLastMg, &fr, pp(1, 0.0)), &[&a, &b]);

        // Index tracking error: [index price, composite].
        let ip: Vec<IndexPrice> = (0..n).map(|i| IndexPrice { price: 100.0 + 2.0 * w(i, 0.4), timestamp: t0 + i as i64 * 1000, ..Default::default() }).collect();
        let ci: Vec<CompositeIndex> = (0..n).map(|i| CompositeIndex { price: 100.0 + 2.0 * w(i, 0.43), components: Vec::new(), timestamp: t0 + i as i64 * 1100 + 9 }).collect();
        let fr = GpuEventFrame::merged(&[
            MergedStream::scalar(&ip, |f| f.timestamp, |f| f.price),
            MergedStream::scalar(&ci, |f| f.timestamp, |f| f.price),
        ]);
        let mut m = IndexTrackingError::new(6);
        let mut c = Vec::new();
        let ti: Vec<i64> = ip.iter().map(|f| f.timestamp).collect();
        let tc: Vec<i64> = ci.iter().map(|f| f.timestamp).collect();
        for (_, s, k) in mk(vec![ti, tc]) {
            if s == 0 { m.update_index_price(&ip[k]); } else { m.update_composite_index(&ci[k]); }
            c.push(m.indicator_value());
        }
        assert_close(&launch_cube_events(CubeFormula::IndexTrackingMg, &fr, pp(6, 0.0))[0], &c);
        let _ = Ev::F(&fund[0]);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): merged multi-stream rows 1078..=1082.
    #[test]
    fn lane_matches_cpu_merged_batch2() {
        use super::super::event_frame::{GpuEventFrame, MergedStream};
        use super::super::kernels_ev::launch_cube_events;
        use crate::core::types::{IndexPrice, Liquidation, MarkPrice, OpenInterest, SettlementEvent, VolatilityIndex};
        use crate::engine::streams::index_price_consumer::IndexPriceConsumer;
        use crate::engine::streams::liquidation_consumer::LiquidationConsumer;
        use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
        use crate::engine::streams::open_interest_consumer::OpenInterestConsumer;
        use crate::engine::streams::settlement_event_consumer::SettlementEventConsumer;
        use crate::engine::streams::volatility_index_consumer::VolatilityIndexConsumer;
        use crate::indicators::composites::squeeze_probability::SqueezeProbability;
        use crate::indicators::composites::vol_regime_entry::VolRegimeEntry;
        use crate::indicators::index_basis::price_vs_index_spread::PriceVsIndexSpread;
        use crate::indicators::open_interest::oi_price_correlation::OiPriceCorrelation;
        use crate::indicators::settlement::settlement_vs_mark_spread::SettlementVsMarkSpread;

        let n = 50usize;
        let w = |i: usize, k: f64| ((i as f64 * k).sin() * 0.5 + 0.5);
        let t0 = 1_700_000_000_000i64;
        let pp = |p: u32, a: f32, b: f32| { let mut c = CubeParams::period(p); c.a = a; c.b = b; c };
        let mk = |lists: Vec<Vec<i64>>| -> Vec<(i64, usize, usize)> {
            let mut v: Vec<(i64, usize, usize)> = lists.iter().enumerate().flat_map(|(s, l)| l.iter().enumerate().map(move |(k, t)| (*t, s, k))).collect();
            v.sort();
            v
        };
        let oi: Vec<OpenInterest> = (0..n).map(|i| OpenInterest { open_interest: 1000.0 + 100.0 * w(i, 0.6), timestamp: t0 + i as i64 * 1700 + 20, ..Default::default() }).collect();
        let marks: Vec<MarkPrice> = (0..n).map(|i| MarkPrice { mark_price: 100.0 + 3.0 * w(i, 0.5), timestamp: t0 + i as i64 * 900 + 10, ..Default::default() }).collect();
        let ts_o: Vec<i64> = oi.iter().map(|f| f.timestamp).collect();
        let ts_m: Vec<i64> = marks.iter().map(|f| f.timestamp).collect();
        let so = MergedStream::scalar(&oi, |f| f.timestamp, |f| f.open_interest);
        let sm = MergedStream::scalar(&marks, |f| f.timestamp, |f| f.mark_price);

        let fr = GpuEventFrame::merged(&[so.clone(), sm.clone()]);
        let mut m = OiPriceCorrelation::new(8);
        let mut c = Vec::new();
        for (_, s, k) in mk(vec![ts_o.clone(), ts_m.clone()]) {
            if s == 0 { m.update_oi(&oi[k]); } else { m.update_mark(&marks[k]); }
            c.push(m.indicator_value());
        }
        assert_close(&launch_cube_events(CubeFormula::OiPriceCorrMg, &fr, pp(8, 0.0, 0.0))[0], &c);

        // Price vs index: [mark, index] (NaN rows compare as equal in assert_close only when both NaN).
        let ip: Vec<IndexPrice> = (0..n).map(|i| IndexPrice { price: 100.0 + 2.0 * w(i, 0.4), timestamp: t0 + i as i64 * 1000 + 300, ..Default::default() }).collect();
        let ti: Vec<i64> = ip.iter().map(|f| f.timestamp).collect();
        let fr = GpuEventFrame::merged(&[sm.clone(), MergedStream::scalar(&ip, |f| f.timestamp, |f| f.price)]);
        let mut m = PriceVsIndexSpread::new();
        let (mut a, mut b, mut c3) = (Vec::new(), Vec::new(), Vec::new());
        for (_, s, k) in mk(vec![ts_m.clone(), ti]) {
            if s == 0 { m.update_mark(&marks[k]); } else { m.update_index_price(&ip[k]); }
            a.push(m.price());
            b.push(m.index());
            c3.push(m.spread());
        }
        let g = launch_cube_events(CubeFormula::PriceVsIndexMg, &fr, pp(1, 0.0, 0.0));
        for (gc, cc) in g.iter().zip([&a, &b, &c3]) {
            for (x, y) in gc.iter().zip(cc.iter()) {
                assert!((x.is_nan() && y.is_nan()) || ((*x as f64) - y).abs() <= 1e-3 * (1.0 + y.abs()), "{x} vs {y}");
            }
        }

        // Vol regime entry: [vol index, mark].
        let vi: Vec<VolatilityIndex> = (0..n).map(|i| { let mut v = VolatilityIndex::default(); v.value = 0.3 + 0.1 * w(i, 0.9); v.timestamp = t0 + i as i64 * 1200 + 7; v }).collect();
        let tv: Vec<i64> = vi.iter().map(|f| f.timestamp).collect();
        let fr = GpuEventFrame::merged(&[MergedStream::scalar(&vi, |f| f.timestamp, |f| f.value), sm.clone()]);
        let mut m = VolRegimeEntry::new(10);
        let mut c = Vec::new();
        for (_, s, k) in mk(vec![tv, ts_m.clone()]) {
            if s == 0 { m.update_volatility_index(&vi[k]); } else { m.update_mark(&marks[k]); }
            c.push(m.indicator_value());
        }
        assert_close(&launch_cube_events(CubeFormula::VolRegimeEntryMg, &fr, pp(10, 0.0, 0.0))[0], &c);

        // Settlement vs mark: [settlement, mark].
        let st: Vec<SettlementEvent> = (0..n / 5).map(|i| SettlementEvent { settlement_price: 100.0 + w(i, 0.8), settlement_time: 0, timestamp: t0 + i as i64 * 4000 + 100, ..Default::default() }).collect();
        let tt: Vec<i64> = st.iter().map(|f| f.timestamp).collect();
        let fr = GpuEventFrame::merged(&[MergedStream::scalar(&st, |f| f.timestamp, |f| f.settlement_price), sm.clone()]);
        let mut m = SettlementVsMarkSpread::new();
        let (mut a, mut b, mut c3) = (Vec::new(), Vec::new(), Vec::new());
        for (_, s, k) in mk(vec![tt, ts_m.clone()]) {
            if s == 0 { m.update_settlement(&st[k]); } else { m.update_mark(&marks[k]); }
            a.push(m.settlement());
            b.push(m.mark());
            c3.push(m.spread());
        }
        assert_cols(&launch_cube_events(CubeFormula::SettleVsMarkMg, &fr, pp(1, 0.0, 0.0)), &[&a, &b, &c3]);

        // Squeeze probability: [open interest, mark, liquidation].
        let liqs: Vec<Liquidation> = (0..n).map(|i| Liquidation { price: 100.0, quantity: 1.0, timestamp: t0 + i as i64 * 700 + 40, ..Default::default() }).collect();
        let tl: Vec<i64> = liqs.iter().map(|f| f.timestamp).collect();
        let fr = GpuEventFrame::merged(&[so, sm, MergedStream::scalar(&liqs, |f| f.timestamp, |f| f.quantity)]);
        let mut m = SqueezeProbability::new(6000, 10.0);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for (_, s, k) in mk(vec![ts_o, ts_m, tl]) {
            match s { 0 => m.update_oi(&oi[k]), 1 => m.update_mark(&marks[k]), _ => m.update_liquidation(&liqs[k]) }
            a.push(m.prob());
            b.push(m.dir());
        }
        assert_cols(&launch_cube_events(CubeFormula::SqueezeProbMg, &fr, pp(1, 6000.0, 10.0)), &[&a, &b]);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): merged multi-stream rows 1083..=1085.
    #[test]
    fn lane_matches_cpu_merged_batch3() {
        use super::super::event_frame::{GpuEventFrame, MergedStream};
        use super::super::kernels_ev::launch_cube_events;
        use crate::core::types::{AggTrade, FundingRate, InsuranceFund, Liquidation, LongShortRatio, VolatilityIndex};
        use crate::engine::streams::agg_trade_consumer::AggTradeConsumer;
        use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
        use crate::engine::streams::insurance_fund_consumer::InsuranceFundConsumer;
        use crate::engine::streams::liquidation_consumer::LiquidationConsumer;
        use crate::engine::streams::long_short_ratio_consumer::LongShortRatioConsumer;
        use crate::engine::streams::volatility_index_consumer::VolatilityIndexConsumer;
        use crate::indicators::composites::market_stress_composite::MarketStressComposite;
        use crate::indicators::composites::risk_off_detector::RiskOffDetector;
        use crate::indicators::composites::sentiment_composite::SentimentComposite;

        let n = 50usize;
        let w = |i: usize, k: f64| ((i as f64 * k).sin() * 0.5 + 0.5);
        let t0 = 1_700_000_000_000i64;
        let mk = |lists: Vec<Vec<i64>>| -> Vec<(i64, usize, usize)> {
            let mut v: Vec<(i64, usize, usize)> = lists.iter().enumerate().flat_map(|(s, l)| l.iter().enumerate().map(move |(k, t)| (*t, s, k))).collect();
            v.sort();
            v
        };
        let vi: Vec<VolatilityIndex> = (0..n).map(|i| { let mut v = VolatilityIndex::default(); v.value = 0.3 + 0.4 * w(i, 0.5); v.timestamp = t0 + i as i64 * 1200 + 7; v }).collect();
        let liqs: Vec<Liquidation> = (0..n).map(|i| Liquidation { price: 100.0, quantity: 1.0, timestamp: t0 + i as i64 * 700 + 40, ..Default::default() }).collect();
        let fund: Vec<FundingRate> = (0..n).map(|i| FundingRate { rate: (w(i, 0.9) - 0.5) * 0.004, timestamp: t0 + i as i64 * 1900 + 3, ..Default::default() }).collect();
        let ins: Vec<InsuranceFund> = (0..n).map(|i| InsuranceFund { balance: 1.0e6 - 3.0e3 * i as f64 * w(i, 0.3) + 1.0e4 * w(i, 1.7), timestamp: t0 + i as i64 * 1500 + 11 }).collect();
        let tv: Vec<i64> = vi.iter().map(|f| f.timestamp).collect();
        let tl: Vec<i64> = liqs.iter().map(|f| f.timestamp).collect();
        let tf: Vec<i64> = fund.iter().map(|f| f.timestamp).collect();
        let ti: Vec<i64> = ins.iter().map(|f| f.timestamp).collect();
        let streams = [
            MergedStream::scalar(&vi, |f| f.timestamp, |f| f.value),
            MergedStream::scalar(&liqs, |f| f.timestamp, |f| f.quantity),
            MergedStream::scalar(&fund, |f| f.timestamp, |f| f.rate),
            MergedStream::scalar(&ins, |f| f.timestamp, |f| f.balance),
        ];
        let fr = GpuEventFrame::merged(&streams);
        let mut p = CubeParams::period(2);
        p.a = 5000.0;
        p.b = 0.5;
        p.c = 0.1;
        let mut m = RiskOffDetector::new(5000, 0.5, 2, 0.1);
        let mut c = Vec::new();
        for (_, s, k) in mk(vec![tv.clone(), tl.clone(), tf.clone(), ti.clone()]) {
            match s { 0 => m.update_volatility_index(&vi[k]), 1 => m.update_liquidation(&liqs[k]), 2 => m.update_funding(&fund[k]), _ => m.update_insurance_fund(&ins[k]) }
            c.push(m.indicator_value());
        }
        assert_close(&launch_cube_events(CubeFormula::RiskOffMg, &fr, p)[0], &c);

        let mut p = CubeParams::period(8);
        p.a = 5000.0;
        p.b = 6.0;
        p.c = -0.001;
        let mut m = MarketStressComposite::new(5000, 6.0, 8, -0.001);
        let mut c = Vec::new();
        for (_, s, k) in mk(vec![tv, tl, tf.clone(), ti]) {
            match s { 0 => m.update_volatility_index(&vi[k]), 1 => m.update_liquidation(&liqs[k]), 2 => m.update_funding(&fund[k]), _ => m.update_insurance_fund(&ins[k]) }
            c.push(m.indicator_value());
        }
        assert_close(&launch_cube_events(CubeFormula::MarketStressMg, &fr, p)[0], &c);

        let lsr: Vec<LongShortRatio> = (0..n).map(|i| LongShortRatio { long_ratio: 0.3 + 0.4 * w(i, 0.8), timestamp: t0 + i as i64 * 2100 + 30, ..Default::default() }).collect();
        let aggs: Vec<AggTrade> = (0..n).map(|i| AggTrade { price: 100.0 + w(i, 0.3), quantity: 0.2 + w(i, 0.77), timestamp: t0 + i as i64 * 400, is_buy: w(i, 1.1) > 0.5, ..Default::default() }).collect();
        let tls: Vec<i64> = lsr.iter().map(|f| f.timestamp).collect();
        let tag: Vec<i64> = aggs.iter().map(|f| f.timestamp).collect();
        let fr = GpuEventFrame::merged(&[
            MergedStream::scalar(&lsr, |f| f.timestamp, |f| f.long_ratio),
            MergedStream::from_fn(&aggs, 3, |f| f.timestamp, |f| vec![f.price, f.quantity, if f.is_buy { 1.0 } else { 0.0 }]),
            MergedStream::scalar(&fund, |f| f.timestamp, |f| f.rate),
        ]);
        let mut p = CubeParams::period(1);
        p.a = 3000.0;
        let mut m = SentimentComposite::new(3000);
        let mut c = Vec::new();
        for (_, s, k) in mk(vec![tls, tag, tf]) {
            match s { 0 => m.update_long_short_ratio(&lsr[k]), 1 => m.update_agg_trade(&aggs[k]), _ => m.update_funding(&fund[k]) }
            c.push(m.indicator_value());
        }
        assert_close(&launch_cube_events(CubeFormula::SentimentCompMg, &fr, p)[0], &c);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): merged multi-stream rows 1086..=1091.
    #[test]
    fn lane_matches_cpu_merged_batch4() {
        use super::super::event_frame::{GpuEventFrame, MergedStream};
        use super::super::kernels_ev::launch_cube_events;
        use crate::core::types::{
            AggTrade, BlockTrade, FundingRate, FundingSettlement, Liquidation, LongShortRatio, MarkPrice, OpenInterest, TradeSide,
        };
        use crate::engine::streams::agg_trade_consumer::AggTradeConsumer;
        use crate::engine::streams::block_trade_consumer::BlockTradeConsumer;
        use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
        use crate::engine::streams::funding_settlement_consumer::FundingSettlementConsumer;
        use crate::engine::streams::liquidation_consumer::LiquidationConsumer;
        use crate::engine::streams::long_short_ratio_consumer::LongShortRatioConsumer;
        use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
        use crate::engine::streams::open_interest_consumer::OpenInterestConsumer;
        use crate::indicators::composites::block_trade_volume_ratio::BlockTradeVolumeRatio;
        use crate::indicators::composites::capitulation_detector::CapitulationDetector;
        use crate::indicators::composites::compound_squeeze_probability::CompoundSqueezeProbability;
        use crate::indicators::funding_advanced::funding_settlement_impact::FundingSettlementImpact;
        use crate::indicators::liquidations::stop_hunt_detector::StopHuntDetector;
        use crate::indicators::sentiment::ratio_vs_price_divergence::RatioVsPriceDivergence;

        let n = 60usize;
        let w = |i: usize, k: f64| ((i as f64 * k).sin() * 0.5 + 0.5);
        let t0 = 1_700_000_000_000i64;
        let mk = |lists: Vec<Vec<i64>>| -> Vec<(i64, usize, usize)> {
            let mut v: Vec<(i64, usize, usize)> = lists.iter().enumerate().flat_map(|(s, l)| l.iter().enumerate().map(move |(k, t)| (*t, s, k))).collect();
            v.sort();
            v
        };
        let oi: Vec<OpenInterest> = (0..n).map(|i| OpenInterest { open_interest: 1000.0 + 100.0 * w(i, 0.6), timestamp: t0 + i as i64 * 1700 + 20, ..Default::default() }).collect();
        let marks: Vec<MarkPrice> = (0..n).map(|i| MarkPrice { mark_price: 100.0 * (1.0 + 0.03 * (w(i, 0.25) - 0.5)), timestamp: t0 + i as i64 * 900 + 10, ..Default::default() }).collect();
        let liqs: Vec<Liquidation> = (0..n).map(|i| Liquidation { price: 100.0, quantity: 1.0 + 20.0 * w(i, 0.5), timestamp: t0 + i as i64 * 700 + 40, side: if w(i, 0.9) > 0.5 { TradeSide::Buy } else { TradeSide::Sell }, ..Default::default() }).collect();
        let fund: Vec<FundingRate> = (0..n).map(|i| FundingRate { rate: (w(i, 0.9) - 0.5) * 0.004, timestamp: t0 + i as i64 * 1900 + 3, ..Default::default() }).collect();
        let aggs: Vec<AggTrade> = (0..n).map(|i| AggTrade { price: 100.0 + w(i, 0.3), quantity: 0.2 + 3.0 * w(i, 0.77), timestamp: t0 + i as i64 * 400, is_buy: w(i, 1.1) > 0.5, ..Default::default() }).collect();
        let t_o: Vec<i64> = oi.iter().map(|f| f.timestamp).collect();
        let t_m: Vec<i64> = marks.iter().map(|f| f.timestamp).collect();
        let t_l: Vec<i64> = liqs.iter().map(|f| f.timestamp).collect();
        let t_f: Vec<i64> = fund.iter().map(|f| f.timestamp).collect();
        let t_a: Vec<i64> = aggs.iter().map(|f| f.timestamp).collect();
        let pp = |p: u32, a: f32, b: f32, c: f32| { let mut x = CubeParams::period(p); x.a = a; x.b = b; x.c = c; x };

        // Compound squeeze: [oi, mark, liquidation, funding].
        let fr = GpuEventFrame::merged(&[
            MergedStream::scalar(&oi, |f| f.timestamp, |f| f.open_interest),
            MergedStream::scalar(&marks, |f| f.timestamp, |f| f.mark_price),
            MergedStream::scalar(&liqs, |f| f.timestamp, |f| f.quantity),
            MergedStream::scalar(&fund, |f| f.timestamp, |f| f.rate),
        ]);
        let mut m = CompoundSqueezeProbability::new(6000, 10.0);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for (_, s, k) in mk(vec![t_o.clone(), t_m.clone(), t_l.clone(), t_f.clone()]) {
            match s { 0 => m.update_oi(&oi[k]), 1 => m.update_mark(&marks[k]), 2 => m.update_liquidation(&liqs[k]), _ => m.update_funding(&fund[k]) }
            a.push(m.prob());
            b.push(m.dir());
        }
        assert_cols(&launch_cube_events(CubeFormula::CompoundSqueezeMg, &fr, pp(1, 6000.0, 10.0, 0.0)), &[&a, &b]);

        // Capitulation: [liquidation side, agg (price, qty), mark].
        let fr = GpuEventFrame::merged(&[
            MergedStream::scalar(&liqs, |f| f.timestamp, |f| if f.side == TradeSide::Buy { 1.0 } else { -1.0 }),
            MergedStream::from_fn(&aggs, 2, |f| f.timestamp, |f| vec![f.price, f.quantity]),
            MergedStream::scalar(&marks, |f| f.timestamp, |f| f.mark_price),
        ]);
        let mut m = CapitulationDetector::new(3, 100.0, 5000);
        let mut c = Vec::new();
        for (_, s, k) in mk(vec![t_l.clone(), t_a.clone(), t_m.clone()]) {
            match s { 0 => m.update_liquidation(&liqs[k]), 1 => m.update_agg_trade(&aggs[k]), _ => m.update_mark(&marks[k]) }
            c.push(m.indicator_value());
        }
        assert_close(&launch_cube_events(CubeFormula::CapitulationMg, &fr, pp(3, 5000.0, 100.0, 0.0))[0], &c);

        // Block trade volume ratio: [block (price, qty, is_iv), agg (price, qty)].
        let blocks: Vec<BlockTrade> = (0..n / 3).map(|i| BlockTrade { block_id: String::new(), price: 100.0, quantity: 10.0 + 5.0 * w(i, 0.77), is_buy: i % 2 == 0, timestamp: t0 + i as i64 * 1300 + 17, is_iv: i % 5 == 4 }).collect();
        let t_b: Vec<i64> = blocks.iter().map(|f| f.timestamp).collect();
        let fr = GpuEventFrame::merged(&[
            MergedStream::from_fn(&blocks, 3, |f| f.timestamp, |f| vec![f.price, f.quantity, if f.is_iv { 1.0 } else { 0.0 }]),
            MergedStream::from_fn(&aggs, 2, |f| f.timestamp, |f| vec![f.price, f.quantity]),
        ]);
        let mut m = BlockTradeVolumeRatio::new(4000);
        let mut c = Vec::new();
        for (_, s, k) in mk(vec![t_b, t_a.clone()]) {
            if s == 0 { m.update_block_trade(&blocks[k]); } else { m.update_agg_trade(&aggs[k]); }
            c.push(m.indicator_value());
        }
        assert_close(&launch_cube_events(CubeFormula::BlockTradeRatioMg, &fr, pp(1, 4000.0, 0.0, 0.0))[0], &c);

        // Stop hunt: [liquidation (quote value, side), mark].
        let fr = GpuEventFrame::merged(&[
            MergedStream::from_fn(&liqs, 2, |f| f.timestamp, |f| vec![f.quote_value(), if f.side == TradeSide::Buy { 1.0 } else { -1.0 }]),
            MergedStream::scalar(&marks, |f| f.timestamp, |f| f.mark_price),
        ]);
        let mut m = StopHuntDetector::new(500.0, 3000);
        let mut c = Vec::new();
        for (_, s, k) in mk(vec![t_l, t_m.clone()]) {
            if s == 0 { m.update_liquidation(&liqs[k]); } else { m.update_mark(&marks[k]); }
            c.push(m.indicator_value());
        }
        assert_close(&launch_cube_events(CubeFormula::StopHuntMg, &fr, pp(1, 3000.0, 500.0, 0.0))[0], &c);

        // Ratio vs price divergence: [long ratio, price].
        let lsr: Vec<LongShortRatio> = (0..n).map(|i| LongShortRatio { long_ratio: 0.3 + 0.4 * w(i, 0.2), timestamp: t0 + i as i64 * 2100 + 30, ..Default::default() }).collect();
        let prices: Vec<(i64, f64)> = (0..n).map(|i| (t0 + i as i64 * 1100 + 5, 100.0 + 4.0 * w(i, 0.17))).collect();
        let t_ls: Vec<i64> = lsr.iter().map(|f| f.timestamp).collect();
        let t_p: Vec<i64> = prices.iter().map(|f| f.0).collect();
        let fr = GpuEventFrame::merged(&[
            MergedStream::scalar(&lsr, |f| f.timestamp, |f| f.long_ratio),
            MergedStream::scalar(&prices, |f| f.0, |f| f.1),
        ]);
        let mut m = RatioVsPriceDivergence::new(6);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for (_, s, k) in mk(vec![t_ls, t_p]) {
            if s == 0 { m.update_long_short_ratio(&lsr[k]); } else { m.update_price(prices[k].1); }
            a.push(m.score());
            b.push(m.side());
        }
        assert_cols(&launch_cube_events(CubeFormula::RatioVsPriceMg, &fr, pp(6, 0.0, 0.0, 0.0)), &[&a, &b]);

        // Funding settlement impact: [settlement (settlement time), mark].
        let sets: Vec<FundingSettlement> = (0..n / 6).map(|i| FundingSettlement { settled_rate: 0.0001, settlement_time: t0 + i as i64 * 5200 + 2500, timestamp: t0 + i as i64 * 5200 + 100 }).collect();
        let t_s: Vec<i64> = sets.iter().map(|f| f.timestamp).collect();
        let fr = GpuEventFrame::merged(&[
            MergedStream::scalar(&sets, |f| f.timestamp, |f| f.settlement_time as f64).with_time_cols(&[0]),
            MergedStream::scalar(&marks, |f| f.timestamp, |f| f.mark_price),
        ]);
        let mut m = FundingSettlementImpact::new(8);
        let mut c = Vec::new();
        for (_, s, k) in mk(vec![t_s, t_m]) {
            if s == 0 { m.update_funding_settlement(&sets[k]); } else { m.update_mark(&marks[k]); }
            c.push(m.indicator_value());
        }
        assert_close(&launch_cube_events(CubeFormula::FundingSettleImpactMg, &fr, pp(8, 0.0, 0.0, 0.0))[0], &c);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): bar formulas 1200..=1202.
    #[test]
    fn lane_matches_cpu_bar_batch1() {
        use crate::indicators::channels::projection_bands::ProjectionBands;
        use crate::indicators::momentum::stochastikd::StochastikD;
        use crate::indicators::trend_stop::donchian_stop::DonchianStop;

        let bars = bars(150);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let lanes: Vec<[f64; 4]> = bars.iter().map(|b| [b.high, b.low, b.close, b.volume]).collect();
        let _ = (&close, &lanes);
        let cols = |rows: Vec<Vec<f64>>| -> Vec<Vec<f64>> {
            (0..rows[0].len()).map(|k| rows.iter().map(|r| r[k]).collect()).collect()
        };
        let chk = |g: &[Vec<f32>], rows: Vec<Vec<f64>>| {
            let c = cols(rows);
            let r: Vec<&Vec<f64>> = c.iter().collect();
            assert_cols(g, &r);
        };
        let mut p = CubeParams::period(6);
        p.fast = 4;
        let mut m = StochastikD::new(6, 4);
        chk(&run_cols(CubeFormula::StochKdBar, &bars, p), lanes.iter().map(|l| { let (k, d) = m.feed(&[l[0], l[1], l[2]]); vec![k, d] }).collect());
        for (pct, off) in [(false, 0.5f64), (true, 1.5)] {
            let mut p = CubeParams::period(7);
            p.a = off as f32;
            p.flag = pct as u32;
            let mut m = DonchianStop::with_different_periods(9, 7, off, pct);
            chk(&run_cols(CubeFormula::DonchianStopBar, &bars, p), lanes.iter().map(|l| { let (lo, _, _) = m.feed(&[l[0], l[1], l[2]]); vec![lo] }).collect());
        }
        let mut p = CubeParams::period(10);
        p.a = 2.0;
        let mut m = ProjectionBands::new(10, 2.0);
        chk(&run_cols(CubeFormula::ProjBandsBar, &bars, p), close.iter().map(|c| { let (u, mi, l) = m.feed(*c); vec![u, mi, l] }).collect());

    }

    /// UNTESTED on GPU (no GPU on the authoring box): calendar formulas 700..=709.
    #[test]
    fn lane_matches_cpu_calendar_batch() {
        use super::super::kernels_cal::launch_cube_calendar;
        use crate::indicators::calendar::day_of_week_in_month::DayOfWeekInMonthEffect;
        use crate::indicators::calendar::holiday_weekend_proximity::HolidayWeekendProximityEffect;
        use crate::indicators::calendar::hour_of_day_effect::HourOfDayEffect;
        use crate::indicators::calendar::month_turn_effect::MonthTurnEffect;
        use crate::indicators::calendar::quarter_turn_effect::QuarterTurnEffect;
        use crate::indicators::calendar::start_end_of_month_flags::StartEndOfMonthFlags;
        use crate::indicators::calendar::start_end_of_quarter_flags::StartEndOfQuarterFlags;
        use crate::indicators::calendar::start_end_of_week_flags::StartEndOfWeekFlags;
        use crate::indicators::calendar::week_in_month_effect::WeekInMonthEffect;

        // 400 bars, 29 hours apart: spans several months, quarters and month ends.
        let ms: Vec<i64> = (0..400).map(|i| 1_704_067_200_000 + i as i64 * 29 * 3_600_000).collect();
        let t = GpuTimes::from_ms(&ms);
        let col = |f: CubeFormula, w: u32| launch_cube_calendar(f, &t, CubeParams::period(w));
        let one = |f: &mut dyn FnMut(i64) -> f64| -> Vec<f64> { ms.iter().map(|m| f(*m)).collect() };

        let mut e = HourOfDayEffect::new();
        assert_close(&col(CubeFormula::HourOfDay, 1)[0], &one(&mut |m| e.feed(m)));
        let mut e = WeekInMonthEffect::new();
        assert_close(&col(CubeFormula::WeekInMonth, 1)[0], &one(&mut |m| e.feed(m)));
        let mut e = DayOfWeekInMonthEffect::new();
        assert_close(&col(CubeFormula::WeekdayOccurrence, 1)[0], &one(&mut |m| e.feed(m)));
        for w in [2u32, 5] {
            let mut e = MonthTurnEffect::new(w);
            assert_close(&col(CubeFormula::MonthTurn, w)[0], &one(&mut |m| e.feed(m)));
        }
        for w in [3u32, 10] {
            let mut e = QuarterTurnEffect::new(w);
            assert_close(&col(CubeFormula::QuarterTurn, w)[0], &one(&mut |m| e.feed(m)));
        }
        let mut e = HolidayWeekendProximityEffect::new(3);
        assert_close(&col(CubeFormula::WeekendProx, 3)[0], &one(&mut |m| e.feed(m)));

        let (mut s, mut en) = (Vec::new(), Vec::new());
        let mut e = StartEndOfMonthFlags::new(3);
        for m in &ms {
            let (a, b) = e.feed(*m);
            s.push(a);
            en.push(b);
        }
        let g = col(CubeFormula::StartEndMonth, 3);
        assert_close(&g[0], &s);
        assert_close(&g[1], &en);
        let (mut s, mut en) = (Vec::new(), Vec::new());
        let mut e = StartEndOfQuarterFlags::new(4);
        for m in &ms {
            let (a, b) = e.feed(*m);
            s.push(a);
            en.push(b);
        }
        let g = col(CubeFormula::StartEndQuarter, 4);
        assert_close(&g[0], &s);
        assert_close(&g[1], &en);
        let (mut s, mut en) = (Vec::new(), Vec::new());
        let mut e = StartEndOfWeekFlags::new(2);
        for m in &ms {
            let (a, b) = e.feed(*m);
            s.push(a);
            en.push(b);
        }
        let g = col(CubeFormula::StartEndWeek, 2);
        assert_close(&g[0], &s);
        assert_close(&g[1], &en);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): signal formulas 800..=806 (-1 / 0 / 1 as f32).
    #[test]
    fn lane_matches_cpu_signal_batch() {
        use crate::indicators::regime::regime_gate::{GateDirection, RegimeGate};
        use crate::indicators::signal_logic::direction_detector::DirectionDetector;
        use crate::indicators::signal_logic::hysteresis_gate::HysteresisGate;
        use crate::indicators::signal_logic::threshold::{Threshold, ThresholdKind};
        use crate::indicators::signal_logic::threshold_gate::ThresholdGate;
        use crate::indicators::trend::slope_direction_line::SlopeDirectionLine;
        use crate::indicators::volume::volume_event::VolumeEventDetector;

        let bars = bars(120);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let lo = close.iter().cloned().fold(f64::MAX, f64::min);
        let hi = close.iter().cloned().fold(f64::MIN, f64::max);
        let mid = 0.5 * (lo + hi);

        let mut m = DirectionDetector::new();
        assert_close(&run(CubeFormula::DirDetect, &bars, CubeParams::period(1)), &cpu(&close, |v| { m.feed(v); m.value() }));

        let mut g = RegimeGate::new(mid, GateDirection::Above);
        let mut gp = CubeParams::period(1);
        gp.a = mid as f32;
        assert_close(&run(CubeFormula::RegimeGateSig, &bars, gp), &cpu(&close, |v| { g.feed(v); g.value() }));

        for (kind, flag) in [
            (ThresholdKind::Above, 0u32),
            (ThresholdKind::Below, 1),
            (ThresholdKind::InRange, 2),
            (ThresholdKind::OutOfRange, 3),
        ] {
            let up = mid + 0.25 * (hi - lo);
            let dn = mid - 0.25 * (hi - lo);
            let mut t = Threshold::new(kind, up, dn);
            let mut tp = CubeParams::period(1);
            tp.a = up as f32;
            tp.b = dn as f32;
            tp.flag = flag;
            assert_close(&run(CubeFormula::ThresholdEdge, &bars, tp), &cpu(&close, |v| { t.feed(v); t.value() }));
        }

        let mut rp = CubeParams::period(5);
        rp.a = 70.0;
        rp.b = 30.0;
        let mut t = ThresholdGate::with_rsi_period(30.0, 70.0, 5);
        assert_close(&run(CubeFormula::ThresholdGateSig, &bars, rp), &cpu(&close, |v| { t.feed(v); t.value() }));
        let mut h = HysteresisGate::with_rsi_period(30.0, 70.0, 5);
        assert_close(&run(CubeFormula::HysteresisGateSig, &bars, rp), &cpu(&close, |v| { h.feed(v); h.value() }));

        let vols: Vec<f64> = bars.iter().map(|b| b.volume).collect();
        let mut ve = VolumeEventDetector::new(5, 1.1);
        let cpu_ve: Vec<f64> = vols.iter().map(|v| { ve.feed(&[*v]); ve.value() }).collect();
        let mut vp = CubeParams::period(5);
        vp.a = 1.1;
        vp.lane = crate::engine::ohlcv_field::OhlcvField::Volume;
        assert_close(&run(CubeFormula::VolEventSig, &bars, vp), &cpu_ve);

        let mut sd = SlopeDirectionLine::from_smoother(5, SmootherId::Ema);
        let mut sp = CubeParams::period(5);
        sp.smoother = CubeSmoother::Ema;
        sp.smooth_period = 5;
        assert_close(&run(CubeFormula::SlopeDirLine, &bars, sp), &cpu(&close, |v| { sd.feed(v); sd.value() }));
    }

    /// UNTESTED on GPU (no GPU on the authoring box): two-series signals 810..=815.
    #[test]
    fn lane_matches_cpu_signal2_batch() {
        use crate::engine::contract_engine::SmootherSlot;
        use crate::indicators::regime::relative_position::RelativePosition;
        use crate::indicators::regime::volatility_regime::VolatilityRegimeDetector;
        use crate::indicators::signal_logic::logic_gates::{AndGate, OrGate, SignCombiner, XorGate};
        use crate::indicators::statistics::cusum_filter::CusumFilter;

        let bars = bars(150);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let mut p = CubeParams::period(1);
        p.fast = 5;
        p.slow = 12;
        let mut g = AndGate::with_periods(5, 12);
        assert_close(&run(CubeFormula::LogicAnd, &bars, p), &cpu(&close, |v| { g.feed(v); g.value() }));
        let mut g = OrGate::with_periods(5, 12);
        assert_close(&run(CubeFormula::LogicOr, &bars, p), &cpu(&close, |v| { g.feed(v); g.value() }));
        let mut g = XorGate::with_periods(5, 12);
        assert_close(&run(CubeFormula::LogicXor, &bars, p), &cpu(&close, |v| { g.feed(v); g.value() }));
        let mut g = SignCombiner::with_periods(5, 12);
        assert_close(&run(CubeFormula::LogicSign, &bars, p), &cpu(&close, |v| { g.feed(v); g.value() }));

        let lo = close.iter().cloned().fold(f64::MAX, f64::min);
        let hi = close.iter().cloned().fold(f64::MIN, f64::max);
        let mut vr = VolatilityRegimeDetector::new(lo + 0.3 * (hi - lo), lo + 0.7 * (hi - lo));
        let mut vp = CubeParams::period(1);
        vp.a = (lo + 0.3 * (hi - lo)) as f32;
        vp.b = (lo + 0.7 * (hi - lo)) as f32;
        assert_close(&run(CubeFormula::VolRegimeSig, &bars, vp), &cpu(&close, |v| { vr.feed(v); vr.value() }));

        let mut rp = RelativePosition::new(SmootherSlot::new(SmootherId::Ema, 5), SmootherSlot::new(SmootherId::Sma, 12));
        let mut sp = CubeParams::period(1);
        sp.smoother = CubeSmoother::Ema;
        sp.smooth_period = 5;
        sp.smoother2 = CubeSmoother::Sma;
        sp.smooth_period2 = 12;
        let cpu_rp: Vec<f64> = bars.iter().map(|b| { rp.feed(&[b.open, b.high, b.low, b.close]); rp.value() }).collect();
        assert_close(&run(CubeFormula::RelPositionSig, &bars, sp), &cpu_rp);

        let mut cf = CusumFilter::new(0.01);
        let mut cp = CubeParams::period(1);
        cp.a = 0.01;
        assert_close(&run(CubeFormula::CusumFilter, &bars, cp), &cpu(&close, |v| { cf.feed(v); cf.event as f64 }));
    }

    /// UNTESTED on GPU (no GPU on the authoring box): bar-pattern signals 830..=833.
    #[test]
    fn lane_matches_cpu_barsig_batch() {
        use crate::indicators::structure::bos_event_detector::BosEventDetector;
        use crate::indicators::structure::fvg_event_detector::FvgEventDetector;
        use crate::indicators::structure::pivot::Pivot;
        use crate::indicators::swing::williams_fractals::WilliamsFractals;

        let bars = bars(150);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let (mut up, mut dn) = (Vec::new(), Vec::new());
        let mut f = WilliamsFractals::new();
        for b in &bars {
            let (u, d) = f.feed(&[b.high, b.low]);
            up.push(if u { 1.0 } else { 0.0 });
            dn.push(if d { 1.0 } else { 0.0 });
        }
        assert_cols(&run_cols(CubeFormula::Fractals, &bars, CubeParams::period(1)), &[&up, &dn]);

        let mut f = FvgEventDetector::new();
        let c: Vec<f64> = bars.iter().map(|b| { f.feed(&[b.high, b.low]); f.value() }).collect();
        assert_close(&run(CubeFormula::FvgSig, &bars, CubeParams::period(1)), &c);

        let mut pv = Pivot::new(3, 2);
        let mut pp = CubeParams::period(1);
        pp.fast = 3;
        pp.slow = 2;
        assert_close(&run(CubeFormula::NbarPivotSig, &bars, pp), &cpu(&close, |v| { pv.feed(v); pv.value() }));

        let mut bo = BosEventDetector::new(8);
        let c: Vec<f64> = bars.iter().map(|b| { bo.feed(&[b.high, b.low]); bo.value() }).collect();
        assert_close(&run(CubeFormula::BosSig, &bars, CubeParams::period(8)), &c);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): event-frame formulas 900..=917.
    #[test]
    fn lane_matches_cpu_event_batch() {
        use super::super::event_frame::GpuEventFrame;
        use super::super::kernels_ev::launch_cube_events;
        use crate::core::types::{
            AuctionEvent, BlockTrade, FundingRate, HistoricalVolatility, InsuranceFund,
            OptionGreeks, RiskLimit, Tick, Ticker,
        };
        use crate::engine::streams::auction_event_consumer::AuctionEventConsumer;
        use crate::engine::streams::block_trade_consumer::BlockTradeConsumer;
        use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
        use crate::engine::streams::historical_volatility_consumer::HistoricalVolatilityConsumer;
        use crate::engine::streams::insurance_fund_consumer::InsuranceFundConsumer;
        use crate::engine::streams::option_greeks_consumer::OptionGreeksConsumer;
        use crate::engine::streams::risk_limit_consumer::RiskLimitConsumer;
        use crate::engine::streams::tick_consumer::TickConsumer;
        use crate::engine::streams::ticker_consumer::TickerConsumer;

        let n = 60usize;
        let w = |i: usize, k: f64| ((i as f64 * k).sin() * 0.5 + 0.5);
        let ev = |f: CubeFormula, fr: &GpuEventFrame, p: CubeParams| launch_cube_events(f, fr, p);
        let t0 = 1_700_000_000_000i64;

        // Ticker.
        let tickers: Vec<Ticker> = (0..n)
            .map(|i| Ticker {
                last_price: 100.0 + 10.0 * w(i, 0.3),
                bid_price: Some(99.5 + 10.0 * w(i, 0.3)),
                ask_price: Some(100.5 + 10.0 * w(i, 0.3)),
                high_24h: Some(110.0 + w(i, 0.2)),
                low_24h: Some(95.0 - w(i, 0.5)),
                price_change_percent_24h: Some(5.0 * (w(i, 0.7) - 0.5)),
                timestamp: t0 + i as i64 * 1000,
                ..Default::default()
            })
            .collect();
        let fr = GpuEventFrame::from_tickers(&tickers);
        let mut m = crate::indicators::ticker_advanced::high_low_range_ratio::HighLowRangeRatio::new();
        let c: Vec<f64> = tickers.iter().map(|t| { m.update_ticker(t); m.value() }).collect();
        assert_close(&ev(CubeFormula::HlRangeRatio, &fr, CubeParams::period(1))[0], &c);
        let mut m = crate::indicators::ticker_advanced::ticker_spread_ratio::TickerSpreadRatio::new();
        let c: Vec<f64> = tickers.iter().map(|t| { m.update_ticker(t); m.value() }).collect();
        assert_close(&ev(CubeFormula::TickerSpread, &fr, CubeParams::period(1))[0], &c);
        let mut m = crate::indicators::ticker_advanced::price_change_24h_z_score::PriceChange24hZScore::new(8);
        let c: Vec<f64> = tickers.iter().map(|t| { m.update_ticker(t); m.value() }).collect();
        assert_close(&ev(CubeFormula::PctChangeZ, &fr, CubeParams::period(8))[0], &c);

        // Funding.
        let fund: Vec<FundingRate> = (0..n)
            .map(|i| FundingRate { rate: (w(i, 0.9) - 0.5) * 0.002, timestamp: t0 + i as i64 * 1000, ..Default::default() })
            .collect();
        let fr = GpuEventFrame::from_funding(&fund);
        let mut m = crate::indicators::funding_advanced::funding_direction_shift::FundingDirectionShift::new();
        let c: Vec<f64> = fund.iter().map(|t| { m.update_funding(t); m.value() }).collect();
        assert_close(&ev(CubeFormula::FundingDirShift, &fr, CubeParams::period(1))[0], &c);

        // Greeks.
        let greeks: Vec<OptionGreeks> = (0..n)
            .map(|i| OptionGreeks {
                delta: w(i, 0.4) - 0.5,
                gamma: 0.01 * w(i, 0.8),
                vega: 1.0,
                theta: -2.0 * w(i, 0.6),
                rho: 0.1,
                mark_iv: 0.5,
                bid_iv: Some(0.4 + 0.1 * w(i, 0.3)),
                ask_iv: Some(0.5 + 0.1 * w(i, 0.5)),
                timestamp: t0 + i as i64 * 1000,
            })
            .collect();
        let fr = GpuEventFrame::from_greeks(&greeks);
        let mut m = crate::indicators::greeks::iv_skew::IvSkew::new();
        let c: Vec<f64> = greeks.iter().map(|t| { m.update_option_greeks(t); m.value() }).collect();
        assert_close(&ev(CubeFormula::IvSkewEv, &fr, CubeParams::period(1))[0], &c);
        let mut m = crate::indicators::greeks::theta_decay_tracker::ThetaDecayTracker::new(7);
        let c: Vec<f64> = greeks.iter().map(|t| { m.update_option_greeks(t); m.value() }).collect();
        assert_close(&ev(CubeFormula::ThetaDecay, &fr, CubeParams::period(7))[0], &c);
        let mut m = crate::indicators::greeks::pin_risk_detector::PinRiskDetector::new(0.5, 0.1, 0.5);
        let c: Vec<f64> = greeks.iter().map(|t| { m.update_option_greeks(t); m.value() }).collect();
        let mut pp = CubeParams::period(1);
        pp.a = 0.5;
        pp.b = 0.1;
        pp.c = 0.5;
        assert_close(&ev(CubeFormula::PinRisk, &fr, pp)[0], &c);

        // Risk limits.
        let risks: Vec<RiskLimit> = (0..n)
            .map(|i| RiskLimit {
                tier: (i % 5) as u32,
                max_leverage: 50.0 - 5.0 * w(i, 0.35).round(),
                max_position_value: 1.0e6,
                mmr: 0.005 + 0.01 * w(i, 0.2),
                imr: 0.01 + 0.01 * w(i, 0.2),
                timestamp: t0 + i as i64 * 1000,
            })
            .collect();
        let fr = GpuEventFrame::from_risk_limits(&risks);
        let mut m = crate::indicators::risk::mmr_tracker::MmrTracker::new();
        let c: Vec<f64> = risks.iter().map(|t| { m.update_risk_limit(t); m.value() }).collect();
        assert_close(&ev(CubeFormula::MmrTrack, &fr, CubeParams::period(1))[0], &c);
        let mut m = crate::indicators::risk::risk_limit_proximity::RiskLimitProximity::new();
        let c: Vec<f64> = risks.iter().map(|t| { m.update_risk_limit(t); m.value() }).collect();
        assert_close(&ev(CubeFormula::RiskProximity, &fr, CubeParams::period(1))[0], &c);
        let mut m = crate::indicators::risk::leverage_reduction_warning::LeverageReductionWarning::new();
        let c: Vec<f64> = risks.iter().map(|t| { m.update_risk_limit(t); m.value() }).collect();
        assert_close(&ev(CubeFormula::LeverageReduction, &fr, CubeParams::period(1))[0], &c);

        // Ticks.
        let ticks: Vec<Tick> = (0..n)
            .map(|i| Tick::new(t0 + i as i64 * 100, 100.0 + w(i, 0.3), 1.0 + w(i, 0.1), w(i, 1.3) > 0.45))
            .collect();
        let fr = GpuEventFrame::from_ticks(&ticks);
        let mut m = crate::indicators::volume::aggressor_imbalance::AggressorImbalance::new(9);
        let c: Vec<f64> = ticks.iter().map(|t| { m.update_tick(t); m.value() }).collect();
        assert_close(&ev(CubeFormula::AggressorImb, &fr, CubeParams::period(9))[0], &c);
        let mut m = crate::indicators::tick_advanced::trade_run_detector::TradeRunDetector::new(3);
        let (mut cs, mut cr) = (Vec::new(), Vec::new());
        for t in &ticks {
            m.update_tick(t);
            cs.push(m.side());
            cr.push(m.run_length());
        }
        assert_cols(&ev(CubeFormula::TradeRun, &fr, CubeParams::period(1)), &[&cs, &cr]);

        // Insurance fund, historical vol, block trades, auctions.
        let ins: Vec<InsuranceFund> = (0..n)
            .map(|i| InsuranceFund { balance: 1.0e6 - 1.0e4 * i as f64 * w(i, 0.2), timestamp: t0 + i as i64 * 1000 })
            .collect();
        let fr = GpuEventFrame::from_insurance(&ins);
        let mut m = crate::indicators::stress::fund_stress_detector::FundStressDetector::new(6, 1000.0);
        let c: Vec<f64> = ins.iter().map(|t| { m.update_insurance_fund(t); m.value() }).collect();
        let mut ip = CubeParams::period(6);
        ip.a = 1000.0;
        assert_close(&ev(CubeFormula::FundStress, &fr, ip)[0], &c);

        let hv: Vec<HistoricalVolatility> = (0..n)
            .map(|i| HistoricalVolatility { volatility: 0.5 + w(i, 0.6) * (1.0 + (i % 9 == 0) as u8 as f64), timestamp: t0 + i as i64 * 1000 })
            .collect();
        let fr = GpuEventFrame::from_historical_vol(&hv);
        let mut m = crate::indicators::volatility_advanced::hv_spike::HvSpike::new(8, 1.3);
        let c: Vec<f64> = hv.iter().map(|t| { m.update_historical_volatility(t); m.value() }).collect();
        let mut hp = CubeParams::period(8);
        hp.a = 1.3;
        assert_close(&ev(CubeFormula::HvSpikeEv, &fr, hp)[0], &c);

        let blocks: Vec<BlockTrade> = (0..n)
            .map(|i| BlockTrade { block_id: String::new(), price: 100.0, quantity: 10.0 + 50.0 * w(i, 0.77), is_buy: i % 2 == 0, timestamp: t0 + i as i64 * 1000, is_iv: false })
            .collect();
        let fr = GpuEventFrame::from_block_trades(&blocks);
        let mut m = crate::indicators::microstructure::block_trade_size_anomaly::BlockTradeSizeAnomaly::new(10);
        let c: Vec<f64> = blocks.iter().map(|t| { m.update_block_trade(t); m.value() }).collect();
        assert_close(&ev(CubeFormula::BlockSizeZ, &fr, CubeParams::period(10))[0], &c);

        let auctions: Vec<AuctionEvent> = (0..n)
            .map(|i| AuctionEvent {
                auction_id: String::new(),
                indicative_price: 100.0,
                indicative_qty: 5.0 + 20.0 * w(i, 0.45),
                state: String::new(),
                timestamp: t0 + i as i64 * 1000,
            })
            .collect();
        let fr = GpuEventFrame::from_auctions(&auctions);
        let mut m = crate::indicators::auction::auction_liquidity_score::AuctionLiquidityScore::new(6);
        let c: Vec<f64> = auctions.iter().map(|t| { m.update_auction(t); m.value() }).collect();
        assert_close(&ev(CubeFormula::AuctionLiq, &fr, CubeParams::period(6))[0], &c);
    }

    /// UNTESTED on GPU (no GPU on the authoring box): time-window event formulas 920..=936.
    #[test]
    fn lane_matches_cpu_event_time_batch() {
        use super::super::event_frame::GpuEventFrame;
        use super::super::kernels_ev::launch_cube_events;
        use crate::core::types::{AggTrade, BlockTrade, Liquidation, OpenInterest, Tick, TradeSide};
        use crate::engine::streams::agg_trade_consumer::AggTradeConsumer;
        use crate::engine::streams::block_trade_consumer::BlockTradeConsumer;
        use crate::engine::streams::liquidation_consumer::LiquidationConsumer;
        use crate::engine::streams::open_interest_consumer::OpenInterestConsumer;
        use crate::engine::streams::tick_consumer::TickConsumer;

        let n = 80usize;
        let w = |i: usize, k: f64| ((i as f64 * k).sin() * 0.5 + 0.5);
        let t0 = 1_700_000_000_000i64;
        // Irregular but increasing times (ms), at most a few seconds apart.
        let times: Vec<i64> = (0..n).scan(t0, |t, i| { *t += 50 + (w(i, 1.7) * 900.0) as i64; Some(*t) }).collect();
        let pw = |ms: f32| { let mut p = CubeParams::period(1); p.a = ms; p };

        let liqs: Vec<Liquidation> = (0..n)
            .map(|i| Liquidation {
                side: if w(i, 0.9) > 0.5 { TradeSide::Buy } else { TradeSide::Sell },
                price: 100.0 + w(i, 0.3),
                quantity: 1.0 + 5.0 * w(i, 0.5),
                timestamp: times[i],
                ..Default::default()
            })
            .collect();
        let fr = GpuEventFrame::from_liquidations(&liqs);
        let mut m = crate::indicators::liquidations::liquidation_rate::LiquidationRate::new(4000);
        let c: Vec<f64> = liqs.iter().map(|t| { m.update_liquidation(t); m.value() }).collect();
        assert_close(&launch_cube_events(CubeFormula::LiqRate, &fr, pw(4000.0))[0], &c);
        let mut m = crate::indicators::liquidations::liquidation_cooldown::LiquidationCooldown::new();
        let c: Vec<f64> = liqs.iter().map(|t| { m.update_liquidation(t); m.value() }).collect();
        assert_close(&launch_cube_events(CubeFormula::LiqCooldown, &fr, pw(1.0))[0], &c);
        let mut m = crate::indicators::liquidations::liquidation_volume_velocity::LiquidationVolumeVelocity::new(4000);
        let c: Vec<f64> = liqs.iter().map(|t| { m.update_liquidation(t); m.value() }).collect();
        assert_close(&launch_cube_events(CubeFormula::LiqVolVelocity, &fr, pw(4000.0))[0], &c);
        let mut m = crate::indicators::liquidations::liquidation_volume_imbalance::LiquidationVolumeImbalance::new(4000);
        let (mut a, mut b, mut cc) = (Vec::new(), Vec::new(), Vec::new());
        for t in &liqs {
            m.update_liquidation(t);
            a.push(m.imbalance());
            b.push(m.long_vol());
            cc.push(m.short_vol());
        }
        assert_cols(&launch_cube_events(CubeFormula::LiqVolImbalance, &fr, pw(4000.0)), &[&a, &b, &cc]);
        let mut m = crate::indicators::liquidations::liquidation_cascade::LiquidationCascade::new(4000, 5);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for t in &liqs {
            m.update_liquidation(t);
            a.push(m.flag());
            b.push(m.count());
        }
        let mut cp = pw(4000.0);
        cp.period = 5;
        assert_cols(&launch_cube_events(CubeFormula::LiqCascade, &fr, cp), &[&a, &b]);

        let aggs: Vec<AggTrade> = (0..n)
            .map(|i| AggTrade {
                aggregate_id: i as i64,
                price: 100.0,
                quantity: 1.0 + w(i, 0.4),
                first_trade_id: 0,
                last_trade_id: 0,
                is_buy: w(i, 1.1) > 0.5,
                timestamp: times[i],
                is_best_match: None,
                non_rpi_qty: None,
                quote_qty: None,
            })
            .collect();
        let fr = GpuEventFrame::from_agg_trades(&aggs);
        let mut m = crate::indicators::sentiment::agg_trade_flow_imbalance::AggTradeFlowImbalance::new(3000);
        let c: Vec<f64> = aggs.iter().map(|t| { m.update_agg_trade(t); m.value() }).collect();
        assert_close(&launch_cube_events(CubeFormula::AggFlowImb, &fr, pw(3000.0))[0], &c);

        let blocks: Vec<BlockTrade> = (0..n)
            .map(|i| BlockTrade { block_id: String::new(), price: 100.0, quantity: 10.0 + 5.0 * w(i, 0.77), is_buy: i % 3 != 0, timestamp: times[i], is_iv: false })
            .collect();
        let fr = GpuEventFrame::from_block_trades(&blocks);
        let mut m = crate::indicators::microstructure::block_trade_flow::BlockTradeFlow::new(5000);
        let c: Vec<f64> = blocks.iter().map(|t| { m.update_block_trade(t); m.value() }).collect();
        assert_close(&launch_cube_events(CubeFormula::BlockFlow, &fr, pw(5000.0))[0], &c);
        let mut m = crate::indicators::microstructure::block_trade_impact::BlockTradeImpact::new(5000);
        let c: Vec<f64> = blocks.iter().map(|t| { m.update_block_trade(t); m.value() }).collect();
        assert_close(&launch_cube_events(CubeFormula::BlockRate, &fr, pw(5000.0))[0], &c);

        let ois: Vec<OpenInterest> = (0..n)
            .map(|i| OpenInterest {
                open_interest: 1000.0 + 50.0 * w(i, 0.2),
                timestamp: times[i],
                ..Default::default()
            })
            .collect();
        let fr = GpuEventFrame::from_open_interest(&ois);
        let mut m = crate::indicators::open_interest::oi_change_rate::OiChangeRate::new();
        let (mut a, mut b) = (Vec::new(), Vec::new());
        for t in &ois {
            m.update_oi(t);
            a.push(m.rate());
            b.push(m.current());
        }
        assert_cols(&launch_cube_events(CubeFormula::OiChangeRateEv, &fr, pw(1.0)), &[&a, &b]);

        let ticks: Vec<Tick> = (0..n)
            .map(|i| Tick::new(times[i], 100.0 + w(i, 0.3), 0.5 + 3.0 * w(i, 0.13), w(i, 1.3) > 0.45))
            .collect();
        let fr = GpuEventFrame::from_ticks(&ticks);
        let mut m = crate::indicators::tick_advanced::tick_frequency_anomaly::TickFrequencyAnomaly::new(1500, 6000);
        let c: Vec<f64> = ticks.iter().map(|t| { m.update_tick(t); m.value() }).collect();
        let mut fp = pw(1500.0);
        fp.b = 6000.0;
        assert_close(&launch_cube_events(CubeFormula::TickFreqAnomaly, &fr, fp)[0], &c);
        let mut m = crate::indicators::tick_advanced::aggressor_burst_detector::AggressorBurstDetector::new(3000, 4, 0.7);
        let c: Vec<f64> = ticks.iter().map(|t| { m.update_tick(t); m.value() }).collect();
        let mut bp = pw(3000.0);
        bp.period = 4;
        bp.c = 0.7;
        assert_close(&launch_cube_events(CubeFormula::AggBurst, &fr, bp)[0], &c);
        let mut m = crate::indicators::tick_advanced::large_tick_momentum::LargeTickMomentum::new(1.5, 3000);
        let c: Vec<f64> = ticks.iter().map(|t| { m.update_tick(t); m.value() }).collect();
        let mut lp = pw(3000.0);
        lp.b = 1.5;
        assert_close(&launch_cube_events(CubeFormula::LargeTickMom, &fr, lp)[0], &c);
        let mut m = crate::indicators::tick_advanced::size_weighted_directional_momentum::SizeWeightedDirectionalMomentum::new(3000);
        let c: Vec<f64> = ticks.iter().map(|t| { m.update_tick(t); m.value() }).collect();
        assert_close(&launch_cube_events(CubeFormula::SizeWtMom, &fr, pw(3000.0))[0], &c);
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
        for depth in [2u32, 3u32] {
            let mut book_slope = OrderBookSlope::with_levels(depth as usize);
            let cpu_slope: Vec<f64> = books
                .iter()
                .map(|b| {
                    book_slope.update_orderbook(b);
                    book_slope.value()
                })
                .collect();
            let mut slope_params = CubeParams::period(1);
            slope_params.levels = depth;
            assert_close(
                &launch_cube(CubeFormula::BookSlope, &book_samples, slope_params),
                &cpu_slope,
            );
        }

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

        let wper = 5usize;
        let mut wr = WilliamsR::new(wper);
        let cpu_wr: Vec<f64> = bars
            .iter()
            .map(|b| wr.feed(&[b.high, b.low, b.close]))
            .collect();
        assert_close(
            &run(CubeFormula::WilliamsR, &bars, CubeParams::period(wper as u32)),
            &cpu_wr,
        );
        let mut obv = Obv::new();
        let cpu_obv: Vec<f64> = bars
            .iter()
            .map(|b| obv.feed(&[b.close, b.volume]))
            .collect();
        assert_close(&run(CubeFormula::Obv, &bars, CubeParams::period(1)), &cpu_obv);
        let mut obv_hi = Obv::new();
        let cpu_obv_hi: Vec<f64> = bars
            .iter()
            .map(|b| obv_hi.feed(&[b.high, b.volume]))
            .collect();
        let mut obv_hi_params = CubeParams::period(1);
        obv_hi_params.lane = OhlcvField::High;
        assert_close(&run(CubeFormula::Obv, &bars, obv_hi_params), &cpu_obv_hi);
        let mut pvt = PriceVolumeTrend::new();
        let cpu_pvt: Vec<f64> = bars
            .iter()
            .map(|b| pvt.feed(&[b.close, b.volume]))
            .collect();
        assert_close(&run(CubeFormula::Pvt, &bars, CubeParams::period(1)), &cpu_pvt);
        let mut mfi = Mfi::new(wper);
        let cpu_mfi: Vec<f64> = bars
            .iter()
            .map(|b| mfi.feed(&[b.high, b.low, b.close, b.volume]))
            .collect();
        assert_close(
            &run(CubeFormula::Mfi, &bars, CubeParams::period(wper as u32)),
            &cpu_mfi,
        );
        let mut ad = AccumulationDistribution::new();
        let cpu_ad: Vec<f64> = bars
            .iter()
            .map(|b| ad.feed(&[b.high, b.low, b.close, b.volume]))
            .collect();
        assert_close(&run(CubeFormula::AdLine, &bars, CubeParams::period(1)), &cpu_ad);
        let mut dem = Demarker::new(wper);
        let cpu_dem: Vec<f64> = bars.iter().map(|b| dem.feed(&[b.high, b.low])).collect();
        assert_close(
            &run(CubeFormula::Demarker, &bars, CubeParams::period(wper as u32)),
            &cpu_dem,
        );
        let mut ulcer = UlcerIndex::with_period(wper);
        let cpu_ulcer: Vec<f64> = bars.iter().map(|b| ulcer.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::Ulcer, &bars, CubeParams::period(wper as u32)),
            &cpu_ulcer,
        );
        let mut rv = RealizedVol::new(wper, 252.0_f64.sqrt());
        let cpu_rv: Vec<f64> = bars.iter().map(|b| rv.feed(b.close)).collect();
        let mut rv_params = CubeParams::period(wper as u32);
        rv_params.a = (252.0_f32).sqrt();
        assert_close(&run(CubeFormula::RealizedVol, &bars, rv_params), &cpu_rv);
        let mut rv_raw = RealizedVol::new(wper, 0.0);
        let cpu_rv_raw: Vec<f64> = bars.iter().map(|b| rv_raw.feed(b.close)).collect();
        let mut rv_raw_params = CubeParams::period(wper as u32);
        rv_raw_params.a = 0.0;
        assert_close(&run(CubeFormula::RealizedVol, &bars, rv_raw_params), &cpu_rv_raw);
        let mut er = EfficiencyRatio::new(wper);
        let cpu_er: Vec<f64> = bars.iter().map(|b| er.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::Efficiency, &bars, CubeParams::period(wper as u32)),
            &cpu_er,
        );

        let mut gapo = Gapo::new(wper);
        let cpu_gapo: Vec<f64> = bars.iter().map(|b| gapo.feed(&[b.high, b.low])).collect();
        assert_close(
            &run(CubeFormula::Gapo, &bars, CubeParams::period(wper as u32)),
            &cpu_gapo,
        );
        let mut wvf = Wvf::new(wper);
        let cpu_wvf: Vec<f64> = bars.iter().map(|b| wvf.feed(&[b.low, b.close])).collect();
        assert_close(
            &run(CubeFormula::Wvf, &bars, CubeParams::period(wper as u32)),
            &cpu_wvf,
        );
        let mut rq = RealizedQuarticity::new(wper);
        let cpu_rq: Vec<f64> = bars.iter().map(|b| rq.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::Quarticity, &bars, CubeParams::period(wper as u32)),
            &cpu_rq,
        );
        let mut hv = HistoricalVolatilityC2C::new(wper);
        let cpu_hv: Vec<f64> = bars.iter().map(|b| hv.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::HvC2c, &bars, CubeParams::period(wper as u32)),
            &cpu_hv,
        );
        let mut psl = Psl::new(wper);
        let cpu_psl: Vec<f64> = bars.iter().map(|b| psl.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::Psl, &bars, CubeParams::period(wper as u32)),
            &cpu_psl,
        );
        let mut imi = IntradayMomentumIndex::new(wper);
        let cpu_imi: Vec<f64> = bars
            .iter()
            .map(|b| imi.feed(&[b.open, b.high, b.low, b.close]))
            .collect();
        assert_close(
            &run(CubeFormula::Imi, &bars, CubeParams::period(wper as u32)),
            &cpu_imi,
        );
        let mut pzo = Pzo::new(wper);
        let cpu_pzo: Vec<f64> = bars.iter().map(|b| pzo.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::Pzo, &bars, CubeParams::period(wper as u32)),
            &cpu_pzo,
        );
        let mut cog = CenterOfGravity::new(wper);
        let cpu_cog: Vec<f64> = bars.iter().map(|b| cog.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::Cog, &bars, CubeParams::period(wper as u32)),
            &cpu_cog,
        );
        let mut bpv = BipowerVariance::new(wper);
        let cpu_bpv: Vec<f64> = bars.iter().map(|b| bpv.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::Bipower, &bars, CubeParams::period(wper as u32)),
            &cpu_bpv,
        );
        let mut vhf = Vhf::new(wper);
        let cpu_vhf: Vec<f64> = bars.iter().map(|b| vhf.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::Vhf, &bars, CubeParams::period(wper as u32)),
            &cpu_vhf,
        );
        let mut pfe = Pfe::new(wper);
        let cpu_pfe: Vec<f64> = bars.iter().map(|b| pfe.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::Pfe, &bars, CubeParams::period(wper as u32)),
            &cpu_pfe,
        );
        let mut vz = VolumeZscore::new(wper);
        let cpu_vz: Vec<f64> = bars.iter().map(|b| vz.feed(&[b.volume])).collect();
        assert_close(
            &run(CubeFormula::VolumeZ, &bars, CubeParams::period(wper as u32)),
            &cpu_vz,
        );
        let mut mz = MomentumZscore::new(3, wper);
        let cpu_mz: Vec<f64> = bars.iter().map(|b| mz.feed(b.close)).collect();
        let mut mz_params = CubeParams::period(3);
        mz_params.fast = wper as u32;
        assert_close(&run(CubeFormula::MomZ, &bars, mz_params), &cpu_mz);
        let mut pb = PercentB::new(wper, 2.0);
        let cpu_pb: Vec<f64> = bars.iter().map(|b| pb.feed(b.close)).collect();
        let mut pb_params = CubeParams::period(wper as u32);
        pb_params.a = 2.0;
        assert_close(&run(CubeFormula::PercentB, &bars, pb_params), &cpu_pb);
        let mut cfo = Cfo::new(wper);
        let cpu_cfo: Vec<f64> = bars.iter().map(|b| cfo.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::Cfo, &bars, CubeParams::period(wper as u32)),
            &cpu_cfo,
        );
        let mut rmid = RollingMidline::new(wper);
        let cpu_rmid: Vec<f64> = bars.iter().map(|b| rmid.feed(&[b.high, b.low])).collect();
        assert_close(
            &run(CubeFormula::Rmid, &bars, CubeParams::period(wper as u32)),
            &cpu_rmid,
        );
        let mut wad = WilliamsAd::new();
        let cpu_wad: Vec<f64> = bars
            .iter()
            .map(|b| wad.feed(&[b.high, b.low, b.close]))
            .collect();
        assert_close(&run(CubeFormula::Wad, &bars, CubeParams::period(1)), &cpu_wad);
        let mut mad = PriceMadZscore::new(wper);
        let cpu_mad: Vec<f64> = bars.iter().map(|b| mad.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::MadZ, &bars, CubeParams::period(wper as u32)),
            &cpu_mad,
        );
        let mut cmf = ChaikinMoneyFlow::new(wper);
        let cpu_cmf: Vec<f64> = bars
            .iter()
            .map(|b| cmf.feed(&[b.high, b.low, b.close, b.volume]))
            .collect();
        assert_close(
            &run(CubeFormula::Cmf, &bars, CubeParams::period(wper as u32)),
            &cpu_cmf,
        );
        let mut vwap = Vwap::new(wper);
        let cpu_vwap: Vec<f64> = bars
            .iter()
            .map(|b| vwap.feed(&[b.high, b.low, b.close, b.volume]))
            .collect();
        assert_close(
            &run(CubeFormula::Vwap, &bars, CubeParams::period(wper as u32)),
            &cpu_vwap,
        );
        let mut rsx = Rsx::new(wper);
        let cpu_rsx: Vec<f64> = bars.iter().map(|b| rsx.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::Rsx, &bars, CubeParams::period(wper as u32)),
            &cpu_rsx,
        );
        let mut asi = AccumulativeSwingIndex::new();
        let cpu_asi: Vec<f64> = bars
            .iter()
            .map(|b| asi.feed(&[b.open, b.high, b.low, b.close]))
            .collect();
        assert_close(&run(CubeFormula::Asi, &bars, CubeParams::period(1)), &cpu_asi);
        let mut var = Var::new(wper, 0.95);
        let cpu_var: Vec<f64> = bars.iter().map(|b| var.feed(b.close)).collect();
        let mut var_params = CubeParams::period(wper as u32);
        var_params.a = 0.95;
        assert_close(&run(CubeFormula::Var, &bars, var_params), &cpu_var);
        let mut chop = ChoppinessIndex::with_period(wper);
        let cpu_chop: Vec<f64> = bars
            .iter()
            .map(|b| chop.feed(&[b.high, b.low, b.close]))
            .collect();
        assert_close(
            &run(CubeFormula::Chop, &bars, CubeParams::period(wper as u32)),
            &cpu_chop,
        );
        let mut ao = AwesomeOscillator::new();
        let cpu_ao: Vec<f64> = bars
            .iter()
            .map(|b| {
                ao.feed(&[b.high, b.low]);
                ao.value()
            })
            .collect();
        assert_close(&run(CubeFormula::Ao, &bars, CubeParams::period(1)), &cpu_ao);
        let mut dpo = DpoPercent::new(wper);
        let cpu_dpo: Vec<f64> = bars.iter().map(|b| dpo.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::DpoPct, &bars, CubeParams::period(wper as u32)),
            &cpu_dpo,
        );
        let mut env = EnvelopeBandwidth::new(wper, 2.5);
        let cpu_env: Vec<f64> = bars.iter().map(|b| env.feed(b.close)).collect();
        let mut env_params = CubeParams::period(wper as u32);
        env_params.a = 2.5;
        assert_close(&run(CubeFormula::Envbw, &bars, env_params), &cpu_env);
        let mut ac = AccelerationDeceleration::new();
        let cpu_ac: Vec<f64> = bars
            .iter()
            .map(|b| {
                ac.feed(&[b.high, b.low]);
                ac.value()
            })
            .collect();
        assert_close(&run(CubeFormula::Ac, &bars, CubeParams::period(1)), &cpu_ac);
        let mut wmfi = MarketFacilitationIndex::new();
        let cpu_wmfi: Vec<f64> = bars
            .iter()
            .map(|b| wmfi.feed(&[b.high, b.low, b.volume]))
            .collect();
        assert_close(
            &run(CubeFormula::WilliamsMfi, &bars, CubeParams::period(1)),
            &cpu_wmfi,
        );

        // UNTESTED on GPU (no GPU on the authoring box): batch I, codes 74..=79.
        let mut vfi = Vfi::new(wper);
        let cpu_vfi: Vec<f64> = bars
            .iter()
            .map(|b| vfi.feed(&[b.high, b.low, b.close, b.volume]))
            .collect();
        assert_close(
            &run(CubeFormula::Vfi, &bars, CubeParams::period(wper as u32)),
            &cpu_vfi,
        );
        let mut vzo = Vzo::new(wper);
        let cpu_vzo: Vec<f64> = bars.iter().map(|b| vzo.feed(&[b.close, b.volume])).collect();
        assert_close(
            &run(CubeFormula::Vzo, &bars, CubeParams::period(wper as u32)),
            &cpu_vzo,
        );
        let mut iip = IntradayIntensityPercent::new();
        let cpu_iip: Vec<f64> = bars
            .iter()
            .map(|b| iip.feed(&[b.high, b.low, b.close, b.volume]))
            .collect();
        assert_close(
            &run(CubeFormula::IntradayPct, &bars, CubeParams::period(1)),
            &cpu_iip,
        );
        let mut iir = IntradayIntensityRatio::new(wper);
        let cpu_iir: Vec<f64> = bars
            .iter()
            .map(|b| iir.feed(&[b.high, b.low, b.close, b.volume]))
            .collect();
        assert_close(
            &run(CubeFormula::IntradayRatio, &bars, CubeParams::period(wper as u32)),
            &cpu_iir,
        );
        let mut dcpos = DonchianPosition::new(wper);
        let cpu_dcpos: Vec<f64> = bars
            .iter()
            .map(|b| dcpos.feed(&[b.high, b.low, b.close]))
            .collect();
        assert_close(
            &run(CubeFormula::DonchianPos, &bars, CubeParams::period(wper as u32)),
            &cpu_dcpos,
        );
        let mut dcw = DonchianWidth::new(wper);
        let cpu_dcw: Vec<f64> = bars.iter().map(|b| dcw.feed(&[b.high, b.low])).collect();
        assert_close(
            &run(CubeFormula::DonchianWidth, &bars, CubeParams::period(wper as u32)),
            &cpu_dcw,
        );

        // UNTESTED on GPU (no GPU on the authoring box): batch J, codes 80..=85.
        let mut pcho = PriceChannelOscillator::new(wper);
        let cpu_pcho: Vec<f64> = bars
            .iter()
            .map(|b| pcho.feed(&[b.high, b.low, b.close]))
            .collect();
        assert_close(
            &run(CubeFormula::PriceChannelOsc, &bars, CubeParams::period(wper as u32)),
            &cpu_pcho,
        );
        let mut pchw = PriceChannelWidth::new(wper);
        let cpu_pchw: Vec<f64> = bars
            .iter()
            .map(|b| pchw.feed(&[b.high, b.low]))
            .collect();
        assert_close(
            &run(CubeFormula::PriceChannelWidth, &bars, CubeParams::period(wper as u32)),
            &cpu_pchw,
        );
        let mut er_full = EfficiencyRatioFullHistory::new(wper);
        let cpu_er_full: Vec<f64> = bars.iter().map(|b| er_full.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::ErFull, &bars, CubeParams::period(wper as u32)),
            &cpu_er_full,
        );
        let mut er_ring = EfficiencyRatioRingWindow::new(wper);
        let cpu_er_ring: Vec<f64> = bars.iter().map(|b| er_ring.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::ErRing, &bars, CubeParams::period(wper as u32)),
            &cpu_er_ring,
        );
        let mut r2 = RSquared::new(wper);
        let cpu_r2: Vec<f64> = bars.iter().map(|b| r2.feed(b.close)).collect();
        assert_close(
            &run(CubeFormula::RSquared, &bars, CubeParams::period(wper as u32)),
            &cpu_r2,
        );
        let mut vdist = VwapDistance::new(wper);
        let cpu_vdist: Vec<f64> = bars
            .iter()
            .map(|b| vdist.feed(&[b.high, b.low, b.close, b.volume]))
            .collect();
        assert_close(
            &run(CubeFormula::VwapDistance, &bars, CubeParams::period(wper as u32)),
            &cpu_vdist,
        );

        // UNTESTED on GPU (no GPU on the authoring box): batches K and L, codes 86..=90.
        let mut cyber = CyberCycle::new(0.2);
        let cpu_cyber: Vec<f64> = bars.iter().map(|b| cyber.feed(b.close)).collect();
        let mut cyber_params = CubeParams::period(1);
        cyber_params.a = 0.2;
        assert_close(&run(CubeFormula::CyberCycle, &bars, cyber_params), &cpu_cyber);
        let mut ama = Ama::new(5, 2, 30);
        let cpu_ama: Vec<f64> = bars.iter().map(|b| ama.feed(b.close)).collect();
        let mut ama_params = CubeParams::period(5);
        ama_params.fast = 2;
        ama_params.slow = 30;
        assert_close(&run(CubeFormula::Ama, &bars, ama_params), &cpu_ama);
        let mut vbexp = VolatilityBreakExp::new(0.1, 2.0);
        let cpu_vbexp: Vec<f64> = bars.iter().map(|b| vbexp.feed(b.close)).collect();
        let mut vbexp_params = CubeParams::period(1);
        vbexp_params.a = 0.1;
        vbexp_params.b = 2.0;
        assert_close(&run(CubeFormula::VolBreak, &bars, vbexp_params), &cpu_vbexp);
        let mut acorr = Autocorr::new(2, wper);
        let cpu_acorr: Vec<f64> = bars.iter().map(|b| acorr.feed(b.close)).collect();
        let mut acorr_params = CubeParams::period(wper as u32);
        acorr_params.fast = 2;
        assert_close(&run(CubeFormula::Autocorr, &bars, acorr_params), &cpu_acorr);
        let mut vr = VarianceRatio::new(20, 5);
        let cpu_vr: Vec<f64> = bars.iter().map(|b| vr.feed(b.close)).collect();
        let mut vr_params = CubeParams::period(20);
        vr_params.fast = 5;
        assert_close(&run(CubeFormula::VarianceRatio, &bars, vr_params), &cpu_vr);

        // UNTESTED on GPU (no GPU on the authoring box): multi-column batch M, codes 100..=105.
        let mut dc = DonchianChannel::new(wper);
        let (mut dc_u, mut dc_m, mut dc_l) = (Vec::new(), Vec::new(), Vec::new());
        for b in &bars {
            let (u, l, m) = dc.feed(&[b.high, b.low]);
            dc_u.push(u);
            dc_m.push(m);
            dc_l.push(l);
        }
        assert_cols(
            &run_cols(CubeFormula::DonchianBands, &bars, CubeParams::period(wper as u32)),
            &[&dc_u, &dc_m, &dc_l],
        );
        let mut dcm = DonchianMetrics::new(wper);
        let (mut dcm_w, mut dcm_p) = (Vec::new(), Vec::new());
        for b in &bars {
            let (w, p) = dcm.feed(&[b.high, b.low, b.close]);
            dcm_w.push(w);
            dcm_p.push(p);
        }
        assert_cols(
            &run_cols(CubeFormula::DonchianMetrics, &bars, CubeParams::period(wper as u32)),
            &[&dcm_w, &dcm_p],
        );
        let mut aroon = Aroon::new(wper);
        let (mut ar_u, mut ar_d, mut ar_o) = (Vec::new(), Vec::new(), Vec::new());
        for b in &bars {
            let (u, d, o) = aroon.feed(&[b.high, b.low]);
            ar_u.push(u);
            ar_d.push(d);
            ar_o.push(o);
        }
        assert_cols(
            &run_cols(CubeFormula::AroonCols, &bars, CubeParams::period(wper as u32)),
            &[&ar_u, &ar_d, &ar_o],
        );
        let mut cpr = CentralPivotRange::new();
        let (mut cpr_bc, mut cpr_p, mut cpr_tc) = (Vec::new(), Vec::new(), Vec::new());
        for b in &bars {
            cpr.feed(&[b.high, b.low, b.close]);
            cpr_bc.push(cpr.bc());
            cpr_p.push(cpr.pivot());
            cpr_tc.push(cpr.tc());
        }
        assert_cols(
            &run_cols(CubeFormula::CentralPivotRange, &bars, CubeParams::period(1)),
            &[&cpr_bc, &cpr_p, &cpr_tc],
        );
        let mut ha = HeikinAshi::new();
        let (mut ha_o, mut ha_h, mut ha_l, mut ha_c) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for b in &bars {
            let (o, h, l, c) = ha.feed(&[b.open, b.high, b.low, b.close]);
            ha_o.push(o);
            ha_h.push(h);
            ha_l.push(l);
            ha_c.push(c);
        }
        assert_cols(
            &run_cols(CubeFormula::HeikinAshiCols, &bars, CubeParams::period(1)),
            &[&ha_o, &ha_h, &ha_l, &ha_c],
        );
        let mut anat = CandleAnatomy::new(0.6);
        let (mut an_b, mut an_u, mut an_l, mut an_lu, mut an_ll) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for b in &bars {
            let v = anat.feed(&[b.open, b.high, b.low, b.close]);
            an_b.push(v.body);
            an_u.push(v.upper_wick);
            an_l.push(v.lower_wick);
            an_lu.push(if v.long_upper { 1.0 } else { 0.0 });
            an_ll.push(if v.long_lower { 1.0 } else { 0.0 });
        }
        let mut anat_params = CubeParams::period(1);
        anat_params.a = 0.6;
        assert_cols(
            &run_cols(CubeFormula::CandleAnatomyCols, &bars, anat_params),
            &[&an_b, &an_u, &an_l, &an_lu, &an_ll],
        );

        // UNTESTED on GPU (no GPU on the authoring box): smoothed formulas, codes 120..=127.
        // Every cube smoother is checked against its core on Qstick; the other formulas
        // check the default leg plus one cascaded smoother.
        let smoothers: [(CubeSmoother, SmootherId); 9] = [
            (CubeSmoother::Sma, SmootherId::Sma),
            (CubeSmoother::Ema, SmootherId::Ema),
            (CubeSmoother::Wma, SmootherId::Wma),
            (CubeSmoother::Rma, SmootherId::Rma),
            (CubeSmoother::Dema, SmootherId::Dema),
            (CubeSmoother::Tema, SmootherId::Tema),
            (CubeSmoother::Tma, SmootherId::Tma),
            (CubeSmoother::Hma, SmootherId::Hma),
            (CubeSmoother::Alma, SmootherId::Alma),
        ];
        assert_eq!(CubeSmoother::from_id(SmootherId::Trima), CubeSmoother::Tma);
        for (cs, id) in smoothers {
            let mut q = Qstick::from_smoother(wper, id);
            let cpu_q: Vec<f64> = bars.iter().map(|b| q.feed(&[b.open, b.close])).collect();
            let mut qp = CubeParams::period(wper as u32);
            qp.smoother = cs;
            assert_close(&run(CubeFormula::QstickSmoothed, &bars, qp), &cpu_q);
        }
        for (cs, id) in [
            (CubeSmoother::Ema, SmootherId::Ema),
            (CubeSmoother::Hma, SmootherId::Hma),
        ] {
            let mut fi = ForceIndex::from_smoothers(id, wper);
            let cpu_fi: Vec<f64> = bars.iter().map(|b| fi.feed(&[b.close, b.volume])).collect();
            let mut fp = CubeParams::period(wper as u32);
            fp.smoother = cs;
            assert_close(&run(CubeFormula::ForceIndexSmoothed, &bars, fp), &cpu_fi);

            let mut cop = CoppockCurve::from_smoother(4, 6, wper, id);
            let cpu_cop: Vec<f64> = bars.iter().map(|b| cop.feed(b.close)).collect();
            let mut cp = CubeParams::period(4);
            cp.fast = 6;
            cp.smoother = cs;
            cp.smooth_period = wper as u32;
            assert_close(&run(CubeFormula::CoppockSmoothed, &bars, cp), &cpu_cop);

            let mut vo = VolumeOscillator::from_smoothers(id, 3, SmootherId::Sma, 8);
            let cpu_vo: Vec<f64> = bars.iter().map(|b| vo.feed(b.volume)).collect();
            let mut vp = CubeParams::period(3);
            vp.smoother = cs;
            vp.smooth_period = 3;
            vp.smoother2 = CubeSmoother::Sma;
            vp.smooth_period2 = 8;
            assert_close(&run(CubeFormula::VolumeOscSmoothed, &bars, vp), &cpu_vo);

            let mut cho = ChaikinOscillator::from_smoothers(id, 3, SmootherId::Ema, 10);
            let cpu_cho: Vec<f64> = bars
                .iter()
                .map(|b| cho.feed(&[b.high, b.low, b.close, b.volume]))
                .collect();
            let mut hp = CubeParams::period(3);
            hp.smoother = cs;
            hp.smooth_period = 3;
            hp.smoother2 = CubeSmoother::Ema;
            hp.smooth_period2 = 10;
            assert_close(&run(CubeFormula::ChaikinOscSmoothed, &bars, hp), &cpu_cho);

            let mut natr = Natr::from_smoother(wper, id);
            let cpu_natr: Vec<f64> = bars
                .iter()
                .map(|b| natr.feed(&[b.high, b.low, b.close]))
                .collect();
            let mut np = CubeParams::period(wper as u32);
            np.smoother = cs;
            assert_close(&run(CubeFormula::NatrSmoothed, &bars, np), &cpu_natr);
        }
        let mut ii = IntradayIntensity::new(wper);
        let cpu_ii: Vec<f64> = bars
            .iter()
            .map(|b| ii.feed(&[b.high, b.low, b.close, b.volume]))
            .collect();
        assert_close(
            &run(CubeFormula::IntradayIntensitySmoothed, &bars, CubeParams::period(wper as u32)),
            &cpu_ii,
        );
        let mut eom = EaseOfMovement::with_params(wper, 1000.0);
        let cpu_eom: Vec<f64> = bars
            .iter()
            .map(|b| eom.feed(&[b.high, b.low, b.volume]))
            .collect();
        let mut ep = CubeParams::period(wper as u32);
        ep.a = 1000.0;
        assert_close(&run(CubeFormula::EaseOfMovementSmoothed, &bars, ep), &cpu_eom);

        // UNTESTED on GPU (no GPU on the authoring box): batch N (codes 91..=93) and the
        // calendar adapter (codes 140..=143).
        let mut vroc = VolumeRateOfChange::with_params(5, 3);
        let cpu_vroc: Vec<f64> = bars.iter().map(|b| vroc.feed(b.volume)).collect();
        let mut vroc_params = CubeParams::period(5);
        vroc_params.lane = OhlcvField::Volume;
        assert_close(&run(CubeFormula::Vroc, &bars, vroc_params), &cpu_vroc);
        let mut donbo = DonchianBreakout::new(wper);
        let cpu_donbo: Vec<f64> = bars
            .iter()
            .map(|b| donbo.feed(&[b.high, b.low, b.close]) as f64)
            .collect();
        assert_close(
            &run(CubeFormula::DonchianBreakout, &bars, CubeParams::period(wper as u32)),
            &cpu_donbo,
        );
        let mut hat = HeikinAshiTrend::new();
        let cpu_hat: Vec<f64> = bars
            .iter()
            .map(|b| hat.feed(&[b.open, b.high, b.low, b.close]) as f64)
            .collect();
        assert_close(&run(CubeFormula::HeikinAshiTrend, &bars, CubeParams::period(1)), &cpu_hat);

        // Calendar bars: 17 h apart from 2024-01-01 UTC, so weekdays, sessions, months
        // and days of month all vary.
        let timed: Vec<ResearchBar> = bars
            .iter()
            .enumerate()
            .map(|(i, b)| {
                ResearchBar::new(
                    1_704_067_200_000 + i as i64 * 17 * 3_600_000,
                    b.open,
                    b.high,
                    b.low,
                    b.close,
                    b.volume,
                )
            })
            .collect();
        let tsamples: Vec<GpuSample> = timed.iter().map(GpuSample::from).collect();
        let tcols = GpuTimes::from_bars(&timed);
        let mut wk = WeekdayEffect::new();
        let cpu_wk: Vec<f64> = timed
            .iter()
            .map(|b| {
                wk.feed(b.time, b.close);
                wk.value()
            })
            .collect();
        assert_close(
            &launch_cube_timed(CubeFormula::WeekdayEffect, &tsamples, &tcols, CubeParams::period(1)),
            &cpu_wk,
        );
        let mut se = SessionEffect::new();
        let cpu_se: Vec<f64> = timed
            .iter()
            .map(|b| {
                se.feed(b.time, b.close);
                se.value()
            })
            .collect();
        assert_close(
            &launch_cube_timed(CubeFormula::SessionEffect, &tsamples, &tcols, CubeParams::period(1)),
            &cpu_se,
        );
        let mut mq = MonthQuarterEffect::new();
        let cpu_mq: Vec<f64> = timed
            .iter()
            .map(|b| {
                mq.feed(b.time, b.close);
                mq.value()
            })
            .collect();
        assert_close(
            &launch_cube_timed(CubeFormula::MonthEffect, &tsamples, &tcols, CubeParams::period(1)),
            &cpu_mq,
        );
        let mut dw = DayOfMonthWeekOfQuarterEffect::new();
        let cpu_dw: Vec<f64> = timed
            .iter()
            .map(|b| {
                dw.feed(b.time, b.close);
                dw.value()
            })
            .collect();
        assert_close(
            &launch_cube_timed(CubeFormula::DayOfMonthEffect, &tsamples, &tcols, CubeParams::period(1)),
            &cpu_dw,
        );

        // UNTESTED on GPU (no GPU on the authoring box): Hampel filter, code 94.
        let mut hampel = HampelFilter::new(7, 2.0);
        let cpu_hampel: Vec<f64> = bars.iter().map(|b| hampel.feed(b.close)).collect();
        let mut hampel_params = CubeParams::period(7);
        hampel_params.a = 2.0;
        assert_close(&run(CubeFormula::Hampel, &bars, hampel_params), &cpu_hampel);

        assert!(run(CubeFormula::WindowMean, &[], CubeParams::period(5)).is_empty());
    }
}
