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
