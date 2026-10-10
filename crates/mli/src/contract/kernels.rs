//! One CubeCL kernel for every [`CubeFormula`].
//!
//! The CPU cores stay f64. This kernel is the f32 batch map of the same
//! lane. It is compiled only with the `gpu` feature, which links the CubeCL
//! wgpu runtime. A new formula is another arm here, not a new kernel file
//! and not a trait impl on the indicator.

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::CubeFormula;

#[cube]
fn wma_point(input: &[f32], i: usize, wp: usize) -> f32 {
    let mut out = input[i];
    if i + 1 >= wp {
        let start = i + 1 - wp;
        let mut acc = 0.0f32;
        let mut wsum = 0.0f32;
        for k in 0..wp {
            let w = (k + 1) as f32;
            acc = acc + input[start + k] * w;
            wsum = wsum + w;
        }
        out = acc / wsum;
    }
    out
}

#[cube]
fn prefix_mean(input: &[f32], i: usize, period: usize) -> f32 {
    let mut window = period;
    if window > i + 1 {
        window = i + 1;
    }
    let start = i + 1 - window;
    let mut acc = 0.0f32;
    for k in 0..window {
        acc = acc + input[start + k];
    }
    acc / (window as f32)
}

/// Sequential formulas. One unit writes the whole lane. The math matches the
/// f64 `feed` of the indicator that carries the formula.
#[cube]
fn scan_lane(input: &[f32], output: &mut [f32], period: u32, formula: u32) {
    let n = input.len();
    let mut p = period as usize;
    if p < 1 {
        p = 1;
    }
    let pf = p as f32;
    let alpha = 2.0f32 / (pf + 1.0f32);

    if formula == 5u32 {
        let mut y = input[0];
        output[0] = y;
        for k in 1..n {
            y = alpha * input[k] + (1.0f32 - alpha) * y;
            output[k] = y;
        }
    } else if formula == 6u32 {
        let mut y = input[0];
        output[0] = y;
        for k in 1..n {
            y = (y * (pf - 1.0f32) + input[k]) / pf;
            output[k] = y;
        }
    } else if formula == 7u32 {
        let mut e1 = input[0];
        let mut e2 = e1;
        output[0] = 2.0f32 * e1 - e2;
        for k in 1..n {
            e1 = alpha * input[k] + (1.0f32 - alpha) * e1;
            e2 = alpha * e1 + (1.0f32 - alpha) * e2;
            output[k] = 2.0f32 * e1 - e2;
        }
    } else if formula == 8u32 {
        let mut e1 = input[0];
        let mut e2 = e1;
        let mut e3 = e2;
        output[0] = 3.0f32 * e1 - 3.0f32 * e2 + e3;
        for k in 1..n {
            e1 = alpha * input[k] + (1.0f32 - alpha) * e1;
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
                acc = acc + prefix_mean(input, start + j, p);
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
                let w1 = wma_point(input, idx, half);
                let w2 = wma_point(input, idx, p);
                output[idx] = 2.0f32 * w1 - w2;
            } else {
                let start = idx + 1 - sq;
                let mut acc = 0.0f32;
                let mut wsum = 0.0f32;
                for k in 0..sq {
                    let w1 = wma_point(input, start + k, half);
                    let w2 = wma_point(input, start + k, p);
                    let diff = 2.0f32 * w1 - w2;
                    let w = (k + 1) as f32;
                    acc = acc + diff * w;
                    wsum = wsum + w;
                }
                output[idx] = acc / wsum;
            }
        }
    } else if formula == 11u32 {
        let m = 0.85f32 * (pf - 1.0f32);
        let mut s = pf / 6.0f32;
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
                    acc = acc + input[start + k] * wi;
                    wsum = wsum + wi;
                }
                output[idx] = acc / wsum;
            }
        }
    } else if formula == 12u32 {
        let a = 0.7f32;
        let one = 1.0f32 - a;
        let c6 = a * a * a;
        let c5 = 3.0f32 * a * a * one;
        let c4 = 3.0f32 * a * one * one;
        let c3 = one * one * one;
        let mut e1 = input[0];
        let mut e2 = e1;
        let mut e3 = e2;
        let mut e4 = e3;
        let mut e5 = e4;
        let mut e6 = e5;
        output[0] = e6 * c6 + e5 * c5 + e4 * c4 + e3 * c3;
        for k in 1..n {
            e1 = alpha * input[k] + (1.0f32 - alpha) * e1;
            e2 = alpha * e1 + (1.0f32 - alpha) * e2;
            e3 = alpha * e2 + (1.0f32 - alpha) * e3;
            e4 = alpha * e3 + (1.0f32 - alpha) * e4;
            e5 = alpha * e4 + (1.0f32 - alpha) * e5;
            e6 = alpha * e5 + (1.0f32 - alpha) * e6;
            output[k] = e6 * c6 + e5 * c5 + e4 * c4 + e3 * c3;
        }
    } else if formula == 13u32 {
        let mut y = input[0];
        output[0] = y;
        for k in 1..n {
            if y == 0.0f32 {
                y = input[k];
            } else {
                let mut ratio = input[k] / y;
                if ratio < 0.0f32 {
                    ratio = -ratio;
                }
                let r2 = ratio * ratio;
                let mut denom = pf * r2 * r2;
                if denom < 1.0e-9 {
                    denom = 1.0e-9;
                }
                y = y + (input[k] - y) / denom;
            }
            output[k] = y;
        }
    } else if formula == 14u32 {
        for idx in 0..n {
            if idx + 1 < p {
                output[idx] = 0.0f32;
            } else {
                let prev = input[idx - (p - 1)];
                output[idx] = (input[idx] - prev) / prev;
            }
        }
    } else if formula == 15u32 {
        output[0] = 0.0f32;
        let mut prev = input[0];
        let mut gain = 0.0f32;
        let mut loss = 0.0f32;
        let mut gcount = 0usize;
        let mut lcount = 0usize;
        let mut value = 0.0f32;
        for k in 1..n {
            let diff = input[k] - prev;
            let mut up = 0.0f32;
            let mut down = 0.0f32;
            if diff > 0.0f32 {
                up = diff;
            } else {
                down = -diff;
            }
            prev = input[k];
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
        let mut prev = input[0];
        let mut gain = 0.0f32;
        let mut loss = 0.0f32;
        let mut gcount = 0usize;
        let mut lcount = 0usize;
        let mut index = 1usize;
        for k in 1..n {
            let diff = input[k] - prev;
            let mut up = 0.0f32;
            let mut down = 0.0f32;
            if diff > 0.0f32 {
                up = diff;
            } else {
                down = -diff;
            }
            prev = input[k];
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
                    acc = acc + input[start + j];
                }
                let sma = acc / pf;
                let mut sa = sma;
                if sa < 0.0f32 {
                    sa = -sa;
                }
                if sa < 1.0e-12 {
                    output[idx] = 0.0f32;
                } else {
                    output[idx] = input[idx] / sma - 1.0f32;
                }
            }
        }
    }
}

