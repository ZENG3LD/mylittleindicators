//! Pivot / segment extraction for the EWA scanner.
//!
//! Pivot detection reuses the EXISTING `events::swing_detection::
//! SwingDetection` in `SwingMode::Lookahead { n }` — the fractal "confirmed
//! n bars later" swing definition — rather than inventing a second zigzag
//! detector. This module turns that streaming per-bar detector into a
//! batch pivot list, then a segment list, over a fixed
//! `&[(f64, f64, f64, f64)]` (open, high, low, close) slice — the shape
//! `ewa::scan` enumerates wave candidates over. Pure functions, no
//! allocation beyond the two output `Vec`s, O(bars) — one
//! `SwingDetection::detect_from_values` call per bar, same per-bar cost
//! every other scan primitive in this workspace already pays.

use crate::core::signal::direction::Direction;
use crate::indicators::swing::swing_detection::{SwingDetection, SwingMode};

use super::types::EwaSegmentDirection;

/// Whether a confirmed swing pivot is a local high or a local low.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PivotKind {
    High,
    Low,
}

/// One confirmed swing pivot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EwaSwingPivot {
    /// Bar index the pivot occurred ON (resolved `swing_n` bars back from
    /// the bar that confirmed it).
    pub index: usize,
    pub price: f64,
    pub kind: PivotKind,
    /// Bar index the detector actually fired on — `index + swing_n` for
    /// `SwingMode::Lookahead`.
    pub confirmed_index: usize,
}

/// One leg between two consecutive swing pivots.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EwaSwingSegment {
    /// Index into the `pivots` slice (NOT a bar index) of the leg's start.
    pub start: usize,
    /// Index into the `pivots` slice of the leg's end.
    pub end: usize,
    pub direction: EwaSegmentDirection,
    pub bars: usize,
    pub price_delta: f64,
}

/// Extract confirmed swing pivots from an OHLC bar slice using
/// `SwingMode::Lookahead { n: swing_n }`.
///
/// `bars` are `(open, high, low, close)`, oldest first; `open` is read only
/// for tuple-shape symmetry with `crate::core::types::Bar` — the detector
/// itself needs only `high`/`low`/`close`. `swing_n` is clamped to at least
/// 1. The very first and very last `swing_n` bars of any price swing can
/// never be confirmed as a pivot (the detector needs `swing_n` bars of
/// FUTURE history to confirm one) — this is the detector's own documented
/// lag, not a bug here.
pub fn extract_pivots(bars: &[(f64, f64, f64, f64)], swing_n: usize) -> Vec<EwaSwingPivot> {
    let n = swing_n.max(1);
    let mut detector = SwingDetection::new(SwingMode::Lookahead { n });
    let mut pivots = Vec::new();

    for (confirmed_index, &(_open, high, low, close)) in bars.iter().enumerate() {
        let Some((_, direction)) = detector.detect_from_values(high, low, close) else {
            continue;
        };
        let kind = match direction {
            Direction::Up => PivotKind::High,
            Direction::Down => PivotKind::Low,
            Direction::Neutral => continue,
        };
        let index = confirmed_index.saturating_sub(n);
        let price = match kind {
            PivotKind::High => bars.get(index).map(|b| b.1).unwrap_or(high),
            PivotKind::Low => bars.get(index).map(|b| b.2).unwrap_or(low),
        };
        pivots.push(EwaSwingPivot { index, price, kind, confirmed_index });
    }

    pivots
}

/// Build the leg list between consecutive pivots.
///
/// A pair of consecutive pivots sharing the same `kind` (two highs or two
/// lows in a row — the detector's own lag can occasionally produce this on
/// a rounded top/bottom) is skipped rather than forced into a fake leg,
/// same posture `build_segments` in the proprietary engine takes for the
/// identical case. Segment prices are re-derived from `bars` at each
/// pivot's `index` (never from the pivot's own cached `price` field) — the
/// segment is always the true reading of the bars it was built from, not a
/// copy that could drift from it. An out-of-bounds pivot index (a caller
/// error — pivots not actually extracted from this `bars` slice) falls
/// back to the cached price rather than panicking.
pub fn build_segments(bars: &[(f64, f64, f64, f64)], pivots: &[EwaSwingPivot]) -> Vec<EwaSwingSegment> {
    let mut segments = Vec::with_capacity(pivots.len().saturating_sub(1));

    for i in 1..pivots.len() {
        let start = &pivots[i - 1];
        let end = &pivots[i];
        if start.kind == end.kind || end.index <= start.index {
            continue;
        }

        let start_price = pivot_price(bars, start);
        let end_price = pivot_price(bars, end);
        let price_delta = end_price - start_price;

        segments.push(EwaSwingSegment {
            start: i - 1,
            end: i,
            direction: EwaSegmentDirection::from_delta(price_delta),
            bars: end.index - start.index,
            price_delta,
        });
    }

    segments
}

