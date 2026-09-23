//! Fibonacci swing-pair selection — a pure function over the EXISTING
//! `ewa::swing` pivot/segment output that picks which confirmed swing leg(s)
//! a Fib autodetector should draw retracement/extension levels on inside a
//! bar range.
//!
//! A "swing pair" is one already-confirmed `EwaSwingSegment` (the leg
//! between two consecutive `EwaSwingPivot`s `ewa::swing::build_segments`
//! already builds) — this module never re-derives swing geometry or
//! searches across non-adjacent pivots; it only selects among segments
//! `ewa::swing` already produced, exactly the "vocabulary and detection math
//! live in `mli`... over raw pivot arrays" split
//! `docs/mlc/plans/autodetectors-arc-2026-09-23.md` §1 establishes for every
//! scanner in this arc. Zero UI, zero render cache — the mlc `FibScan`
//! primitive (Phase A4) owns range-drag, settings, and rendering.

use std::ops::Range;

use super::swing::{EwaSwingPivot, EwaSwingSegment};
use super::types::EwaSegmentDirection;

/// Which segment(s) a Fib autodetector selects out of a bar range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FibSwingMode {
    /// The single segment with the largest absolute price move — "the
    /// biggest high\u{2194}low move in range."
    LargestSwing,
    /// The most recent `n` segments, in chronological order (oldest of the
    /// selected first) — "every swing pair in range," bounded to the last
    /// `n`. `n = 0` selects none.
    LastSwings(usize),
}

/// Direction filter applied before `FibSwingMode` selects among the
/// remaining segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FibSwingDirection {
    /// No filtering — both `Up` and `Down` segments are eligible.
    Auto,
    Up,
    Down,
}

/// One swing pair selected for Fib level drawing: the bar indices and
/// prices of its two endpoints, oldest first, plus the leg's direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FibSwingPair {
    pub start_index: usize,
    pub end_index: usize,
    pub start_price: f64,
    pub end_price: f64,
    pub direction: EwaSegmentDirection,
}

/// Selects the swing pair(s) a Fib autodetector should draw levels on.
///
/// `pivots`/`segments` are `ewa::swing::{extract_pivots, build_segments}`'s
/// own output. A segment is eligible only when BOTH its endpoint pivots'
/// bar indices fall inside `bar_range` — a swing partially outside the
/// dragged range is not "inside" it — and, unless `direction` is `Auto`,
/// only when its direction matches. Prices are read from the pivots' own
/// cached `price` field (this function takes no raw bar slice, by design —
/// see the module doc).
///
/// Returns an empty `Vec` when no segment is eligible (an empty range, a
/// direction filter no segment satisfies, or `LastSwings(0)`) — never a
/// fallback to "every segment," matching every other scan primitive in this
/// arc's own "asking for nothing is not shorthand for everything" posture
/// (`scan::scan_waves`'s own doc).
pub fn select_fib_swings(
    pivots: &[EwaSwingPivot],
    segments: &[EwaSwingSegment],
    bar_range: Range<usize>,
    mode: FibSwingMode,
    direction: FibSwingDirection,
) -> Vec<FibSwingPair> {
    let mut candidates: Vec<&EwaSwingSegment> = segments
        .iter()
        .filter(|seg| segment_inside_range(pivots, seg, &bar_range))
        .filter(|seg| direction_matches(direction, seg.direction))
        .collect();

    match mode {
        FibSwingMode::LargestSwing => candidates
            .into_iter()
            .max_by(|a, b| a.price_delta.abs().partial_cmp(&b.price_delta.abs()).unwrap_or(std::cmp::Ordering::Equal))
            .and_then(|seg| to_pair(pivots, seg))
            .into_iter()
            .collect(),
        FibSwingMode::LastSwings(n) => {
            candidates.sort_by_key(|seg| pivots.get(seg.end).map(|p| p.index).unwrap_or(usize::MAX));
            let take_from = candidates.len().saturating_sub(n);
            candidates[take_from..].iter().filter_map(|seg| to_pair(pivots, seg)).collect()
        }
    }
}

