//! Tick-with-book frame for the hybrid formulas (codes 997..=999): one row per trade, carrying the
//! trade (`price`, `size`, `is_buy`) and the order-book snapshot immediately before it (the
//! `HybridTickBookConsumer` contract: the caller pairs each tick with its book). UNTESTED on GPU.

use super::book_frame::GpuBookFrame;
use crate::core::types::{OrderBook, Tick};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpuHybridFrame {
    pub n: usize,
    pub price: Vec<f32>,
    pub size: Vec<f32>,
    /// `1.0` buyer-initiated, `0.0` seller-initiated.
    pub buy: Vec<f32>,
    /// Book before each trade, one snapshot per row.
    pub book: GpuBookFrame,
}

impl GpuHybridFrame {
    /// `depth` must cover the deepest level a formula reads (sweep analysis walks every level up to it).
    pub fn from_pairs(pairs: &[(Tick, OrderBook)], depth: usize) -> Self {
        let books: Vec<OrderBook> = pairs.iter().map(|(_, b)| b.clone()).collect();
        GpuHybridFrame {
            n: pairs.len(),
            price: pairs.iter().map(|(t, _)| t.price as f32).collect(),
            size: pairs.iter().map(|(t, _)| t.size as f32).collect(),
            buy: pairs.iter().map(|(t, _)| if t.is_buy { 1.0 } else { 0.0 }).collect(),
            book: GpuBookFrame::from_books(&books, depth),
        }
    }
}
