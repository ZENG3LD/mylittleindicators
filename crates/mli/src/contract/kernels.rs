//! One CubeCL kernel for every [`CubeFormula`].
//!
//! The CPU cores stay f64. This kernel is the f32 batch map of the same
//! lane. It is compiled only with the `gpu` feature, which links the CubeCL
//! wgpu runtime. A new formula is another arm here, not a new kernel file
//! and not a trait impl on the indicator.

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::CubeFormula;

#[cube(launch_unchecked)]
fn lane_map(input: &[f32], output: &mut [f32], period: u32, formula: u32) {
    let i = ABSOLUTE_POS;
    if i < input.len() {
        if formula == 0u32 {
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
    use crate::indicators::average::sma::Sma;
    use crate::indicators::average::wma::Wma;
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
        (0..n).map(|i| (i as f64) * 0.5 + 1.0).collect()
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

        assert!(launch_cube(CubeFormula::WindowMean, &[], 5).is_empty());
    }
}
