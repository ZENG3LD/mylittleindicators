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
use super::CubeFormula;
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

/// Sequential formulas. One unit writes the whole lane.
#[cube]
fn scan_lane(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
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
    }
}

#[cube(launch_unchecked)]
fn lane_map(
    open: &[f32],
    high: &[f32],
    low: &[f32],
    close: &[f32],
    volume: &[f32],
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
    formula: u32,
) {
    let i = ABSOLUTE_POS;
    if i < open.len() {
        if formula == 20u32 {
            let mut denom = high[i] - low[i];
            if denom < 0.0f32 {
                denom = -denom;
            }
            if denom < 1.0e-12 {
                denom = 1.0e-12;
            }
            output[i] = (close[i] - open[i]) / denom;
        } else if formula >= 5u32 {
            if i == 0 {
                scan_lane(
                    open, high, low, close, volume, output, lane, lane2, period, fast, slow,
                    signal, a, b, flag, formula,
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

/// Run one cube formula over OHLCV bars.
///
/// `params` carries the source lanes and the config axes (`period`, `fast`,
/// `slow`, `signal`, `a`, `b`, `flag`). An empty bar slice returns an empty
/// buffer and does not create a device.
pub fn launch_cube(formula: CubeFormula, bars: &[ResearchBar], params: CubeParams) -> Vec<f32> {
    if bars.is_empty() {
        return Vec::new();
    }
    let n = bars.len();
    let open: Vec<f32> = bars.iter().map(|b| b.open as f32).collect();
    let high: Vec<f32> = bars.iter().map(|b| b.high as f32).collect();
    let low: Vec<f32> = bars.iter().map(|b| b.low as f32).collect();
    let close: Vec<f32> = bars.iter().map(|b| b.close as f32).collect();
    let volume: Vec<f32> = bars.iter().map(|b| b.volume as f32).collect();
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let open_b = client.create_from_slice(f32::as_bytes(&open));
    let high_b = client.create_from_slice(f32::as_bytes(&high));
    let low_b = client.create_from_slice(f32::as_bytes(&low));
    let close_b = client.create_from_slice(f32::as_bytes(&close));
    let volume_b = client.create_from_slice(f32::as_bytes(&volume));
    let output = client.empty(n * core::mem::size_of::<f32>());
    let dim = 64u32;
    let cubes = (n as u32).div_ceil(dim);
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
    use crate::indicators::swing::highest::Highest;
    use crate::indicators::swing::lowest::Lowest;
    use crate::indicators::volatility::atr::Atr;
    use crate::indicators::volatility::true_range::TrueRange;

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

    fn cpu(values: &[f64], mut step: impl FnMut(f64) -> f64) -> Vec<f64> {
        values.iter().copied().map(|v| step(v)).collect()
    }

    #[test]
    fn lane_matches_cpu() {
        let bars = bars(70);
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let high: Vec<f64> = bars.iter().map(|b| b.high).collect();
        let p5 = CubeParams::period(5);

        let identity = launch_cube(CubeFormula::Identity, &bars, CubeParams::period(1));
        assert_close(&identity, &close);

        let mut vol_lane = CubeParams::period(1);
        vol_lane.lane = OhlcvField::Volume;
        let volumes: Vec<f64> = bars.iter().map(|b| b.volume).collect();
        assert_close(
            &launch_cube(CubeFormula::Identity, &bars, vol_lane),
            &volumes,
        );

        let mut sma = Sma::new(5);
        assert_close(
            &launch_cube(CubeFormula::WindowMean, &bars, p5),
            &cpu(&close, |v| sma.feed(v)),
        );

        let mut sma_hl2 = Sma::new(5);
        let hl2: Vec<f64> = bars.iter().map(|b| (b.high + b.low) / 2.0).collect();
        let mut hl2_params = CubeParams::period(5);
        hl2_params.lane = OhlcvField::HL2;
        assert_close(
            &launch_cube(CubeFormula::WindowMean, &bars, hl2_params),
            &cpu(&hl2, |v| sma_hl2.feed(v)),
        );

        let mut highest = Highest::new(5);
        let mut hi_params = CubeParams::period(5);
        hi_params.lane = OhlcvField::High;
        assert_close(
            &launch_cube(CubeFormula::WindowMax, &bars, hi_params),
            &cpu(&high, |v| highest.feed(v)),
        );

        let mut lowest = Lowest::new(5);
        assert_close(
            &launch_cube(CubeFormula::WindowMin, &bars, p5),
            &cpu(&close, |v| lowest.feed(v)),
        );

        let mut wma = Wma::new(4);
        assert_close(
            &launch_cube(CubeFormula::WindowWeighted, &bars, CubeParams::period(4)),
            &cpu(&close, |v| wma.feed(v)),
        );

        let mut ema = Ema::new(5);
        assert_close(
            &launch_cube(CubeFormula::Ema, &bars, p5),
            &cpu(&close, |v| ema.feed(v)),
        );
        let mut rma = Rma::new(5);
        assert_close(
            &launch_cube(CubeFormula::Rma, &bars, p5),
            &cpu(&close, |v| rma.feed(v)),
        );
        let mut dema = Dema::new(5);
        assert_close(
            &launch_cube(CubeFormula::Dema, &bars, p5),
            &cpu(&close, |v| dema.feed(v)),
        );
        let mut tema = Tema::new(5);
        assert_close(
            &launch_cube(CubeFormula::Tema, &bars, p5),
            &cpu(&close, |v| tema.feed(v)),
        );
        let mut tma = Tma::new(5);
        assert_close(
            &launch_cube(CubeFormula::Tma, &bars, p5),
            &cpu(&close, |v| tma.feed(v)),
        );
        let mut trima = Trima::new(5);
        assert_close(
            &launch_cube(CubeFormula::Tma, &bars, p5),
            &cpu(&close, |v| trima.feed(v)),
        );
        let mut hma = Hma::new(8);
        assert_close(
            &launch_cube(CubeFormula::Hma, &bars, CubeParams::period(8)),
            &cpu(&close, |v| hma.feed(v)),
        );

        let mut alma = Alma::new(5);
        assert_close(
            &launch_cube(CubeFormula::Alma, &bars, p5),
            &cpu(&close, |v| alma.feed(v)),
        );
        let mut alma_custom = Alma::with_params(5, 0.5, 3.0);
        let mut alma_params = CubeParams::period(5);
        alma_params.a = 0.5;
        alma_params.b = 3.0;
        assert_close(
            &launch_cube(CubeFormula::Alma, &bars, alma_params),
            &cpu(&close, |v| alma_custom.feed(v)),
        );

        let mut t3 = T3::new(5);
        let mut t3_params = CubeParams::period(5);
        t3_params.a = 0.7;
        assert_close(
            &launch_cube(CubeFormula::T3, &bars, t3_params),
            &cpu(&close, |v| t3.feed(v)),
        );
        let mut t3_custom = T3::with_alpha(5, 0.4);
        let mut t3_custom_params = CubeParams::period(5);
        t3_custom_params.a = 0.4;
        assert_close(
            &launch_cube(CubeFormula::T3, &bars, t3_custom_params),
            &cpu(&close, |v| t3_custom.feed(v)),
        );

        let mut md = McGinleyDynamic::new(5);
        assert_close(
            &launch_cube(CubeFormula::Mcginley, &bars, p5),
            &cpu(&close, |v| md.feed(v)),
        );

        let mut roc = Roc::new(5, false);
        assert_close(
            &launch_cube(CubeFormula::Roc, &bars, p5),
            &cpu(&close, |v| roc.feed(v)),
        );
        let mut roc_log = Roc::new(5, true);
        let mut roc_params = CubeParams::period(5);
        roc_params.flag = 1;
        assert_close(
            &launch_cube(CubeFormula::Roc, &bars, roc_params),
            &cpu(&close, |v| roc_log.feed(v)),
        );

        let mut rsi = Rsi::new(5);
        assert_close(
            &launch_cube(CubeFormula::Rsi, &bars, p5),
            &cpu(&close, |v| rsi.feed(v)),
        );
        let mut cmo = Cmo::new(5);
        assert_close(
            &launch_cube(CubeFormula::Cmo, &bars, p5),
            &cpu(&close, |v| cmo.feed(v)),
        );
        let mut bias = Bias::new(5);
        assert_close(
            &launch_cube(CubeFormula::Bias, &bars, p5),
            &cpu(&close, |v| bias.feed(v)),
        );

        let mut tr = TrueRange::new();
        let cpu_tr: Vec<f64> = bars
            .iter()
            .map(|b| tr.feed(&[b.high, b.low, b.close]))
            .collect();
        assert_close(
            &launch_cube(CubeFormula::TrueRange, &bars, CubeParams::period(1)),
            &cpu_tr,
        );

        let mut atr = Atr::new_wilder(5);
        let cpu_atr: Vec<f64> = bars
            .iter()
            .map(|b| atr.feed(&[b.high, b.low, b.close]))
            .collect();
        assert_close(&launch_cube(CubeFormula::Atr, &bars, p5), &cpu_atr);

        let mut bop = Bop::new();
        let cpu_bop: Vec<f64> = bars
            .iter()
            .map(|b| bop.feed(&[b.open, b.high, b.low, b.close]))
            .collect();
        assert_close(
            &launch_cube(CubeFormula::Bop, &bars, CubeParams::period(1)),
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
            &launch_cube(CubeFormula::Vwma, &bars, vwma_params),
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
            &launch_cube(CubeFormula::Vwma, &hold, hold_params),
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
            &launch_cube(CubeFormula::Macd, &bars, macd_params),
            &cpu_macd,
        );

        let mut apo = Apo::new(4, 9);
        assert_close(
            &launch_cube(
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

        assert!(launch_cube(CubeFormula::WindowMean, &[], CubeParams::period(5)).is_empty());
    }
}
