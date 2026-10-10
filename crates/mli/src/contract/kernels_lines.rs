//! Cross-line combiner kernels: `LineCross` (M x N cross grid) and `PriceLineCross` (price vs N lines).
//!
//! The inner `LineProducer` trees are arbitrary catalog indicators resolved at run time, so the HOST
//! resolves them (each producer's Price-domain output columns, from any path: cube formula, CPU or
//! shader) and these kernels only combine the resolved line columns. Line matrices are line-major:
//! `lines[i * n + t]`. UNTESTED on GPU (no GPU on the authoring box).

use cubecl::prelude::*;
use cubecl::__private::Runtime;

/// Cross grid between `m` left lines and `nn` right lines. Output `out[(i * nn + j) * n + t]` is the
/// signal of pair `(i, j)` (`+1` up-cross, `-1` down-cross, `0`; sticky keeps the last non-zero).
/// Bar 0 has no previous bar, so every signal is 0. `sticky` is 0 / 1.
#[cube]
fn lc_scan(left: &[f32], right: &[f32], out: &mut [f32], n: u32, m: u32, nn: u32, sticky: u32) {
    let nu = n as usize;
    let mu = m as usize;
    let nnu = nn as usize;
    for i in 0..mu {
        for j in 0..nnu {
            let mut st = 0.0f32;
            for t in 0..nu {
                let mut sig = 0.0f32;
                if t > 0usize {
                    let a0 = left[i * nu + t - 1];
                    let a1 = left[i * nu + t];
                    let b0 = right[j * nu + t - 1];
                    let b1 = right[j * nu + t];
                    if a0 <= b0 && a1 > b1 {
                        sig = 1.0f32;
                    } else if a0 >= b0 && a1 < b1 {
                        sig = 0.0f32 - 1.0f32;
                    }
                }
                if sticky == 1u32 {
                    if sig != 0.0f32 {
                        st = sig;
                    }
                    out[(i * nnu + j) * nu + t] = st;
                } else {
                    out[(i * nnu + j) * nu + t] = sig;
                }
            }
        }
    }
}

#[cube(launch_unchecked)]
fn lc_map(left: &[f32], right: &[f32], out: &mut [f32], n: u32, m: u32, nn: u32, sticky: u32) {
    lc_scan(left, right, out, n, m, nn, sticky);
}

/// Combine resolved left / right line columns. Returns `m * nn` columns, pair `(i, j)` at `i * nn + j`
/// (column 0 is the CPU `signal`).
pub fn launch_line_cross(left: &[Vec<f32>], right: &[Vec<f32>], sticky: bool) -> Vec<Vec<f32>> {
    let (m, nn) = (left.len(), right.len());
    if m == 0 || nn == 0 || left[0].is_empty() {
        return Vec::new();
    }
    let n = left[0].len();
    let lf: Vec<f32> = left.iter().flatten().copied().collect();
    let rf: Vec<f32> = right.iter().flatten().copied().collect();
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let lb = client.create_from_slice(f32::as_bytes(&lf));
    let rb = client.create_from_slice(f32::as_bytes(&rf));
    let ob = client.empty(m * nn * n * 4);
    unsafe {
        lc_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(lb, lf.len()),
            BufferArg::from_raw_parts(rb, rf.len()),
            BufferArg::from_raw_parts(ob.clone(), m * nn * n),
            n as u32,
            m as u32,
            nn as u32,
            sticky as u32,
        );
    }
    let flat = f32::from_bytes(&client.read_one_unchecked(ob)).to_vec();
    flat.chunks(n).map(|c| c.to_vec()).collect()
}

