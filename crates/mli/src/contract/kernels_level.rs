//! Level-update formulas (keyed per-price-level state) over a [`GpuLevelFrame`]: `1403` iceberg detector.
//! The host flattens each book delta into one row per touched level, interning `(side, price bucket)`
//! keys to ids; the kernel keeps `last_size` / `replenishment_count` per key id in bounded arrays and
//! writes the `[side, price, count]` state after the last row of every delta. UNTESTED on GPU.

use cubecl::prelude::*;
use cubecl::__private::Runtime;

use super::gpu::CubeParams;
use super::CubeFormula;
use crate::core::types::OrderbookDelta;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpuLevelFrame {
    /// Number of deltas (output rows).
    pub n: usize,
    /// Number of interned keys.
    pub k: usize,
    /// Per level row: owning delta index, `1` bid / `-1` ask, key id, price, size.
    pub delta: Vec<f32>,
    pub side: Vec<f32>,
    pub key: Vec<f32>,
    pub price: Vec<f32>,
    pub size: Vec<f32>,
}

impl GpuLevelFrame {
    /// Bids before asks inside a delta (the CPU order); bucket = `floor(price / price_bucket)`.
    pub fn from_deltas(v: &[OrderbookDelta], price_bucket: f64) -> Self {
        let pb = price_bucket.max(1e-9);
        let mut keys: Vec<(i8, i64)> = Vec::new();
        let mut f = GpuLevelFrame { n: v.len(), ..Default::default() };
        for (i, d) in v.iter().enumerate() {
            let mut push = |side: i8, price: f64, size: f64| {
                let b = (price / pb).floor() as i64;
                let id = match keys.iter().position(|k| *k == (side, b)) {
                    Some(p) => p,
                    None => {
                        keys.push((side, b));
                        keys.len() - 1
                    }
                };
                f.delta.push(i as f32);
                f.side.push(side as f32);
                f.key.push(id as f32);
                f.price.push(price as f32);
                f.size.push(size as f32);
            };
            for l in &d.bids {
                push(1, l.price, l.size);
            }
            for l in &d.asks {
                push(-1, l.price, l.size);
            }
        }
        f.k = keys.len().max(1);
        f
    }
}

#[cube]
fn lv_scan(
    delta: &[f32],
    side: &[f32],
    key: &[f32],
    price: &[f32],
    size: &[f32],
    st: &mut [f32],
    out: &mut [f32],
    nd: u32,
    nr: u32,
    k: u32,
    thr: f32,
) {
    let ndu = nd as usize;
    let nru = nr as usize;
    let ku = k as usize;
    for q in 0..(2usize * ku) {
        st[q] = 0.0f32;
    }
    let mut ls = 0.0f32;
    let mut lp = 0.0f32;
    let mut lc = 0.0f32;
    let mut r = 0usize;
    for d in 0..ndu {
        let mut go = true;
        while go {
            if r < nru && (delta[r] as usize) == d {
                let id = key[r] as usize;
                let sz = size[r];
                if sz == 0.0f32 {
                    st[id] = 0.0f32;
                } else if st[id] == 0.0f32 && sz > 0.0f32 {
                    st[ku + id] = st[ku + id] + 1.0f32;
                    st[id] = sz;
                    if st[ku + id] >= thr {
                        ls = side[r];
                        lp = price[r];
                        lc = st[ku + id];
                    }
                } else {
                    st[id] = sz;
                }
                r = r + 1usize;
            } else {
                go = false;
            }
        }
        out[d] = ls;
        out[ndu + d] = lp;
        out[2usize * ndu + d] = lc;
    }
}

#[cube(launch_unchecked)]
fn lv_map(
    delta: &[f32],
    side: &[f32],
    key: &[f32],
    price: &[f32],
    size: &[f32],
    st: &mut [f32],
    out: &mut [f32],
    nd: u32,
    nr: u32,
    k: u32,
    thr: f32,
) {
    lv_scan(delta, side, key, price, size, st, out, nd, nr, k, thr);
}

/// `[side, price, count]` per delta; `params.period` = replenishment threshold (>= 1).
pub fn launch_cube_levels(formula: CubeFormula, fr: &GpuLevelFrame, params: CubeParams) -> Vec<Vec<f32>> {
    let _ = formula;
    let n = fr.n;
    if n == 0 {
        return Vec::new();
    }
    let nr = fr.delta.len();
    let client =
        cubecl::wgpu::WgpuRuntime::<cubecl::wgpu::AutoCompiler>::client(&Default::default());
    let up = |s: &Vec<f32>| client.create_from_slice(f32::as_bytes(if s.is_empty() { &[0.0f32][..] } else { &s[..] }));
    let st = client.empty(2 * fr.k * 4);
    let out = client.empty(3 * n * 4);
    unsafe {
        lv_map::launch_unchecked(
            &client,
            CubeCount::new_1d(1),
            CubeDim::new_1d(1),
            BufferArg::from_raw_parts(up(&fr.delta), nr.max(1)),
            BufferArg::from_raw_parts(up(&fr.side), nr.max(1)),
            BufferArg::from_raw_parts(up(&fr.key), nr.max(1)),
            BufferArg::from_raw_parts(up(&fr.price), nr.max(1)),
            BufferArg::from_raw_parts(up(&fr.size), nr.max(1)),
            BufferArg::from_raw_parts(st, 2 * fr.k),
            BufferArg::from_raw_parts(out.clone(), 3 * n),
            n as u32,
            nr as u32,
            fr.k as u32,
            params.period.max(1) as f32,
        );
    }
    let flat = f32::from_bytes(&client.read_one_unchecked(out)).to_vec();
    vec![flat[0..n].to_vec(), flat[n..2 * n].to_vec(), flat[2 * n..3 * n].to_vec()]
}