fn pivot_price(bars: &[(f64, f64, f64, f64)], pivot: &EwaSwingPivot) -> f64 {
    bars.get(pivot.index)
        .map(|&(_, high, low, _)| match pivot.kind {
            PivotKind::High => high,
            PivotKind::Low => low,
        })
        .unwrap_or(pivot.price)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One monotonic ramp of `steps` bars moving from `from` toward (but
    /// never reaching) `to`, `spread` apart — never itself a local extreme,
    /// so it can't be mistaken for a pivot.
    fn ramp(from: f64, to: f64, steps: usize, spread: f64) -> Vec<(f64, f64, f64, f64)> {
        (1..=steps)
            .map(|k| {
                let t = k as f64 / (steps as f64 + 1.0);
                let level = from + (to - from) * t;
                let (high, low) = if to >= from { (level, level - spread) } else { (level + spread, level) };
                (level, high, low, level)
            })
            .collect()
    }

    fn pivot_bar(kind: PivotKind, price: f64, spread: f64) -> (f64, f64, f64, f64) {
        match kind {
            PivotKind::High => (price - spread / 2.0, price, price - spread, price - spread / 2.0),
            PivotKind::Low => (price + spread / 2.0, price + spread, price, price + spread / 2.0),
        }
    }

    /// Builds a bar series walking through `prices` (alternating
    /// High/Low), padded on both ends so the first and last pivot are
    /// confirmable, with `steps` ramp bars between each pair.
    pub(super) fn zigzag_bars(
        kinds: &[PivotKind],
        prices: &[f64],
        steps: usize,
        spread: f64,
    ) -> Vec<(f64, f64, f64, f64)> {
        assert_eq!(kinds.len(), prices.len());
        let mut bars = Vec::new();
        // Approach the first pivot from the side that can't compete with
        // it: from below into a High, from above into a Low.
        let lead_from = match kinds[0] {
            PivotKind::High => prices[0] - 8.0,
            PivotKind::Low => prices[0] + 8.0,
        };
        bars.extend(ramp(lead_from, prices[0], 2, spread));
        for i in 0..prices.len() {
            if i > 0 {
                bars.extend(ramp(prices[i - 1], prices[i], steps, spread));
            }
            bars.push(pivot_bar(kinds[i], prices[i], spread));
        }
        let last_price = *prices.last().unwrap();
        let trail_to = match kinds.last().unwrap() {
            PivotKind::High => last_price - 8.0,
            PivotKind::Low => last_price + 8.0,
        };
        bars.extend(ramp(last_price, trail_to, 2, spread));
        bars
    }

    #[test]
    fn extract_pivots_finds_a_simple_v_shape() {
        let kinds = [PivotKind::High, PivotKind::Low, PivotKind::High];
        let prices = [100.0, 90.0, 105.0];
        let bars = zigzag_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let found_kinds: Vec<PivotKind> = pivots.iter().map(|p| p.kind).collect();
        assert_eq!(found_kinds, kinds.to_vec(), "must find exactly the 3 pivots in order, no spurious ramp hits");
        for (pivot, &expected_price) in pivots.iter().zip(prices.iter()) {
            assert!((pivot.price - expected_price).abs() < 1e-6, "pivot price must match the fixture's extreme");
        }
    }

    #[test]
    fn build_segments_skips_same_kind_consecutive_pivots() {
        let pivots = vec![
            EwaSwingPivot { index: 0, price: 100.0, kind: PivotKind::High, confirmed_index: 2 },
            EwaSwingPivot { index: 1, price: 101.0, kind: PivotKind::High, confirmed_index: 3 },
            EwaSwingPivot { index: 5, price: 90.0, kind: PivotKind::Low, confirmed_index: 7 },
        ];
        let bars = vec![(100.0, 100.0, 99.0, 100.0); 10];
        let segments = build_segments(&bars, &pivots);
        assert_eq!(segments.len(), 1, "same-kind consecutive pivots must not form a leg");
        assert_eq!(segments[0].start, 1);
        assert_eq!(segments[0].end, 2);
    }

    #[test]
    fn build_segments_reports_bar_count_and_direction() {
        let kinds = [PivotKind::Low, PivotKind::High];
        let prices = [100.0, 110.0];
        let bars = zigzag_bars(&kinds, &prices, 3, 0.2);
        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].direction, EwaSegmentDirection::Up);
        assert!(segments[0].price_delta > 0.0);
        assert!(segments[0].bars > 0);
    }
}
