//! Order-book frame for the book event formulas (codes 960..=979).
//!
//! One row per book snapshot. Level planes are row-major, `depth` levels per snapshot
//! (`bid_px[i * depth + l]`); `nb` / `na` are the real level counts of snapshot `i` (levels past
//! the count are `0.0`). `ts` is milliseconds from the first snapshot as `f32` (exact up to
//! 4.66 hours). `depth` must be at least the largest level count a formula reads: for
//! `WallDetector` that is `2 * levels_to_sample`. UNTESTED on GPU.

use crate::core::types::OrderBook;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpuBookFrame {
    pub n: usize,
    pub depth: usize,
    pub bid_px: Vec<f32>,
    pub bid_sz: Vec<f32>,
    pub ask_px: Vec<f32>,
    pub ask_sz: Vec<f32>,
    pub nb: Vec<f32>,
    pub na: Vec<f32>,
    pub ts: Vec<f32>,
}

impl GpuBookFrame {
    pub fn from_books(books: &[OrderBook], depth: usize) -> Self {
        let depth = depth.max(1);
        let n = books.len();
        let t0 = books.first().map(|b| b.timestamp).unwrap_or(0);
        let mut f = GpuBookFrame {
            n,
            depth,
            bid_px: vec![0.0; n * depth],
            bid_sz: vec![0.0; n * depth],
            ask_px: vec![0.0; n * depth],
            ask_sz: vec![0.0; n * depth],
            nb: Vec::with_capacity(n),
            na: Vec::with_capacity(n),
            ts: Vec::with_capacity(n),
        };
        for (i, b) in books.iter().enumerate() {
            let kb = b.bids.len().min(depth);
            let ka = b.asks.len().min(depth);
            for l in 0..kb {
                f.bid_px[i * depth + l] = b.bids[l].price as f32;
                f.bid_sz[i * depth + l] = b.bids[l].size as f32;
            }
            for l in 0..ka {
                f.ask_px[i * depth + l] = b.asks[l].price as f32;
                f.ask_sz[i * depth + l] = b.asks[l].size as f32;
            }
            f.nb.push(kb as f32);
            f.na.push(ka as f32);
            f.ts.push((b.timestamp - t0) as f32);
        }
        f
    }

    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        self.n == 0
    }
}
