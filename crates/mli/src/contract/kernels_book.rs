//! Order-book snapshot formulas, codes 960..=979, one sequential scan over a [`GpuBookFrame`].
//! Output column `c` of snapshot `i` is `out[c * n + i]` (up to three columns). Parameters:
//! `period` window / levels, `a` / `b` thresholds, `levels` for the wall detector.
//! UNTESTED on GPU (no GPU on the authoring box).

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::book_frame::GpuBookFrame;
use super::gpu::CubeParams;
use super::CubeFormula;

/// k-th smallest (0-based) of the first `len` values of `src` (rank count, O(len^2)).
#[cube]
fn kth_scratch(src: &mut [f32], len: usize, k: u32) -> f32 {
    let mut kk = k;
    if kk >= len as u32 {
        kk = (len - 1) as u32;
    }
    let mut res = src[0];
    for j in 0..len {
        let x = src[j];
        let mut lt = 0u32;
        let mut le = 0u32;
        for m in 0..len {
            let y = src[m];
            if y < x {
                lt = lt + 1u32;
            }
            if y <= x {
                le = le + 1u32;
            }
        }
        if lt <= kk && kk < le {
            res = x;
        }
    }
    res
}

/// 960 bid/ask bounce rate (`period` snapshots), 961 mid-price velocity (`period`), 962 book
/// depth change `[bid, ask]` (`period` levels), 963 wall detector `[bid_price, ask_price,
/// total_size]` (`period` history window >= 10, `a` percentile clamped to 50..=99.9, `levels`
/// sampled levels), 964 best-level size volatility `[std_bid, std_ask, max]` (`period`),
/// 965 price level density `[bid, ask, avg]` (`period` top n >= 2), 966 liquidity sweep
/// `[direction, magnitude]`.
#[cube]
fn book_scan(
    bpx: &[f32],
    bsz: &[f32],
    apx: &[f32],
    asz: &[f32],
    nb: &[f32],
    na: &[f32],
    ts: &[f32],
    scr: &mut [f32],
    out: &mut [f32],
    formula: u32,
    period: u32,
    depth: u32,
    levels: u32,
    a: f32,
) {
    let n = nb.len();
    let d = depth as usize;
    let mut h0 = 0.0f32;
    let mut h1 = 0.0f32;
    let mut h2 = 0.0f32;
    let mut pb = 0.0f32;
    let mut pa = 0.0f32;
    let mut has = false;
    for i in 0..n {
        let mut v0 = 0.0f32;
        let mut v1 = 0.0f32;
        let mut v2 = 0.0f32;
        let nbi = nb[i] as usize;
        let nai = na[i] as usize;
        if formula == 960u32 {
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            let mut lo = 0usize;
            if i + 1 > w {
                lo = i + 1 - w;
            }
            let cnt = i + 1 - lo;
            if cnt >= 2 {
                let mut bounces = 0.0f32;
                for j in (lo + 1)..(i + 1) {
                    let mut cb = 0.0f32;
                    let mut ca = 0.0f32;
                    let mut qb = 0.0f32;
                    let mut qa = 0.0f32;
                    if nb[j] > 0.0f32 {
                        cb = bpx[j * d];
                    }
                    if na[j] > 0.0f32 {
                        ca = apx[j * d];
                    }
                    if nb[j - 1] > 0.0f32 {
                        qb = bpx[(j - 1) * d];
                    }
                    if na[j - 1] > 0.0f32 {
                        qa = apx[(j - 1) * d];
                    }
                    if cb != qb || ca != qa {
                        bounces = bounces + 1.0f32;
                    }
                }
                let span = (ts[i] - ts[lo]) / 1000.0f32;
                if span > 1.0e-9f32 {
                    v0 = bounces / span;
                } else {
                    v0 = bounces / ((cnt - 1) as f32);
                }
            }
        } else if formula == 961u32 {
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            if nbi > 0 && nai > 0 {
                let mut count = 1usize;
                let mut o = i;
                let mut j = i;
                while count < w && j > 0 {
                    j = j - 1;
                    if nb[j] > 0.0f32 && na[j] > 0.0f32 {
                        count = count + 1;
                        o = j;
                    }
                }
                if count >= 2 {
                    let mid_n = (bpx[i * d] + apx[i * d]) / 2.0f32;
                    let mid_o = (bpx[o * d] + apx[o * d]) / 2.0f32;
                    let dp = mid_n - mid_o;
                    let dt = ts[i] - ts[o];
                    if dt > 0.0f32 {
                        h0 = dp / (dt / 1000.0f32);
                    } else {
                        h0 = dp;
                    }
                }
            }
            v0 = h0;
        } else if formula == 962u32 {
            let mut lv = period as usize;
            if lv < 1 {
                lv = 1;
            }
            if i > 0 {
                let mut cb = 0.0f32;
                let mut ca = 0.0f32;
                let mut qb = 0.0f32;
                let mut qa = 0.0f32;
                for l in 0..lv {
                    if l < nbi {
                        cb = cb + bsz[i * d + l];
                    }
                    if l < nai {
                        ca = ca + asz[i * d + l];
                    }
                    if l < (nb[i - 1] as usize) {
                        qb = qb + bsz[(i - 1) * d + l];
                    }
                    if l < (na[i - 1] as usize) {
                        qa = qa + asz[(i - 1) * d + l];
                    }
                }
                v0 = cb - qb;
                v1 = ca - qa;
            }
        } else if formula == 963u32 {
            let mut wh = period as usize;
            if wh < 10 {
                wh = 10;
            }
            let mut pct = a;
            if pct < 50.0f32 {
                pct = 50.0f32;
            }
            if pct > 99.9f32 {
                pct = 99.9f32;
            }
            let mut ls = levels as usize;
            if ls < 1 {
                ls = 1;
            }
            // values pushed per snapshot: bids first, then asks, `2 * ls` at most
            let mut acc = 0usize;
            let mut j0 = i;
            let mut go = true;
            let mut j = i + 1;
            while go && j > 0 {
                j = j - 1;
                let mut cb = nb[j] as usize;
                if cb > 2 * ls {
                    cb = 2 * ls;
                }
                let mut ca = na[j] as usize;
                if ca > 2 * ls - cb {
                    ca = 2 * ls - cb;
                }
                acc = acc + cb + ca;
                j0 = j;
                if acc >= wh {
                    go = false;
                }
            }
            if acc >= wh {
                let mut skip = acc - wh;
                let mut fill = 0usize;
                for q in j0..(i + 1) {
                    let mut cb = nb[q] as usize;
                    if cb > 2 * ls {
                        cb = 2 * ls;
                    }
                    let mut ca = na[q] as usize;
                    if ca > 2 * ls - cb {
                        ca = 2 * ls - cb;
                    }
                    for l in 0..cb {
                        if skip > 0 {
                            skip = skip - 1;
                        } else {
                            scr[fill] = bsz[q * d + l];
                            fill = fill + 1;
                        }
                    }
                    for l in 0..ca {
                        if skip > 0 {
                            skip = skip - 1;
                        } else {
                            scr[fill] = asz[q * d + l];
                            fill = fill + 1;
                        }
                    }
                }
                let mut idx = ((pct / 100.0f32) * (wh as f32)) as usize;
                if idx > wh - 1 {
                    idx = wh - 1;
                }
                let thr = kth_scratch(scr, wh, idx as u32);
                let mut found = false;
                let mut best = 0.0f32;
                let mut bp = 0.0f32;
                let lb = if nbi < ls { nbi } else { ls };
                for l in 0..lb {
                    if bsz[i * d + l] >= thr {
                        if !found || bsz[i * d + l] >= best {
                            best = bsz[i * d + l];
                            bp = bpx[i * d + l];
                            found = true;
                        }
                    }
                }
                if found {
                    h0 = bp;
                    h2 = best;
                }
                found = false;
                best = 0.0f32;
                let la = if nai < ls { nai } else { ls };
                for l in 0..la {
                    if asz[i * d + l] >= thr {
                        if !found || asz[i * d + l] >= best {
                            best = asz[i * d + l];
                            bp = apx[i * d + l];
                            found = true;
                        }
                    }
                }
                if found {
                    h1 = bp;
                    // total size keeps the last wall sizes of both sides
                    pa = best;
                }
            }
            v0 = h0;
            v1 = h1;
            v2 = h2 + pa;
        } else if formula == 964u32 {
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            // bid side
            let mut cnt = 0usize;
            let mut sum = 0.0f32;
            let mut j = i + 1;
            while cnt < w && j > 0 {
                j = j - 1;
                if nb[j] > 0.0f32 {
                    cnt = cnt + 1;
                    sum = sum + bsz[j * d];
                }
            }
            if cnt >= 2 {
                let mean = sum / (cnt as f32);
                let mut ss = 0.0f32;
                let mut c2 = 0usize;
                let mut j2 = i + 1;
                while c2 < cnt && j2 > 0 {
                    j2 = j2 - 1;
                    if nb[j2] > 0.0f32 {
                        c2 = c2 + 1;
                        let dd = bsz[j2 * d] - mean;
                        ss = ss + dd * dd;
                    }
                }
                v0 = (ss / (cnt as f32)).sqrt();
            }
            cnt = 0usize;
            sum = 0.0f32;
            j = i + 1;
            while cnt < w && j > 0 {
                j = j - 1;
                if na[j] > 0.0f32 {
                    cnt = cnt + 1;
                    sum = sum + asz[j * d];
                }
            }
            if cnt >= 2 {
                let mean = sum / (cnt as f32);
                let mut ss = 0.0f32;
                let mut c2 = 0usize;
                let mut j2 = i + 1;
                while c2 < cnt && j2 > 0 {
                    j2 = j2 - 1;
                    if na[j2] > 0.0f32 {
                        c2 = c2 + 1;
                        let dd = asz[j2 * d] - mean;
                        ss = ss + dd * dd;
                    }
                }
                v1 = (ss / (cnt as f32)).sqrt();
            }
            v2 = v0;
            if v1 > v2 {
                v2 = v1;
            }
        } else if formula == 965u32 {
            let mut tn = period as usize;
            if tn < 2 {
                tn = 2;
            }
            let kb = if nbi < tn { nbi } else { tn };
            if kb >= 2 {
                let mut lo = bpx[i * d];
                let mut hi = lo;
                for l in 0..kb {
                    let p = bpx[i * d + l];
                    if p < lo {
                        lo = p;
                    }
                    if p > hi {
                        hi = p;
                    }
                }
                if hi - lo > 0.0f32 {
                    v0 = (kb as f32) / (hi - lo);
                }
            }
            let ka = if nai < tn { nai } else { tn };
            if ka >= 2 {
                let mut lo = apx[i * d];
                let mut hi = lo;
                for l in 0..ka {
                    let p = apx[i * d + l];
                    if p < lo {
                        lo = p;
                    }
                    if p > hi {
                        hi = p;
                    }
                }
                if hi - lo > 0.0f32 {
                    v1 = (ka as f32) / (hi - lo);
                }
            }
            v2 = (v0 + v1) / 2.0f32;
        } else if formula == 966u32 {
            if nbi > 0 && nai > 0 {
                let cb = bpx[i * d];
                let ca = apx[i * d];
                if !has {
                    has = true;
                } else {
                    h0 = 0.0f32;
                    h1 = 0.0f32;
                    if ca > pa {
                        h0 = 1.0f32;
                        h1 = ca - pa;
                    } else if cb < pb {
                        h0 = -1.0f32;
                        h1 = pb - cb;
                    }
                }
                pb = cb;
                pa = ca;
            }
            v0 = h0;
            v1 = h1;
        } else if formula == 967u32 {
            // spread distribution `[spread, percentile]`: only books with both sides and a
            // positive spread count; window `period` >= 2 of those spreads
            let mut w = period as usize;
            if w < 2 {
                w = 2;
            }
            if nbi > 0 && nai > 0 {
                let sp = apx[i * d] - bpx[i * d];
                if sp > 0.0f32 {
                    h0 = sp;
                    let mut cnt = 0usize;
                    let mut ge = 0.0f32;
                    let mut j = i + 1;
                    while cnt < w && j > 0 {
                        j = j - 1;
                        if nb[j] > 0.0f32 && na[j] > 0.0f32 {
                            let sj = apx[j * d] - bpx[j * d];
                            if sj > 0.0f32 {
                                cnt = cnt + 1;
                                if sj >= sp {
                                    ge = ge + 1.0f32;
                                }
                            }
                        }
                    }
                    h1 = ge / (cnt as f32) * 100.0f32;
                }
            }
            v0 = h0;
            v1 = h1;
        } else if formula == 968u32 {
            // layer concentration: Gini of the top `period` (>= 2) level sizes per side
            let mut tn = period as usize;
            if tn < 2 {
                tn = 2;
            }
            let kb = if nbi < tn { nbi } else { tn };
            if kb >= 2 {
                let mut sum = 0.0f32;
                for l in 0..kb {
                    sum = sum + bsz[i * d + l];
                }
                if sum > 0.0f32 {
                    let mut ws = 0.0f32;
                    for l in 0..kb {
                        let xv = bsz[i * d + l];
                        let mut rank = 1.0f32;
                        for m in 0..kb {
                            let y = bsz[i * d + m];
                            if y < xv {
                                rank = rank + 1.0f32;
                            } else if y == xv && m < l {
                                rank = rank + 1.0f32;
                            }
                        }
                        ws = ws + (2.0f32 * rank - (kb as f32) - 1.0f32) * xv;
                    }
                    v0 = ws / ((kb as f32) * sum);
                }
            }
            let ka = if nai < tn { nai } else { tn };
            if ka >= 2 {
                let mut sum = 0.0f32;
                for l in 0..ka {
                    sum = sum + asz[i * d + l];
                }
                if sum > 0.0f32 {
                    let mut ws = 0.0f32;
                    for l in 0..ka {
                        let xv = asz[i * d + l];
                        let mut rank = 1.0f32;
                        for m in 0..ka {
                            let y = asz[i * d + m];
                            if y < xv {
                                rank = rank + 1.0f32;
                            } else if y == xv && m < l {
                                rank = rank + 1.0f32;
                            }
                        }
                        ws = ws + (2.0f32 * rank - (ka as f32) - 1.0f32) * xv;
                    }
                    v1 = ws / ((ka as f32) * sum);
                }
            }
            v2 = v0;
            if v1 > v2 {
                v2 = v1;
            }
        } else if formula == 969u32 {
            // order book velocity: mean number of changed levels per snapshot over the last
            // `period` (>= 1) snapshot pairs
            let mut w = period as usize;
            if w < 1 {
                w = 1;
            }
            if i > 0 {
                let mut lo = 1usize;
                if i + 1 > w {
                    lo = i + 1 - w;
                }
                let mut total = 0.0f32;
                for j in lo..(i + 1) {
                    let pbn = nb[j - 1] as usize;
                    let cbn = nb[j] as usize;
                    let pan = na[j - 1] as usize;
                    let can = na[j] as usize;
                    let mb = if pbn > cbn { pbn } else { cbn };
                    for l in 0..mb {
                        if l >= pbn || l >= cbn {
                            total = total + 1.0f32;
                        } else if bpx[(j - 1) * d + l] != bpx[j * d + l] {
                            total = total + 1.0f32;
                        } else {
                            let mut df = bsz[(j - 1) * d + l] - bsz[j * d + l];
                            if df < 0.0f32 {
                                df = 0.0f32 - df;
                            }
                            if df > 1.0e-9f32 {
                                total = total + 1.0f32;
                            }
                        }
                    }
                    let ma = if pan > can { pan } else { can };
                    for l in 0..ma {
                        if l >= pan || l >= can {
                            total = total + 1.0f32;
                        } else if apx[(j - 1) * d + l] != apx[j * d + l] {
                            total = total + 1.0f32;
                        } else {
                            let mut df = asz[(j - 1) * d + l] - asz[j * d + l];
                            if df < 0.0f32 {
                                df = 0.0f32 - df;
                            }
                            if df > 1.0e-9f32 {
                                total = total + 1.0f32;
                            }
                        }
                    }
                }
                v0 = total / ((i + 1 - lo) as f32);
            }
        }
        out[i] = v0;
        out[n + i] = v1;
        out[2 * n + i] = v2;
    }
}

