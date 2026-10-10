//! Tick volume-profile / footprint formulas over a [`GpuProfileFrame`]. The host interns the price
//! buckets (`floor(price / bucket)`) of the whole stream to ids in ASCENDING bucket order, so the kernel can
//! keep bounded per-bucket `buy` / `sell` arrays and scan them in ascending price order.
//!
//! Tie rule (documented, deterministic): the CPU iterates a `HashMap`, whose order is random; the kernel
//! therefore resolves exact ties in favour of the LOWEST price bucket (first strictly-greater wins while
//! scanning ascending). Formulas (state accumulates over the whole tick stream, the tick path never calls
//! `close_bar`):
//! - `1405` FootprintPoc: price of the max-total-volume bucket after every tick.
//! - `1406` FootprintImbalance (`a` = threshold %): `[direction, imb_price, imb_pct]`.
//! - `1407` FootprintChart: `[net_delta, poc_price (0 on the tick path), total_volume]`.
//! - `1408` TpoSessionBalance (`a` = window ms, `b` = price bucket): `[balance price, max tick count, bucket count]`
//!   over the rolling time window (tie: lowest bucket).
//! - `1409` ValueAreaTracker (`a` = window ms, `b` = price bucket, `c` = value area fraction): `[poc, vah, val]`.
//! The CPU matrix grids (`profile_grid` / `footprint_grid`) are not produced. UNTESTED on GPU.

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::gpu::CubeParams;
use super::CubeFormula;
use crate::core::types::Tick;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpuProfileFrame {
    pub n: usize,
    pub k: usize,
    /// Per tick: interned bucket id (ascending bucket order), size, `1` buy / `0` sell.
    pub key: Vec<f32>,
    pub size: Vec<f32>,
    pub buy: Vec<f32>,
    /// Per key: bucket price `bucket * price_bucket`.
    pub kprice: Vec<f32>,
    /// Per tick: time in ms relative to the first tick (windowed rows).
    pub ts: Vec<f32>,
    /// Per key: id of the adjacent bucket `bucket + 1` / `bucket - 1` (`-1` = never traded).
    pub up: Vec<f32>,
    pub dn: Vec<f32>,
}

impl GpuProfileFrame {
    pub fn from_ticks(v: &[Tick], price_bucket: f64) -> Self {
        let pb = price_bucket.max(1e-9);
        let buckets: Vec<i64> = v.iter().map(|t| (t.price / pb).floor() as i64).collect();
        let mut uniq = buckets.clone();
        uniq.sort_unstable();
        uniq.dedup();
        GpuProfileFrame {
            n: v.len(),
            k: uniq.len().max(1),
            key: buckets.iter().map(|b| uniq.binary_search(b).unwrap_or(0) as f32).collect(),
            size: v.iter().map(|t| t.size as f32).collect(),
            buy: v.iter().map(|t| if t.is_buy { 1.0 } else { 0.0 }).collect(),
            kprice: uniq.iter().map(|b| (*b as f64 * pb) as f32).collect(),
            ts: {
                let t0 = v.first().map(|t| t.time).unwrap_or(0);
                v.iter().map(|t| (t.time - t0) as f32).collect()
            },
            up: uniq.iter().map(|b| uniq.binary_search(&(b + 1)).map(|i| i as f32).unwrap_or(-1.0)).collect(),
            dn: uniq.iter().map(|b| uniq.binary_search(&(b - 1)).map(|i| i as f32).unwrap_or(-1.0)).collect(),
        }
    }
}