#[cube(launch_unchecked)]
fn lane_map(input: &[f32], output: &mut [f32], period: u32, formula: u32) {
    let i = ABSOLUTE_POS;
    if i < input.len() {
        if formula >= 5u32 {
            if i == 0 {
                scan_lane(input, output, period, formula);
            }
        } else if formula == 0u32 {
            output[i] = input[i];
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
                    output[i] = input[i];
                } else {
                    let mut acc = 0.0f32;
                    let mut wsum = 0.0f32;
                    for k in 0..p {
                        let w = (k + 1) as f32;
                        acc = acc + input[start + k] * w;
                        wsum = wsum + w;
                    }
                    output[i] = acc / wsum;
                }
            } else if formula == 2u32 {
                let mut best = input[start];
                for k in 0..window {
                    let v = input[start + k];
                    if v > best {
                        best = v;
                    }
                }
                output[i] = best;
            } else if formula == 3u32 {
                let mut best = input[start];
                for k in 0..window {
                    let v = input[start + k];
                    if v < best {
                        best = v;
                    }
                }
                output[i] = best;
            } else {
                let mut acc = 0.0f32;
                for k in 0..window {
                    acc = acc + input[start + k];
                }
                output[i] = acc / (window as f32);
            }
        }
    }
}

/// Run one cube formula over `series` and return the mapped buffer.
///
/// `period` is the window. [`CubeFormula::Identity`] ignores it. An empty
/// series returns an empty buffer and does not create a device.
pub fn launch_cube(formula: CubeFormula, series: &[f32], period: u32) -> Vec<f32> {
    if series.is_empty() {
        return Vec::new();
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let input = client.create_from_slice(f32::as_bytes(series));
    let output = client.empty(series.len() * core::mem::size_of::<f32>());
    let dim = 64u32;
    let cubes = (series.len() as u32).div_ceil(dim);
    unsafe {
        lane_map::launch_unchecked(
            &client,
            CubeCount::new_1d(cubes),
            CubeDim::new_1d(dim),
            BufferArg::from_raw_parts(input, series.len()),
            BufferArg::from_raw_parts(output.clone(), series.len()),
            period,
            formula.code(),
        );
    }
    let bytes = client.read_one_unchecked(output);
    f32::from_bytes(&bytes).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
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
    use crate::indicators::average::wma::Wma;
    use crate::indicators::momentum::bias::Bias;
    use crate::indicators::momentum::cmo::Cmo;
    use crate::indicators::momentum::roc::Roc;
    use crate::indicators::momentum::rsi::Rsi;
    use crate::indicators::swing::highest::Highest;
    use crate::indicators::swing::lowest::Lowest;

    fn assert_close(gpu: &[f32], cpu: &[f64]) {
        assert_eq!(gpu.len(), cpu.len());
        for (g, c) in gpu.iter().zip(cpu) {
            let delta = (*g as f64 - *c).abs();
            assert!(delta < 1e-4, "{g} vs {c}");
        }
    }

    fn series(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| 10.0 + (i as f64 * 0.35).sin() * 4.0 + ((i % 7) as f64) * 0.5)
            .collect()
    }

    fn cpu(values: &[f64], mut step: impl FnMut(f64) -> f64) -> Vec<f64> {
        values.iter().copied().map(|v| step(v)).collect()
    }

    #[test]
    fn lane_matches_cpu() {
        let values = series(70);
        let gpu_in: Vec<f32> = values.iter().map(|v| *v as f32).collect();

        let identity = launch_cube(CubeFormula::Identity, &gpu_in, 1);
        assert_close(&identity, &values);

        let mut sma = Sma::new(5);
        let cpu_sma: Vec<f64> = values.iter().copied().map(|v| sma.feed(v)).collect();
        assert_close(
            &launch_cube(CubeFormula::WindowMean, &gpu_in, 5),
            &cpu_sma,
        );

        let mut highest = Highest::new(5);
        let cpu_hi: Vec<f64> = values.iter().copied().map(|v| highest.feed(v)).collect();
        assert_close(&launch_cube(CubeFormula::WindowMax, &gpu_in, 5), &cpu_hi);

        let mut lowest = Lowest::new(5);
        let cpu_lo: Vec<f64> = values.iter().copied().map(|v| lowest.feed(v)).collect();
        assert_close(&launch_cube(CubeFormula::WindowMin, &gpu_in, 5), &cpu_lo);

        let mut wma = Wma::new(4);
        let cpu_wma: Vec<f64> = values.iter().copied().map(|v| wma.feed(v)).collect();
        assert_close(
            &launch_cube(CubeFormula::WindowWeighted, &gpu_in, 4),
            &cpu_wma,
        );

        let mut ema = Ema::new(5);
        assert_close(&launch_cube(CubeFormula::Ema, &gpu_in, 5), &cpu(&values, |v| ema.feed(v)));
        let mut rma = Rma::new(5);
        assert_close(&launch_cube(CubeFormula::Rma, &gpu_in, 5), &cpu(&values, |v| rma.feed(v)));
        let mut dema = Dema::new(5);
        assert_close(&launch_cube(CubeFormula::Dema, &gpu_in, 5), &cpu(&values, |v| dema.feed(v)));
        let mut tema = Tema::new(5);
        assert_close(&launch_cube(CubeFormula::Tema, &gpu_in, 5), &cpu(&values, |v| tema.feed(v)));
        let mut tma = Tma::new(5);
        assert_close(&launch_cube(CubeFormula::Tma, &gpu_in, 5), &cpu(&values, |v| tma.feed(v)));
        let mut trima = Trima::new(5);
        assert_close(&launch_cube(CubeFormula::Tma, &gpu_in, 5), &cpu(&values, |v| trima.feed(v)));
        let mut hma = Hma::new(8);
        assert_close(&launch_cube(CubeFormula::Hma, &gpu_in, 8), &cpu(&values, |v| hma.feed(v)));
        let mut alma = Alma::new(5);
        assert_close(&launch_cube(CubeFormula::Alma, &gpu_in, 5), &cpu(&values, |v| alma.feed(v)));
        let mut t3 = T3::new(5);
        assert_close(&launch_cube(CubeFormula::T3, &gpu_in, 5), &cpu(&values, |v| t3.feed(v)));
        let mut md = McGinleyDynamic::new(5);
        assert_close(
            &launch_cube(CubeFormula::Mcginley, &gpu_in, 5),
            &cpu(&values, |v| md.feed(v)),
        );
        let mut roc = Roc::new(5, false);
        assert_close(&launch_cube(CubeFormula::Roc, &gpu_in, 5), &cpu(&values, |v| roc.feed(v)));
        let mut rsi = Rsi::new(5);
        assert_close(&launch_cube(CubeFormula::Rsi, &gpu_in, 5), &cpu(&values, |v| rsi.feed(v)));
        let mut cmo = Cmo::new(5);
        assert_close(&launch_cube(CubeFormula::Cmo, &gpu_in, 5), &cpu(&values, |v| cmo.feed(v)));
        let mut bias = Bias::new(5);
        assert_close(&launch_cube(CubeFormula::Bias, &gpu_in, 5), &cpu(&values, |v| bias.feed(v)));

        assert!(launch_cube(CubeFormula::WindowMean, &[], 5).is_empty());
    }
}