#[cube(launch_unchecked)]
fn book_map(
    bpx: &[f32],
    bsz: &[f32],
    apx: &[f32],
    asz: &[f32],
    nb: &[f32],
    na: &[f32],
    ts: &[f32],
    scr: &mut [f32],
    out: &mut [f32],
    formula: u32,
    period: u32,
    depth: u32,
    levels: u32,
    a: f32,
) {
    book_scan(bpx, bsz, apx, asz, nb, na, ts, scr, out, formula, period, depth, levels, a);
}

/// Run a book formula of codes 960..=979. One `Vec` per output column.
pub fn launch_cube_book(
    formula: CubeFormula,
    frame: &GpuBookFrame,
    params: CubeParams,
) -> Vec<Vec<f32>> {
    let n = frame.len();
    if n == 0 {
        return Vec::new();
    }
    if formula == CubeFormula::MarketMicroBk || formula == CubeFormula::OrderFlowImbBk {
        return super::kernels_hybrid::launch_market_micro(frame, formula.code());
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let up = |v: &Vec<f32>| client.create_from_slice(f32::as_bytes(v));
    let scr_len = (params.period.max(10) as usize).max(2);
    let scr = client.empty(scr_len * core::mem::size_of::<f32>());
    let out = client.empty(3 * n * core::mem::size_of::<f32>());
    unsafe {
        book_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(up(&frame.bid_px), frame.bid_px.len()),
            BufferArg::from_raw_parts(up(&frame.bid_sz), frame.bid_sz.len()),
            BufferArg::from_raw_parts(up(&frame.ask_px), frame.ask_px.len()),
            BufferArg::from_raw_parts(up(&frame.ask_sz), frame.ask_sz.len()),
            BufferArg::from_raw_parts(up(&frame.nb), n),
            BufferArg::from_raw_parts(up(&frame.na), n),
            BufferArg::from_raw_parts(up(&frame.ts), n),
            BufferArg::from_raw_parts(scr, scr_len),
            BufferArg::from_raw_parts(out.clone(), 3 * n),
            formula.code(),
            params.period,
            frame.depth as u32,
            params.levels,
            params.a,
        );
    }
    let bytes = client.read_one_unchecked(out);
    let flat = f32::from_bytes(&bytes).to_vec();
    let cols = formula.output_count() as usize;
    (0..cols).map(|k| flat[k * n..(k + 1) * n].to_vec()).collect()
}