#[cube]
fn pf_scan(
    key: &[f32],
    size: &[f32],
    buy: &[f32],
    kprice: &[f32],
    st: &mut [f32],
    out: &mut [f32],
    n: u32,
    k: u32,
    formula: u32,
    thr: f32,
) {
    let nu = n as usize;
    let ku = k as usize;
    for q in 0..(2usize * ku) {
        st[q] = 0.0f32;
    }
    let mut tb = 0.0f32;
    let mut ts = 0.0f32;
    let mut poc = 0.0f32;
    for i in 0..nu {
        let id = key[i] as usize;
        if buy[i] > 0.5f32 {
            st[id] = st[id] + size[i];
            tb = tb + size[i];
        } else {
            st[ku + id] = st[ku + id] + size[i];
            ts = ts + size[i];
        }
        let mut o0 = 0.0f32;
        let mut o1 = 0.0f32;
        let mut o2 = 0.0f32;
        if formula == 1405u32 {
            // max total volume; CPU `max_by` keeps scanning, we keep the lowest bucket on exact ties
            let mut best = 0.0f32;
            let mut has = false;
            for q in 0..ku {
                let tot = st[q] + st[ku + q];
                if tot > 0.0f32 || st[q] != 0.0f32 || st[ku + q] != 0.0f32 {
                    if !has || tot > best {
                        best = tot;
                        poc = kprice[q];
                        has = true;
                    }
                }
            }
            o0 = poc;
        } else if formula == 1406u32 {
            let mut mpct = 0.0f32;
            let mut mprice = 0.0f32;
            let mut mtot = 0.0f32;
            for q in 0..ku {
                let b = st[q];
                let s = st[ku + q];
                let tot = b + s;
                if tot > 0.0f32 {
                    let sp = ((b - s) / tot) * 100.0f32;
                    let stronger = sp.abs() > mpct.abs() + 1.0e-9f32;
                    let tie = sp.abs() > 1.0e-9f32 && (sp.abs() - mpct.abs()).abs() <= 1.0e-9f32 && tot > mtot;
                    if stronger || tie {
                        mpct = sp;
                        mprice = kprice[q];
                        mtot = tot;
                    }
                }
            }
            if mpct >= thr {
                o0 = 1.0f32;
            } else if mpct <= 0.0f32 - thr {
                o0 = -1.0f32;
            }
            o1 = mprice;
            o2 = mpct.abs();
        } else {
            o0 = tb - ts;
            o1 = 0.0f32;
            o2 = tb + ts;
        }
        out[i] = o0;
        out[nu + i] = o1;
        out[2usize * nu + i] = o2;
    }
}

#[cube(launch_unchecked)]
fn pf_map(
    key: &[f32],
    size: &[f32],
    buy: &[f32],
    kprice: &[f32],
    st: &mut [f32],
    out: &mut [f32],
    n: u32,
    k: u32,
    formula: u32,
    thr: f32,
) {
    pf_scan(key, size, buy, kprice, st, out, n, k, formula, thr);
}

pub fn launch_cube_profile(formula: CubeFormula, fr: &GpuProfileFrame, params: CubeParams) -> Vec<Vec<f32>> {
    let n = fr.n;
    if n == 0 {
        return Vec::new();
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let up = |s: &Vec<f32>| client.create_from_slice(f32::as_bytes(s));
    let st = client.empty(2 * fr.k * 4);
    let out = client.empty(3 * n * 4);
    unsafe {
        pf_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(up(&fr.key), n),
            BufferArg::from_raw_parts(up(&fr.size), n),
            BufferArg::from_raw_parts(up(&fr.buy), n),
            BufferArg::from_raw_parts(up(&fr.kprice), fr.k),
            BufferArg::from_raw_parts(st, 2 * fr.k),
            BufferArg::from_raw_parts(out.clone(), 3 * n),
            n as u32,
            fr.k as u32,
            formula.code(),
            params.a.clamp(0.0, 100.0),
        );
    }
    let flat = f32::from_bytes(&client.read_one_unchecked(out)).to_vec();
    let cols = formula.output_count() as usize;
    (0..cols).map(|c| flat[c * n..(c + 1) * n].to_vec()).collect()
}

