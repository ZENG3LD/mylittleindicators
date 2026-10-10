//! Keyed-state formulas over a [`GpuKeyedFrame`]: `1400` weight drift (CompositeWeightDrift and
//! IndexComponentDrift: max relative change between consecutive snapshots over keys present in both),
//! `1401` index correlation breakdown (min Pearson correlation between key series over the last
//! `period` snapshots, absent = 0; constant series give 1.0 like the CPU). UNTESTED on GPU.
//! The CPU matrix grid of `1401` is not produced; only the scalar minimum.

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::gpu::CubeParams;
use super::keyed_frame::GpuKeyedFrame;
use super::CubeFormula;

#[cube]
fn ky_scan(val: &[f32], pres: &[f32], out: &mut [f32], n: u32, k: u32, formula: u32, window: u32) {
    let nu = n as usize;
    let ku = k as usize;
    let wu = window as usize;
    for t in 0..nu {
        let mut res = 0.0f32;
        if formula == 1400u32 {
            if t >= 1usize {
                for q in 0..ku {
                    if pres[t * ku + q] > 0.5f32 && pres[(t - 1usize) * ku + q] > 0.5f32 {
                        let old = val[(t - 1usize) * ku + q];
                        if old.abs() > 1.0e-12f32 {
                            let d = (val[t * ku + q] - old).abs() / old.abs();
                            if d > res {
                                res = d;
                            }
                        }
                    }
                }
            }
        } else {
            res = 1.0f32;
            let mut len = t + 1usize;
            if len > wu {
                len = wu;
            }
            if len >= 2usize {
                let lo = t + 1usize - len;
                let lf = len as f32;
                for i in 0..ku {
                    for j in (i + 1usize)..ku {
                        let mut ma = 0.0f32;
                        let mut mb = 0.0f32;
                        for s in lo..(t + 1usize) {
                            ma = ma + val[s * ku + i];
                            mb = mb + val[s * ku + j];
                        }
                        ma = ma / lf;
                        mb = mb / lf;
                        let mut nm = 0.0f32;
                        let mut da2 = 0.0f32;
                        let mut db2 = 0.0f32;
                        for s in lo..(t + 1usize) {
                            let da = val[s * ku + i] - ma;
                            let db = val[s * ku + j] - mb;
                            nm = nm + da * db;
                            da2 = da2 + da * da;
                            db2 = db2 + db * db;
                        }
                        let den = (da2 * db2).sqrt();
                        let mut cr = 1.0f32;
                        if den >= 1.0e-12f32 {
                            cr = (nm / den).max(-1.0f32).min(1.0f32);
                        }
                        if cr < res {
                            res = cr;
                        }
                    }
                }
            }
        }
        out[t] = res;
    }
}

#[cube(launch_unchecked)]
fn ky_map(val: &[f32], pres: &[f32], out: &mut [f32], n: u32, k: u32, formula: u32, window: u32) {
    ky_scan(val, pres, out, n, k, formula, window);
}

pub fn launch_cube_keyed(formula: CubeFormula, fr: &GpuKeyedFrame, params: CubeParams) -> Vec<Vec<f32>> {
    let n = fr.n;
    if n == 0 {
        return Vec::new();
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let up = |s: &Vec<f32>| client.create_from_slice(f32::as_bytes(s));
    let out = client.empty(n * 4);
    unsafe {
        ky_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(up(&fr.val), fr.val.len()),
            BufferArg::from_raw_parts(up(&fr.pres), fr.pres.len()),
            BufferArg::from_raw_parts(out.clone(), n),
            n as u32,
            fr.k as u32,
            formula.code(),
            params.period.max(2),
        );
    }
    vec![f32::from_bytes(&client.read_one_unchecked(out)).to_vec()]
}
