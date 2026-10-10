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
        if formula.code() >= 60 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ohlcv_field::OhlcvField;
    use crate::indicators::accumulation::accumulation_distribution::AccumulationDistribution;
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
    use crate::indicators::accumulation::williams_ad::WilliamsAd;
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

        assert!(run(CubeFormula::WindowMean, &[], CubeParams::period(5)).is_empty());
    }
}
