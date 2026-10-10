//! Calendar formulas, codes 700..=709, on the time adapter.
//!
//! Layout: the host reduces every unix-ms timestamp to exact small integers
//! ([`GpuTimes`]: weekday, hour, month, day of month, days in month, weekday occurrence,
//! days to the nearest quarter boundary). The kernel only does the arithmetic of each
//! `*Effect` / `*Flags` feed on those columns. The window `w` is `params.period`, clamped
//! as each CPU constructor clamps it. UNTESTED on GPU (no GPU on the authoring box).

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::gpu::CubeParams;
use super::gpu_sample::GpuTimes;
use super::CubeFormula;

/// 700 hour, 701 week in month, 702 weekday occurrence, 703 month turn, 704 quarter turn,
/// 705 holiday/weekend proximity, 706 start/end of month, 707 start/end of quarter,
/// 708 start/end of week, 709 hour sin/cos. Two-column formulas write column 1 at `n + i`.
#[cube]
fn cal_scan(
    weekday: &[f32],
    hour: &[f32],
    month: &[f32],
    dom: &[f32],
    dim: &[f32],
    occ: &[f32],
    qnear: &[f32],
    output: &mut [f32],
    formula: u32,
    window: u32,
) {
    let n = weekday.len();
    let mut w = window as f32;
    if w < 1.0f32 {
        w = 1.0f32;
    }
    for i in 0..n {
        let wd = weekday[i];
        let d = dom[i];
        let m = month[i];
        let dm = dim[i];
        let mut v0 = 0.0f32;
        let mut v1 = 0.0f32;
        if formula == 700u32 {
            v0 = hour[i];
        } else if formula == 701u32 {
            let q = ((d as u32 - 1u32) / 7u32) + 1u32;
            v0 = q as f32;
        } else if formula == 702u32 {
            v0 = occ[i];
        } else if formula == 703u32 {
            let mut wc = w;
            if wc > 10.0f32 {
                wc = 10.0f32;
            }
            let ds = d - 1.0f32;
            let de = dm - d;
            let mut near = ds;
            if de < near {
                near = de;
            }
            if near <= wc {
                v0 = 1.0f32 - near / wc;
            }
        } else if formula == 704u32 {
            let mut wc = w;
            if wc > 15.0f32 {
                wc = 15.0f32;
            }
            let near = qnear[i];
            if near <= wc {
                v0 = 1.0f32 - near / wc;
            }
        } else if formula == 705u32 {
            let mut wc = w;
            if wc > 5.0f32 {
                wc = 5.0f32;
            }
            // Distance to the weekend by weekday: Mon 1, Tue 2, Wed 3, Thu 2, Fri 1, Sat 0, Sun 0.
            let mut dist = 3.0f32;
            if wd == 0.0f32 {
                dist = 1.0f32;
            } else if wd == 1.0f32 {
                dist = 2.0f32;
            } else if wd == 2.0f32 {
                dist = 3.0f32;
            } else if wd == 3.0f32 {
                dist = 2.0f32;
            } else if wd == 4.0f32 {
                dist = 1.0f32;
            } else if wd == 5.0f32 {
                dist = 0.0f32;
            } else if wd == 6.0f32 {
                dist = 0.0f32;
            }
            if dist <= wc {
                v0 = 1.0f32 - dist / wc;
            }
        } else if formula == 706u32 {
            let mut wc = w;
            if wc > 5.0f32 {
                wc = 5.0f32;
            }
            if d - 1.0f32 <= wc {
                v0 = 1.0f32;
            }
            if dm - d <= wc {
                v1 = 1.0f32;
            }
        } else if formula == 707u32 {
            let mut wc = w;
            if wc > 7.0f32 {
                wc = 7.0f32;
            }
            let qsm = (((m as u32 - 1u32) / 3u32) * 3u32 + 1u32) as f32;
            if m == qsm && d <= wc {
                v0 = 1.0f32;
            }
            if m == qsm + 2.0f32 && (dm - d + 1.0f32) <= wc {
                v1 = 1.0f32;
            }
        } else if formula == 708u32 {
            let mut wc = w;
            if wc > 3.0f32 {
                wc = 3.0f32;
            }
            let mut ds = wd;
            if ds > 6.0f32 {
                ds = 6.0f32;
            }
            let de = 6.0f32 - ds;
            if ds <= wc {
                v0 = 1.0f32;
            }
            if de <= wc {
                v1 = 1.0f32;
            }
        } else if formula == 709u32 {
            let angle = 6.2831855f32 * hour[i] / 24.0f32;
            v0 = angle.sin();
            v1 = angle.cos();
        }
        output[i] = v0;
        output[n + i] = v1;
    }
}

#[cube(launch_unchecked)]
fn cal_map(
    weekday: &[f32],
    hour: &[f32],
    month: &[f32],
    dom: &[f32],
    dim: &[f32],
    occ: &[f32],
    qnear: &[f32],
    output: &mut [f32],
    formula: u32,
    window: u32,
) {
    cal_scan(weekday, hour, month, dom, dim, occ, qnear, output, formula, window);
}

/// Run a calendar formula (code 700..=709). Returns one column, or two for 706..=709.
pub fn launch_cube_calendar(
    formula: CubeFormula,
    times: &GpuTimes,
    params: CubeParams,
) -> Vec<Vec<f32>> {
    let n = times.len();
    if n == 0 {
        return Vec::new();
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let wd = client.create_from_slice(f32::as_bytes(&times.weekday));
    let hr = client.create_from_slice(f32::as_bytes(&times.hour));
    let mo = client.create_from_slice(f32::as_bytes(&times.month));
    let dm = client.create_from_slice(f32::as_bytes(&times.dom));
    let dim = client.create_from_slice(f32::as_bytes(&times.dim));
    let occ = client.create_from_slice(f32::as_bytes(&times.occ));
    let qn = client.create_from_slice(f32::as_bytes(&times.qnear));
    let out = client.empty(2 * n * core::mem::size_of::<f32>());
    unsafe {
        cal_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(wd, n),
            BufferArg::from_raw_parts(hr, n),
            BufferArg::from_raw_parts(mo, n),
            BufferArg::from_raw_parts(dm, n),
            BufferArg::from_raw_parts(dim, n),
            BufferArg::from_raw_parts(occ, n),
            BufferArg::from_raw_parts(qn, n),
            BufferArg::from_raw_parts(out.clone(), 2 * n),
            formula.code(),
            params.period,
        );
    }
    let bytes = client.read_one_unchecked(out);
    let flat = f32::from_bytes(&bytes).to_vec();
    let cols = formula.output_count() as usize;
    (0..cols).map(|c| flat[c * n..(c + 1) * n].to_vec()).collect()
}

/// Same as [`launch_cube_calendar`], for callers that go through the timed entry.
pub fn launch_cube_timed_columns(
    formula: CubeFormula,
    times: &GpuTimes,
    params: CubeParams,
) -> Vec<Vec<f32>> {
    launch_cube_calendar(formula, times, params)
}