#[cube]
fn pw_scan(
    key: &[f32],
    size: &[f32],
    ts: &[f32],
    kprice: &[f32],
    up: &[f32],
    dn: &[f32],
    st: &mut [f32],
    out: &mut [f32],
    n: u32,
    k: u32,
    formula: u32,
    win: f32,
    pb: f32,
    pct: f32,
) {
    let nu = n as usize;
    let ku = k as usize;
    let mut head = 0usize;
    for i in 0..nu {
        while head < i && ts[head] < ts[i] - win {
            head = head + 1usize;
        }
        for q in 0..ku {
            st[q] = 0.0f32;
        }
        for e in head..(i + 1usize) {
            let id = key[e] as usize;
            if formula == 1408u32 {
                st[id] = st[id] + 1.0f32;
            } else {
                st[id] = st[id] + size[e];
            }
        }
        // point of control: max volume / count, lowest bucket on exact ties
        let mut poc = 0usize;
        let mut best = 0.0f32;
        let mut has = false;
        let mut nb = 0.0f32;
        let mut tot = 0.0f32;
        for q in 0..ku {
            let present = st[q] != 0.0f32;
            if present {
                nb = nb + 1.0f32;
            }
            tot = tot + st[q];
            if present && (!has || st[q] > best) {
                best = st[q];
                has = true;
                poc = q;
            }
        }
        let mut o0 = 0.0f32;
        let mut o1 = 0.0f32;
        let mut o2 = 0.0f32;
        if formula == 1408u32 {
            o0 = kprice[poc] + pb / 2.0f32;
            o1 = best;
            o2 = nb;
        } else {
            let target = tot * pct;
            let mut acc = best;
            let mut upi = up[poc];
            let mut dni = dn[poc];
            let mut ups = 0.0f32;
            let mut dns = 0.0f32;
            let mut go = true;
            for _s in 0..(2usize * ku + 2usize) {
                if go && acc < target {
                    let mut uv = 0.0f32;
                    if upi >= 0.0f32 {
                        uv = st[upi as usize];
                    }
                    let mut dv = 0.0f32;
                    if dni >= 0.0f32 {
                        dv = st[dni as usize];
                    }
                    if uv == 0.0f32 && dv == 0.0f32 {
                        go = false;
                    } else if uv >= dv {
                        acc = acc + uv;
                        ups = ups + 1.0f32;
                        upi = up[upi as usize];
                    } else {
                        acc = acc + dv;
                        dns = dns + 1.0f32;
                        dni = dn[dni as usize];
                    }
                }
            }
            o0 = kprice[poc] + pb / 2.0f32;
            o1 = kprice[poc] + (ups + 1.0f32) * pb;
            o2 = kprice[poc] - dns * pb;
        }
        out[i] = o0;
        out[nu + i] = o1;
        out[2usize * nu + i] = o2;
    }
}

#[cube(launch_unchecked)]
fn pw_map(
    key: &[f32],
    size: &[f32],
    ts: &[f32],
    kprice: &[f32],
    up: &[f32],
    dn: &[f32],
    st: &mut [f32],
    out: &mut [f32],
    n: u32,
    k: u32,
    formula: u32,
    win: f32,
    pb: f32,
    pct: f32,
) {
    pw_scan(key, size, ts, kprice, up, dn, st, out, n, k, formula, win, pb, pct);
}

/// Windowed profile rows (`1408`, `1409`); `params.a` = window ms, `params.b` = price bucket,
/// `params.c` = value-area fraction.
pub fn launch_cube_profile_window(formula: CubeFormula, fr: &GpuProfileFrame, params: CubeParams) -> Vec<Vec<f32>> {
    let n = fr.n;
    if n == 0 {
        return Vec::new();
    }
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let up = |s: &Vec<f32>| client.create_from_slice(f32::as_bytes(s));
    let st = client.empty(fr.k * 4);
    let out = client.empty(3 * n * 4);
    unsafe {
        pw_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(up(&fr.key), n),
            BufferArg::from_raw_parts(up(&fr.size), n),
            BufferArg::from_raw_parts(up(&fr.ts), n),
            BufferArg::from_raw_parts(up(&fr.kprice), fr.k),
            BufferArg::from_raw_parts(up(&fr.up), fr.k),
            BufferArg::from_raw_parts(up(&fr.dn), fr.k),
            BufferArg::from_raw_parts(st, fr.k),
            BufferArg::from_raw_parts(out.clone(), 3 * n),
            n as u32,
            fr.k as u32,
            formula.code(),
            params.a.max(1.0),
            params.b.max(f32::EPSILON),
            params.c.clamp(f32::EPSILON, 1.0),
        );
    }
    let flat = f32::from_bytes(&client.read_one_unchecked(out)).to_vec();
    (0..3).map(|c| flat[c * n..(c + 1) * n].to_vec()).collect()
}