/// Touch modes of `PriceLineCross` (`TouchMode`): `0` CloseAbove, `1` CloseBelow, `2` WickThrough,
/// `3` WickReject, `4` Touch (`tol`), `5` WithCandle (`pattern[t] > 0` marks a detected candle pattern).
/// `ready_from` is the first bar at which the line producer is ready (signals are 0 before it); the
/// previous-side state tracks every bar. Output `out[i * n + t]`, line `i`.
#[cube]
fn plc_scan(
    h: &[f32],
    l: &[f32],
    c: &[f32],
    lines: &[f32],
    pattern: &[f32],
    out: &mut [f32],
    n: u32,
    nl: u32,
    mode: u32,
    tol: f32,
    ready_from: u32,
) {
    let nu = n as usize;
    let nlu = nl as usize;
    for i in 0..nlu {
        // 0 = no previous bar, 1 = previous close below the level (not above), 2 = previous close above
        let mut pv = 0u32;
        for t in 0..nu {
            let lv = lines[i * nu + t];
            let mut sig = 0.0f32;
            if t >= (ready_from as usize) {
                if mode == 0u32 {
                    if pv == 1u32 && c[t] > lv {
                        sig = 1.0f32;
                    }
                } else if mode == 1u32 {
                    if pv == 2u32 && c[t] < lv {
                        sig = 0.0f32 - 1.0f32;
                    }
                } else if mode == 2u32 {
                    if l[t] <= lv && h[t] >= lv {
                        if c[t] >= lv {
                            sig = 1.0f32;
                        } else {
                            sig = 0.0f32 - 1.0f32;
                        }
                    } else if h[t] > lv && pv == 1u32 {
                        sig = 1.0f32;
                    } else if l[t] < lv && pv == 2u32 {
                        sig = 0.0f32 - 1.0f32;
                    }
                } else if mode == 3u32 {
                    let bull = l[t] < lv && c[t] > lv && (pv == 2u32 || pv == 0u32);
                    let bear = h[t] > lv && c[t] < lv && (pv == 1u32 || pv == 0u32);
                    if bull {
                        sig = 1.0f32;
                    } else if bear {
                        sig = 0.0f32 - 1.0f32;
                    }
                } else if mode == 4u32 {
                    if (h[t] - lv).abs() <= tol || (l[t] - lv).abs() <= tol {
                        sig = 1.0f32;
                    }
                } else {
                    let crossed = (pv == 1u32 && c[t] > lv) || (pv == 2u32 && c[t] < lv);
                    if crossed && pattern[t] > 0.0f32 {
                        if c[t] > lv {
                            sig = 1.0f32;
                        } else {
                            sig = 0.0f32 - 1.0f32;
                        }
                    }
                }
            }
            out[i * nu + t] = sig;
            if c[t] > lv {
                pv = 2u32;
            } else {
                pv = 1u32;
            }
        }
    }
}

#[cube(launch_unchecked)]
fn plc_map(
    h: &[f32],
    l: &[f32],
    c: &[f32],
    lines: &[f32],
    pattern: &[f32],
    out: &mut [f32],
    n: u32,
    nl: u32,
    mode: u32,
    tol: f32,
    ready_from: u32,
) {
    plc_scan(h, l, c, lines, pattern, out, n, nl, mode, tol, ready_from);
}

/// Price vs resolved level lines (`lines[i]` per level). `pattern` is the per-bar candle-pattern hit
/// (only read in mode 5; pass zeros otherwise). Returns one signal column per line.
pub fn launch_price_line_cross(
    h: &[f32],
    l: &[f32],
    c: &[f32],
    lines: &[Vec<f32>],
    mode: u32,
    tol: f32,
    pattern: &[f32],
    ready_from: u32,
) -> Vec<Vec<f32>> {
    let n = c.len();
    let nl = lines.len();
    if n == 0 || nl == 0 {
        return Vec::new();
    }
    let lf: Vec<f32> = lines.iter().flatten().copied().collect();
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let up = |s: &[f32]| client.create_from_slice(f32::as_bytes(s));
    let (hb, lb, cb, lnb, pb) = (up(h), up(l), up(c), up(&lf), up(pattern));
    let ob = client.empty(nl * n * 4);
    unsafe {
        plc_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(hb, n),
            BufferArg::from_raw_parts(lb, n),
            BufferArg::from_raw_parts(cb, n),
            BufferArg::from_raw_parts(lnb, lf.len()),
            BufferArg::from_raw_parts(pb, pattern.len().max(1)),
            BufferArg::from_raw_parts(ob.clone(), nl * n),
            n as u32,
            nl as u32,
            mode,
            tol,
            ready_from,
        );
    }
    let flat = f32::from_bytes(&client.read_one_unchecked(ob)).to_vec();
    flat.chunks(n).map(|c| c.to_vec()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UNTESTED on GPU (no GPU on the authoring box): cross grid and touch signals against scalar references.
    #[test]
    fn line_cross_and_touch_match_reference() {
        let n = 120usize;
        let a: Vec<f32> = (0..n).map(|i| 100.0 + (i as f32 * 0.21).sin() * 4.0).collect();
        let b: Vec<f32> = (0..n).map(|i| 100.0 + (i as f32 * 0.13).cos() * 3.0).collect();
        let g = launch_line_cross(&[a.clone()], &[b.clone()], false);
        for t in 1..n {
            let want = if a[t - 1] <= b[t - 1] && a[t] > b[t] {
                1.0
            } else if a[t - 1] >= b[t - 1] && a[t] < b[t] {
                -1.0
            } else {
                0.0
            };
            assert_eq!(g[0][t], want);
        }
        let h: Vec<f32> = a.iter().map(|v| v + 1.0).collect();
        let l: Vec<f32> = a.iter().map(|v| v - 1.0).collect();
        let r = launch_price_line_cross(&h, &l, &a, &[b.clone()], 0, 0.0, &vec![0.0; n], 0);
        for t in 1..n {
            let want = if a[t - 1] <= b[t - 1] && a[t] > b[t] { 1.0 } else { 0.0 };
            assert_eq!(r[0][t], want);
        }
    }
}
