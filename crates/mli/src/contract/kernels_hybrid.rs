//! Hybrid tick + book formulas: `HiddenLiquidityDetector` (997), `TradeBookAbsorption` (998),
//! `SweepImpactAnalyzer` (999). One sequential pass over trades; the rolling cumulative sums are
//! recomputed from the per-trade output column over the last `period` trades. UNTESTED on GPU.
//!
//! Parameters: `period` = rolling window (>= 1), `a` = price bucket (997), `b` = absorption ratio (998).
//! Price comparisons are in `f32` (the CPU `1e-9` equality test becomes `f32` equality of the converted
//! prices; distinct prices closer than one `f32` ulp compare equal).

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::gpu::CubeParams;
use super::hybrid_frame::GpuHybridFrame;
use super::CubeFormula;

#[cube]
fn hy_scan(
    tpx: &[f32],
    tsz: &[f32],
    tbuy: &[f32],
    bpx: &[f32],
    bsz: &[f32],
    apx: &[f32],
    asz: &[f32],
    nb: &[f32],
    na: &[f32],
    out: &mut [f32],
    n: u32,
    depth: u32,
    formula: u32,
    window: u32,
    a: f32,
    b: f32,
) {
    let nu = n as usize;
    let du = depth as usize;
    let wu = window as usize;
    for i in 0..nu {
        let buy = tbuy[i] > 0.5f32;
        let mut cnt = nb[i] as usize;
        if buy {
            cnt = na[i] as usize;
        }
        let side = if buy { 1.0f32 } else { -1.0f32 };
        let mut o0 = 0.0f32;
        let mut o1 = 0.0f32;
        let mut o2 = 0.0f32;
        if formula == 997u32 {
            let bucket = a.max(1.0e-12f32);
            let mut vis = 0.0f32;
            for l in 0..du {
                if l < cnt {
                    let mut px = bpx[i * du + l];
                    let mut sz = bsz[i * du + l];
                    if buy {
                        px = apx[i * du + l];
                        sz = asz[i * du + l];
                    }
                    if (px - tpx[i]).abs() / bucket < 1.0f32 {
                        vis = vis + sz;
                    }
                }
            }
            let hidden = (tsz[i] - vis).max(0.0f32);
            o1 = hidden;
            if hidden > 0.0f32 {
                o0 = side;
            }
            out[n as usize + i] = hidden;
            let mut lo = 0usize;
            if i + 1usize > wu {
                lo = i + 1usize - wu;
            }
            let mut cs = 0.0f32;
            for j in lo..(i + 1usize) {
                cs = cs + out[nu + j];
            }
            o2 = cs;
        } else if formula == 998u32 {
            let mut vis = 0.0f32;
            let mut lp = tpx[i];
            if cnt > 0usize {
                vis = bsz[i * du];
                lp = bpx[i * du];
                if buy {
                    vis = asz[i * du];
                    lp = apx[i * du];
                }
            }
            let thr = vis * b;
            let mut ab = 0.0f32;
            if (tpx[i] - lp).abs() < 1.0e-9f32 && vis > 0.0f32 && tsz[i] > thr {
                ab = (tsz[i] - thr).max(0.0f32);
            }
            o1 = ab;
            if ab > 0.0f32 {
                o0 = side;
            }
            out[nu + i] = ab;
            let mut lo = 0usize;
            if i + 1usize > wu {
                lo = i + 1usize - wu;
            }
            let mut cs = 0.0f32;
            for j in lo..(i + 1usize) {
                cs = cs + out[nu + j];
            }
            o2 = cs;
        } else {
            let mut init = tpx[i];
            if cnt > 0usize {
                init = bpx[i * du];
                if buy {
                    init = apx[i * du];
                }
            }
            let mut rem = tsz[i];
            let mut swept = 0.0f32;
            let mut fin = init;
            let mut go = true;
            for l in 0..du {
                if l < cnt && go {
                    if rem <= 0.0f32 {
                        go = false;
                    } else {
                        let mut px = bpx[i * du + l];
                        let mut sz = bsz[i * du + l];
                        if buy {
                            px = apx[i * du + l];
                            sz = asz[i * du + l];
                        }
                        rem = rem - sz;
                        swept = swept + 1.0f32;
                        fin = px;
                    }
                }
            }
            if swept > 1.0f32 {
                o0 = side;
                o1 = swept;
                o2 = (fin - init).abs();
            }
        }
        out[i] = o0;
        out[nu + i] = o1;
        out[2usize * nu + i] = o2;
    }
}

#[cube(launch_unchecked)]
fn hy_map(
    tpx: &[f32],
    tsz: &[f32],
    tbuy: &[f32],
    bpx: &[f32],
    bsz: &[f32],
    apx: &[f32],
    asz: &[f32],
    nb: &[f32],
    na: &[f32],
    out: &mut [f32],
    n: u32,
    depth: u32,
    formula: u32,
    window: u32,
    a: f32,
    b: f32,
) {
    hy_scan(tpx, tsz, tbuy, bpx, bsz, apx, asz, nb, na, out, n, depth, formula, window, a, b);
}

/// Run a hybrid formula; returns `[side, last volume / levels swept, cumulative / slippage]`.
pub fn launch_cube_hybrid(formula: CubeFormula, fr: &GpuHybridFrame, params: CubeParams) -> Vec<Vec<f32>> {
    let n = fr.n;
    if n == 0 {
        return Vec::new();
    }
    let bk = &fr.book;
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let up = |s: &Vec<f32>| client.create_from_slice(f32::as_bytes(s));
    let out = client.empty(3 * n * 4);
    unsafe {
        hy_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(up(&fr.price), n),
            BufferArg::from_raw_parts(up(&fr.size), n),
            BufferArg::from_raw_parts(up(&fr.buy), n),
            BufferArg::from_raw_parts(up(&bk.bid_px), bk.bid_px.len()),
            BufferArg::from_raw_parts(up(&bk.bid_sz), bk.bid_sz.len()),
            BufferArg::from_raw_parts(up(&bk.ask_px), bk.ask_px.len()),
            BufferArg::from_raw_parts(up(&bk.ask_sz), bk.ask_sz.len()),
            BufferArg::from_raw_parts(up(&bk.nb), n),
            BufferArg::from_raw_parts(up(&bk.na), n),
            BufferArg::from_raw_parts(out.clone(), 3 * n),
            n as u32,
            bk.depth as u32,
            formula.code(),
            params.period.max(1),
            params.a,
            params.b,
        );
    }
    let flat = f32::from_bytes(&client.read_one_unchecked(out)).to_vec();
    vec![flat[0..n].to_vec(), flat[n..2 * n].to_vec(), flat[2 * n..3 * n].to_vec()]
}
