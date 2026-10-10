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