fn segment_inside_range(pivots: &[EwaSwingPivot], seg: &EwaSwingSegment, bar_range: &Range<usize>) -> bool {
    let (Some(start), Some(end)) = (pivots.get(seg.start), pivots.get(seg.end)) else {
        return false;
    };
    bar_range.contains(&start.index) && bar_range.contains(&end.index)
}

fn direction_matches(filter: FibSwingDirection, direction: EwaSegmentDirection) -> bool {
    match filter {
        FibSwingDirection::Auto => true,
        FibSwingDirection::Up => direction == EwaSegmentDirection::Up,
        FibSwingDirection::Down => direction == EwaSegmentDirection::Down,
    }
}

fn to_pair(pivots: &[EwaSwingPivot], seg: &EwaSwingSegment) -> Option<FibSwingPair> {
    let start = pivots.get(seg.start)?;
    let end = pivots.get(seg.end)?;
    Some(FibSwingPair {
        start_index: start.index,
        end_index: end.index,
        start_price: start.price,
        end_price: end.price,
        direction: seg.direction,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::swing::PivotKind;

    /// A small hand-built pivot/segment fixture, bypassing `extract_pivots`
    /// (already covered in `swing`'s own tests): four pivots at bar indices
    /// 0/5/10/15, giving three segments Up(+20)/Down(-30)/Up(+60).
    fn three_segment_fixture() -> (Vec<EwaSwingPivot>, Vec<EwaSwingSegment>) {
        let pivots = vec![
            EwaSwingPivot { index: 0, price: 100.0, kind: PivotKind::Low, confirmed_index: 2 },
            EwaSwingPivot { index: 5, price: 120.0, kind: PivotKind::High, confirmed_index: 7 },
            EwaSwingPivot { index: 10, price: 90.0, kind: PivotKind::Low, confirmed_index: 12 },
            EwaSwingPivot { index: 15, price: 150.0, kind: PivotKind::High, confirmed_index: 17 },
        ];
        let segments = vec![
            EwaSwingSegment { start: 0, end: 1, direction: EwaSegmentDirection::Up, bars: 5, price_delta: 20.0 },
            EwaSwingSegment { start: 1, end: 2, direction: EwaSegmentDirection::Down, bars: 5, price_delta: -30.0 },
            EwaSwingSegment { start: 2, end: 3, direction: EwaSegmentDirection::Up, bars: 5, price_delta: 60.0 },
        ];
        (pivots, segments)
    }

    /// Five pivots at bar indices 0/5/10/15/20, giving four segments:
    /// Up(+20)/Down(-30)/Up(+60)/Down(-10).
    fn four_segment_fixture() -> (Vec<EwaSwingPivot>, Vec<EwaSwingSegment>) {
        let (mut pivots, mut segments) = three_segment_fixture();
        pivots.push(EwaSwingPivot { index: 20, price: 140.0, kind: PivotKind::Low, confirmed_index: 22 });
        segments.push(EwaSwingSegment { start: 3, end: 4, direction: EwaSegmentDirection::Down, bars: 5, price_delta: -10.0 });
        (pivots, segments)
    }

    #[test]
    fn largest_swing_finds_a_single_up_swing() {
        let pivots = vec![
            EwaSwingPivot { index: 0, price: 100.0, kind: PivotKind::Low, confirmed_index: 2 },
            EwaSwingPivot { index: 5, price: 120.0, kind: PivotKind::High, confirmed_index: 7 },
        ];
        let segments =
            vec![EwaSwingSegment { start: 0, end: 1, direction: EwaSegmentDirection::Up, bars: 5, price_delta: 20.0 }];

        let hits = select_fib_swings(&pivots, &segments, 0..100, FibSwingMode::LargestSwing, FibSwingDirection::Auto);

        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0],
            FibSwingPair {
                start_index: 0,
                end_index: 5,
                start_price: 100.0,
                end_price: 120.0,
                direction: EwaSegmentDirection::Up,
            }
        );
    }

    #[test]
    fn largest_swing_finds_a_single_down_swing() {
        let pivots = vec![
            EwaSwingPivot { index: 0, price: 120.0, kind: PivotKind::High, confirmed_index: 2 },
            EwaSwingPivot { index: 5, price: 90.0, kind: PivotKind::Low, confirmed_index: 7 },
        ];
        let segments =
            vec![EwaSwingSegment { start: 0, end: 1, direction: EwaSegmentDirection::Down, bars: 5, price_delta: -30.0 }];

        let hits = select_fib_swings(&pivots, &segments, 0..100, FibSwingMode::LargestSwing, FibSwingDirection::Auto);

        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0],
            FibSwingPair {
                start_index: 0,
                end_index: 5,
                start_price: 120.0,
                end_price: 90.0,
                direction: EwaSegmentDirection::Down,
            }
        );
    }

    #[test]
    fn largest_swing_picks_the_biggest_of_several() {
        let (pivots, segments) = three_segment_fixture();

        let hits = select_fib_swings(&pivots, &segments, 0..100, FibSwingMode::LargestSwing, FibSwingDirection::Auto);

        assert_eq!(hits.len(), 1, "LargestSwing must return exactly one pair");
        assert_eq!(hits[0].start_index, 10);
        assert_eq!(hits[0].end_index, 15);
        assert_eq!(hits[0].direction, EwaSegmentDirection::Up);
        assert!((hits[0].end_price - hits[0].start_price - 60.0).abs() < 1e-9);
    }

    #[test]
    fn largest_swing_respects_direction_filter() {
        let (pivots, segments) = three_segment_fixture();

        let hits = select_fib_swings(&pivots, &segments, 0..100, FibSwingMode::LargestSwing, FibSwingDirection::Down);

        assert_eq!(hits.len(), 1);
        // Only one Down segment in the fixture (index 1->2, delta -30) --
        // it must be the one returned even though the Up segment (2->3,
        // delta 60) is larger in magnitude.
        assert_eq!(hits[0].start_index, 5);
        assert_eq!(hits[0].end_index, 10);
        assert_eq!(hits[0].direction, EwaSegmentDirection::Down);
    }

    #[test]
    fn last_swings_returns_the_most_recent_n_in_chronological_order() {
        let (pivots, segments) = four_segment_fixture();

        let hits = select_fib_swings(&pivots, &segments, 0..100, FibSwingMode::LastSwings(2), FibSwingDirection::Auto);

        assert_eq!(hits.len(), 2);
        assert_eq!((hits[0].start_index, hits[0].end_index), (10, 15));
        assert_eq!((hits[1].start_index, hits[1].end_index), (15, 20));
    }

    #[test]
    fn last_swings_zero_returns_none() {
        let (pivots, segments) = four_segment_fixture();

        let hits = select_fib_swings(&pivots, &segments, 0..100, FibSwingMode::LastSwings(0), FibSwingDirection::Auto);

        assert!(hits.is_empty());
    }

    #[test]
    fn range_excludes_a_larger_swing_outside_the_window() {
        let (pivots, segments) = three_segment_fixture();

        // 0..11 excludes the segment ending at bar index 15 (the largest,
        // +60) even though it would otherwise win LargestSwing -- only the
        // Down segment (5->10, -30) is fully inside this narrower range.
        let hits = select_fib_swings(&pivots, &segments, 0..11, FibSwingMode::LargestSwing, FibSwingDirection::Auto);

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].start_index, 5);
        assert_eq!(hits[0].end_index, 10);
    }

    #[test]
    fn empty_range_returns_no_swings() {
        let (pivots, segments) = three_segment_fixture();

        let largest =
            select_fib_swings(&pivots, &segments, 1000..2000, FibSwingMode::LargestSwing, FibSwingDirection::Auto);
        let last =
            select_fib_swings(&pivots, &segments, 1000..2000, FibSwingMode::LastSwings(5), FibSwingDirection::Auto);

        assert!(largest.is_empty());
        assert!(last.is_empty());
    }
}
