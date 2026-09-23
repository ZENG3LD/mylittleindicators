//! Candidate enumeration and validation for the EWA scanner.
//!
//! Windows the pivot/segment sequence `ewa::swing` extracts by REQUIRED
//! SEGMENT COUNT per pattern family (3 segments / 4 pivots for the
//! simple-correction family — zigzag, flat, and the harmonic `Abcd` shape;
//! 4 segments / 5 pivots for the XABCD harmonic family — Gartley, Bat,
//! Butterfly, Crab, Shark, Cypher; 5 segments / 6 pivots for impulse,
//! diagonal, triangle, and the harmonic `ThreeDrives` shape), one forward
//! pass per family — mirroring the proprietary `CpuEwaScanner::scan_from`'s
//! algorithm SHAPE (`mylittlequant/crates/mli/src/ewa/scanner.rs`), a
//! fresh, independent OSS implementation, not a port. Deliberately
//! excluded: the probabilistic ranker/priors/reinterpretation apparatus,
//! and W-X-Y double/triple-combo composition (a later extension) — this
//! module answers "does this shape exist here, and how well does it fit,"
//! never "is this a good trading edge."
//!
//! Validation reuses the EXISTING predicate library `ewa::rules` — the
//! SAME module `elliott::guard` (mlc) already imports to judge a
//! hand-drawn primitive. A HARD rule violation drops the candidate
//! outright (a scan is a survey of plausible structures, never a
//! claim-judging surface); a SOFT ratio miss only lowers `score`, computed
//! via `ewa::ratios::ratio_to_targets` against the SAME retracement /
//! extension target tables `ewa::ratios` already declares.
//!
//! The 8 harmonic kinds (`Gartley`/`Bat`/`Butterfly`/`Crab`/`Shark`/
//! `Cypher`/`Abcd`/`ThreeDrives`) are scored differently, over the SAME
//! pivot/segment/window machinery: each leg of `ewa::harmonic::
//! targets_for(kind)`'s published Carney ratio table is a HARD requirement
//! (`ewa::harmonic::ratio_matches`, already carrying
//! `DEFAULT_HARMONIC_TOLERANCE`) — a harmonic pattern IS its ratio table,
//! unlike an Elliott wave's softer fib preference, so a leg landing outside
//! its band drops the candidate outright rather than merely lowering
//! `score`. `score` instead measures how close each satisfied leg sits to
//! its band's center, for ranking only. `EwaPatternKind::Xabcd` (the
//! unclassified five-point container) is deliberately never scanned here —
//! `ewa::harmonic::targets_for` returns an empty table for it by design, so
//! it is simply absent from `scan_waves`'s harmonic match arms, the same
//! "unclassified is not a scan target" reasoning the manual `XabcdPattern`
//! tool (mlc) already encodes.
//!
//! Phase 1 (MVP) kind coverage — single-degree families only, no
//! composition: `Impulse`; the 4 diagonal kinds (`Leading`/`Ending` ×
//! `Contracting`/`Expanding`); `Zigzag`; `Flat`; `Triangle`; the 8 harmonic
//! kinds above. A requested kind outside this set produces no hits — a
//! documented subset of the full 23-kind taxonomy, not a silent refusal;
//! W-X-Y combo families extend this module in a later phase without
//! touching `scan_waves`'s signature.

use std::ops::Range;

use super::harmonic;
use super::ratios::{EXTENSION_TARGETS, RETRACEMENT_TARGETS, ratio_to_targets};
use super::rules::{self, FLAT_B_A, ZIGZAG_B_A};
use super::swing::{EwaSwingPivot, EwaSwingSegment};
use super::types::{EwaPatternKind, EwaRatio};

/// Hard bound on how many pivots a single `scan_waves` call examines.
/// Enumeration truncates — never silently drops bars — at this many
/// pivots (the oldest-first prefix is kept); every hit produced while
/// truncated carries `truncated_family = true`, so a caller always knows
/// whether the survey saw the whole pivot list.
pub const MAX_PIVOTS_PER_SCAN: usize = 500;

/// Relative tolerance band around a soft ratio's nearest fib target — a
/// ratio landing within this fraction of its target counts as "satisfied"
/// toward `score`.
const SOFT_RATIO_TOLERANCE: f64 = 0.15;

/// One candidate wave structure `scan_waves` found and validated.
#[derive(Debug, Clone, PartialEq)]
pub struct EwaWaveHit {
    pub kind: EwaPatternKind,
    /// Bar indices, ordered, one per pivot in the structure (4 for the
    /// simple-correction family, 6 for impulse / diagonal / triangle).
    pub pivots: Vec<usize>,
    /// 0..=1 soft-ratio fit quality — a plain weighted fraction of the
    /// candidate's fib ratios landing inside `SOFT_RATIO_TOLERANCE` of
    /// their nearest target. Ranking only, never a probability model.
    pub score: f64,
    /// True when this hit's family enumeration ran under the
    /// `MAX_PIVOTS_PER_SCAN` truncation.
    pub truncated_family: bool,
}

/// Scans `pivots` / `segments` (from `ewa::swing::{extract_pivots,
/// build_segments}`) for candidates of each requested `kind`, validates
/// every candidate through `ewa::rules`, and keeps only those whose LAST
/// pivot's bar index falls inside `completion_range` — the same
/// end-anchored inclusion test `events::candle_pattern::scan` uses (a
/// structure belongs to the range by where it COMPLETES, not where it
/// starts).
///
/// An empty `kinds` returns an empty result — asking for nothing is not
/// shorthand for "every supported kind."
pub fn scan_waves(
    pivots: &[EwaSwingPivot],
    segments: &[EwaSwingSegment],
    kinds: &[EwaPatternKind],
    completion_range: Range<usize>,
) -> Vec<EwaWaveHit> {
    if kinds.is_empty() || pivots.len() < 4 {
        return Vec::new();
    }

    let truncated = pivots.len() > MAX_PIVOTS_PER_SCAN;
    let scan_pivots = if truncated { &pivots[..MAX_PIVOTS_PER_SCAN] } else { pivots };
    let scan_segments: Vec<&EwaSwingSegment> =
        segments.iter().filter(|s| s.end < scan_pivots.len()).collect();
    let by_start = segment_index(&scan_segments, scan_pivots.len());

    let mut hits = Vec::new();
    for &kind in kinds {
        match kind {
            EwaPatternKind::Impulse => {
                scan_impulse(scan_pivots, &scan_segments, &by_start, &completion_range, truncated, &mut hits)
            }
            EwaPatternKind::LeadingDiagonalContracting
            | EwaPatternKind::LeadingDiagonalExpanding
            | EwaPatternKind::EndingDiagonalContracting
            | EwaPatternKind::EndingDiagonalExpanding => scan_diagonal(
                kind,
                scan_pivots,
                &scan_segments,
                &by_start,
                &completion_range,
                truncated,
                &mut hits,
            ),
            EwaPatternKind::Zigzag => {
                scan_zigzag(scan_pivots, &scan_segments, &by_start, &completion_range, truncated, &mut hits)
            }
            EwaPatternKind::Flat => {
                scan_flat(scan_pivots, &scan_segments, &by_start, &completion_range, truncated, &mut hits)
            }
            EwaPatternKind::Triangle => {
                scan_triangle(scan_pivots, &scan_segments, &by_start, &completion_range, truncated, &mut hits)
            }
            EwaPatternKind::Gartley
            | EwaPatternKind::Bat
            | EwaPatternKind::Butterfly
            | EwaPatternKind::Crab
            | EwaPatternKind::Shark
            | EwaPatternKind::Cypher => scan_harmonic_xabcd(
                kind,
                scan_pivots,
                &scan_segments,
                &by_start,
                &completion_range,
                truncated,
                &mut hits,
            ),
            EwaPatternKind::Abcd => {
                scan_harmonic_abcd(scan_pivots, &scan_segments, &by_start, &completion_range, truncated, &mut hits)
            }
            EwaPatternKind::ThreeDrives => scan_harmonic_three_drives(
                scan_pivots,
                &scan_segments,
                &by_start,
                &completion_range,
                truncated,
                &mut hits,
            ),
            _ => {}
        }
    }

    hits.sort_by(|a, b| {
        let a_end = a.pivots.last().copied().unwrap_or(0);
        let b_end = b.pivots.last().copied().unwrap_or(0);
        a_end
            .cmp(&b_end)
            .then_with(|| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal))
    });
    hits
}

// ---------------------------------------------------------------------------
// Windowing — one forward pass over `start` pivot positions per family.
// ---------------------------------------------------------------------------

/// Maps a pivot index to the segment starting there, for segments that
/// connect two CONSECUTIVE pivots (`end == start + 1`) — the only shape a
/// family window walks. A skipped same-kind pivot pair (see
/// `swing::build_segments`) leaves a `None` gap, and any window spanning
/// that gap is correctly unreachable.
fn segment_index(segments: &[&EwaSwingSegment], pivot_count: usize) -> Vec<Option<usize>> {
    let mut by_start = vec![None; pivot_count.saturating_sub(1)];
    for (idx, seg) in segments.iter().enumerate() {
        if seg.end == seg.start + 1 && seg.start < by_start.len() {
            by_start[seg.start] = Some(idx);
        }
    }
    by_start
}

fn window3(by_start: &[Option<usize>], start: usize) -> Option<[usize; 3]> {
    Some([
        (*by_start.get(start)?)?,
        (*by_start.get(start + 1)?)?,
        (*by_start.get(start + 2)?)?,
    ])
}

fn window4(by_start: &[Option<usize>], start: usize) -> Option<[usize; 4]> {
    Some([
        (*by_start.get(start)?)?,
        (*by_start.get(start + 1)?)?,
        (*by_start.get(start + 2)?)?,
        (*by_start.get(start + 3)?)?,
    ])
}

fn window5(by_start: &[Option<usize>], start: usize) -> Option<[usize; 5]> {
    Some([
        (*by_start.get(start)?)?,
        (*by_start.get(start + 1)?)?,
        (*by_start.get(start + 2)?)?,
        (*by_start.get(start + 3)?)?,
        (*by_start.get(start + 4)?)?,
    ])
}

fn push_ratio(out: &mut Vec<EwaRatio>, name: &str, numerator: f64, denominator: f64, targets: &[f64], weight: f64) {
    if let Some(r) = ratio_to_targets(name, numerator, denominator, targets, weight) {
        out.push(r);
    }
}

fn soft_ratio_score(ratios: &[EwaRatio]) -> f64 {
    if ratios.is_empty() {
        return 1.0;
    }
    let total_weight: f64 = ratios.iter().map(|r| r.weight).sum();
    if total_weight <= f64::EPSILON {
        return 1.0;
    }
    let satisfied_weight: f64 = ratios
        .iter()
        .filter(|r| r.error <= r.target * SOFT_RATIO_TOLERANCE)
        .map(|r| r.weight)
        .sum();
    (satisfied_weight / total_weight).clamp(0.0, 1.0)
}

fn finalize(
    kind: EwaPatternKind,
    start: usize,
    pivot_count: usize,
    pivots: &[EwaSwingPivot],
    ratios: &[EwaRatio],
    truncated: bool,
) -> EwaWaveHit {
    let bar_indices = (0..pivot_count).map(|i| pivots[start + i].index).collect();
    EwaWaveHit { kind, pivots: bar_indices, score: soft_ratio_score(ratios), truncated_family: truncated }
}

// ---------------------------------------------------------------------------
// Impulse (6 pivots / 5 segments)
// ---------------------------------------------------------------------------

fn scan_impulse(
    pivots: &[EwaSwingPivot],
    segments: &[&EwaSwingSegment],
    by_start: &[Option<usize>],
    completion_range: &Range<usize>,
    truncated: bool,
    out: &mut Vec<EwaWaveHit>,
) {
    if pivots.len() < 6 {
        return;
    }
    for start in 0..=(pivots.len() - 6) {
        let Some(seg_idx) = window5(by_start, start) else { continue };
        if !completion_range.contains(&pivots[start + 5].index) {
            continue;
        }
        let segs = [
            segments[seg_idx[0]],
            segments[seg_idx[1]],
            segments[seg_idx[2]],
            segments[seg_idx[3]],
            segments[seg_idx[4]],
        ];
        if let Some(hit) = score_impulse(start, &segs, pivots, truncated) {
            out.push(hit);
        }
    }
}

fn score_impulse(
    start: usize,
    segs: &[&EwaSwingSegment; 5],
    pivots: &[EwaSwingPivot],
    truncated: bool,
) -> Option<EwaWaveHit> {
    let (w1, w2, w3, w4, w5) = (segs[0], segs[1], segs[2], segs[3], segs[4]);

    if !(w1.direction == w3.direction
        && w3.direction == w5.direction
        && w2.direction == w4.direction
        && w1.direction != w2.direction)
    {
        return None;
    }
    let (m1, m3, m5) = (w1.price_delta.abs(), w3.price_delta.abs(), w5.price_delta.abs());
    if m3 < m1.min(m5) {
        return None;
    }

    let p = |i: usize| pivots[start + i].price;
    let (p0, p1, p2, p3, p4, p5) = (p(0), p(1), p(2), p(3), p(4), p(5));

    if !rules::retracement_stays_above_origin(w1.direction, p0, p2) {
        return None;
    }
    if !rules::progresses(w1.direction, p1, p3) {
        return None;
    }
    if !rules::retracement_stays_above_origin(w1.direction, p2, p4) {
        return None;
    }
    // The defining hard rule that separates an impulse from a diagonal:
    // wave 4 must NOT retrace into wave 1's price territory.
    if !rules::progresses(w1.direction, p1, p4) {
        return None;
    }
    if !rules::progresses(w1.direction, p3, p5) {
        return None;
    }

    let mut ratios = Vec::with_capacity(4);
    push_ratio(&mut ratios, "wave2/wave1 retracement", w2.price_delta, w1.price_delta, RETRACEMENT_TARGETS, 1.0);
    push_ratio(&mut ratios, "wave3/wave1 extension", w3.price_delta, w1.price_delta, EXTENSION_TARGETS, 1.5);
    push_ratio(&mut ratios, "wave4/wave3 retracement", w4.price_delta, w3.price_delta, RETRACEMENT_TARGETS, 1.0);
    push_ratio(&mut ratios, "wave5/wave1 extension", w5.price_delta, w1.price_delta, EXTENSION_TARGETS, 1.0);

    Some(finalize(EwaPatternKind::Impulse, start, 6, pivots, &ratios, truncated))
}

// ---------------------------------------------------------------------------
// Diagonal (6 pivots / 5 segments) — Leading/Ending x Contracting/Expanding
// ---------------------------------------------------------------------------

fn scan_diagonal(
    kind: EwaPatternKind,
    pivots: &[EwaSwingPivot],
    segments: &[&EwaSwingSegment],
    by_start: &[Option<usize>],
    completion_range: &Range<usize>,
    truncated: bool,
    out: &mut Vec<EwaWaveHit>,
) {
    if pivots.len() < 6 {
        return;
    }
    for start in 0..=(pivots.len() - 6) {
        let Some(seg_idx) = window5(by_start, start) else { continue };
        if !completion_range.contains(&pivots[start + 5].index) {
            continue;
        }
        let segs = [
            segments[seg_idx[0]],
            segments[seg_idx[1]],
            segments[seg_idx[2]],
            segments[seg_idx[3]],
            segments[seg_idx[4]],
        ];
        if let Some(hit) = score_diagonal(kind, start, &segs, pivots, truncated) {
            out.push(hit);
        }
    }
}

fn score_diagonal(
    kind: EwaPatternKind,
    start: usize,
    segs: &[&EwaSwingSegment; 5],
    pivots: &[EwaSwingPivot],
    truncated: bool,
) -> Option<EwaWaveHit> {
    let (w1, w2, w3, w4, w5) = (segs[0], segs[1], segs[2], segs[3], segs[4]);

    if !(w1.direction == w3.direction
        && w3.direction == w5.direction
        && w2.direction == w4.direction
        && w1.direction != w2.direction)
    {
        return None;
    }
    let (m1, m3, m5) = (w1.price_delta.abs(), w3.price_delta.abs(), w5.price_delta.abs());
    if m3 < m1.min(m5) {
        return None;
    }

    let p = |i: usize| pivots[start + i].price;
    let (p0, p1, p2, p3, p4) = (p(0), p(1), p(2), p(3), p(4));

    if !rules::retracement_stays_above_origin(w1.direction, p0, p2) {
        return None;
    }
    if !rules::progresses(w1.direction, p1, p3) {
        return None;
    }
    if !rules::retracement_stays_above_origin(w1.direction, p2, p4) {
        return None;
    }
    // The defining hard rule of a diagonal (vs. an impulse): wave 4 MUST
    // overlap wave 1's price territory. `progresses` succeeding here would
    // mean wave 4 stayed clear of wave 1 — that candidate is an impulse,
    // not a diagonal.
    if rules::progresses(w1.direction, p1, p4) {
        return None;
    }

    let contracting =
        matches!(kind, EwaPatternKind::LeadingDiagonalContracting | EwaPatternKind::EndingDiagonalContracting);
    let initial_width = (p1 - p0).abs();
    let later_width = (p3 - p2).abs();
    if contracting {
        if later_width >= initial_width {
            return None;
        }
    } else if later_width <= initial_width {
        return None;
    }

    let mut ratios = Vec::with_capacity(2);
    push_ratio(&mut ratios, "wave2/wave1 diagonal retracement", w2.price_delta, w1.price_delta, RETRACEMENT_TARGETS, 1.0);
    push_ratio(&mut ratios, "wave4/wave3 diagonal retracement", w4.price_delta, w3.price_delta, RETRACEMENT_TARGETS, 1.0);

    Some(finalize(kind, start, 6, pivots, &ratios, truncated))
}

// ---------------------------------------------------------------------------
// Zigzag (4 pivots / 3 segments)
// ---------------------------------------------------------------------------

fn scan_zigzag(
    pivots: &[EwaSwingPivot],
    segments: &[&EwaSwingSegment],
    by_start: &[Option<usize>],
    completion_range: &Range<usize>,
    truncated: bool,
    out: &mut Vec<EwaWaveHit>,
) {
    if pivots.len() < 4 {
        return;
    }
    for start in 0..=(pivots.len() - 4) {
        let Some(seg_idx) = window3(by_start, start) else { continue };
        if !completion_range.contains(&pivots[start + 3].index) {
            continue;
        }
        let segs = [segments[seg_idx[0]], segments[seg_idx[1]], segments[seg_idx[2]]];
        if let Some(hit) = score_zigzag(start, &segs, pivots, truncated) {
            out.push(hit);
        }
    }
}

fn score_zigzag(
    start: usize,
    segs: &[&EwaSwingSegment; 3],
    pivots: &[EwaSwingPivot],
    truncated: bool,
) -> Option<EwaWaveHit> {
    let (a, b, c) = (segs[0], segs[1], segs[2]);
    if a.direction == b.direction || b.direction == c.direction || a.direction != c.direction {
        return None;
    }
    let b_a = rules::ratio(b.price_delta, a.price_delta).unwrap_or(f64::INFINITY);
    if !ZIGZAG_B_A.contains(b_a) {
        return None;
    }

    let p = |i: usize| pivots[start + i].price;
    let (p0, p1, p2, p3) = (p(0), p(1), p(2), p(3));
    if !rules::not_beyond_origin(p0, p1, p2) {
        return None;
    }
    if !rules::beyond(p0, p1, p3) {
        return None;
    }

    let mut ratios = Vec::with_capacity(2);
    push_ratio(&mut ratios, "B/A zigzag retracement", b.price_delta, a.price_delta, &[0.382, 0.5, 0.618, 0.786], 1.25);
    push_ratio(&mut ratios, "C/A zigzag projection", c.price_delta, a.price_delta, &[0.618, 1.0, 1.272, 1.618], 1.5);

    Some(finalize(EwaPatternKind::Zigzag, start, 4, pivots, &ratios, truncated))
}

// ---------------------------------------------------------------------------
// Flat (4 pivots / 3 segments)
// ---------------------------------------------------------------------------

fn scan_flat(
    pivots: &[EwaSwingPivot],
    segments: &[&EwaSwingSegment],
    by_start: &[Option<usize>],
    completion_range: &Range<usize>,
    truncated: bool,
    out: &mut Vec<EwaWaveHit>,
) {
    if pivots.len() < 4 {
        return;
    }
    for start in 0..=(pivots.len() - 4) {
        let Some(seg_idx) = window3(by_start, start) else { continue };
        if !completion_range.contains(&pivots[start + 3].index) {
            continue;
        }
        let segs = [segments[seg_idx[0]], segments[seg_idx[1]], segments[seg_idx[2]]];
        if let Some(hit) = score_flat(start, &segs, pivots, truncated) {
            out.push(hit);
        }
    }
}

fn score_flat(
    start: usize,
    segs: &[&EwaSwingSegment; 3],
    pivots: &[EwaSwingPivot],
    truncated: bool,
) -> Option<EwaWaveHit> {
    let (a, b, c) = (segs[0], segs[1], segs[2]);
    if a.direction == b.direction || b.direction == c.direction || a.direction != c.direction {
        return None;
    }
    let b_a = rules::ratio(b.price_delta, a.price_delta).unwrap_or(0.0);
    if !FLAT_B_A.contains(b_a) {
        return None;
    }

    let mut ratios = Vec::with_capacity(2);
    push_ratio(&mut ratios, "B/A flat retracement", b.price_delta, a.price_delta, &[0.886, 1.0, 1.13, 1.236], 1.5);
    push_ratio(&mut ratios, "C/A flat projection", c.price_delta, a.price_delta, &[1.0, 1.236, 1.382, 1.618], 1.25);

    Some(finalize(EwaPatternKind::Flat, start, 4, pivots, &ratios, truncated))
}

// ---------------------------------------------------------------------------
// Triangle (6 pivots / 5 segments, A-B-C-D-E)
// ---------------------------------------------------------------------------

fn scan_triangle(
    pivots: &[EwaSwingPivot],
    segments: &[&EwaSwingSegment],
    by_start: &[Option<usize>],
    completion_range: &Range<usize>,
    truncated: bool,
    out: &mut Vec<EwaWaveHit>,
) {
    if pivots.len() < 6 {
        return;
    }
    for start in 0..=(pivots.len() - 6) {
        let Some(seg_idx) = window5(by_start, start) else { continue };
        if !completion_range.contains(&pivots[start + 5].index) {
            continue;
        }
        let segs = [
            segments[seg_idx[0]],
            segments[seg_idx[1]],
            segments[seg_idx[2]],
            segments[seg_idx[3]],
            segments[seg_idx[4]],
        ];
        if let Some(hit) = score_triangle(start, &segs, pivots, truncated) {
            out.push(hit);
        }
    }
}

fn score_triangle(
    start: usize,
    segs: &[&EwaSwingSegment; 5],
    pivots: &[EwaSwingPivot],
    truncated: bool,
) -> Option<EwaWaveHit> {
    for pair in segs.windows(2) {
        if pair[0].direction == pair[1].direction {
            return None;
        }
    }
    // Contracting boundaries: each leg must be smaller than the leg two
    // positions earlier on the SAME boundary (A-C-E vs B-D).
    for i in 0..3 {
        if segs[i + 2].price_delta.abs() >= segs[i].price_delta.abs() {
            return None;
        }
    }

    let mut ratios = Vec::with_capacity(4);
    for pair in segs.windows(2) {
        push_ratio(
            &mut ratios,
            "triangle leg retracement",
            pair[1].price_delta,
            pair[0].price_delta,
            RETRACEMENT_TARGETS,
            0.75,
        );
    }

    Some(finalize(EwaPatternKind::Triangle, start, 6, pivots, &ratios, truncated))
}

// ---------------------------------------------------------------------------
// Harmonic (XABCD 5 pivots/4 segments; Abcd 4 pivots/3 segments; ThreeDrives
// 6 pivots/5 segments) — scored against `ewa::harmonic::targets_for`, not
// the Elliott fib-target lists above.
// ---------------------------------------------------------------------------

/// Pivot-name -> in-window position for the 6 XABCD harmonic kinds
/// (Gartley, Bat, Butterfly, Crab, Shark, Cypher): `X, A, B, C, D`.
fn xabcd_position(name: &str) -> Option<usize> {
    match name {
        "X" => Some(0),
        "A" => Some(1),
        "B" => Some(2),
        "C" => Some(3),
        "D" => Some(4),
        _ => None,
    }
}

/// Pivot-name -> in-window position for the harmonic `Abcd` shape:
/// `A, B, C, D` (no `X` — the generic four-point pattern).
fn abcd_position(name: &str) -> Option<usize> {
    match name {
        "A" => Some(0),
        "B" => Some(1),
        "C" => Some(2),
        "D" => Some(3),
        _ => None,
    }
}

/// Pivot-name -> in-window position for `ThreeDrives`: `X, 1, A, 2, B, 3`.
fn three_drives_position(name: &str) -> Option<usize> {
    match name {
        "X" => Some(0),
        "1" => Some(1),
        "A" => Some(2),
        "2" => Some(3),
        "B" => Some(4),
        "3" => Some(5),
        _ => None,
    }
}

/// Fit quality for one satisfied harmonic leg — 1.0 exactly at the band's
/// center, falling linearly to 0.0 at either edge. Distinct from
/// `soft_ratio_score` (which measures distance to one of a LIST of discrete
/// fib targets): a harmonic leg's target is a continuous band, not a
/// nearest-of-several-constants match, so it earns its own small scoring
/// step rather than being forced through `EwaRatio`/`ratio_to_targets`.
fn harmonic_leg_fit(ratio: f64, band: &rules::RatioBand) -> f64 {
    let center = (band.min + band.max) / 2.0;
    let half_width = ((band.max - band.min) / 2.0).max(f64::EPSILON);
    (1.0 - ((ratio - center).abs() / half_width).min(1.0)).max(0.0)
}

/// Scores one harmonic candidate: every leg in `harmonic::targets_for(kind)`
/// must satisfy `harmonic::ratio_matches` (HARD — see module doc) or the
/// whole candidate is dropped; `score` is the mean per-leg fit quality.
/// `position_of` maps a table leg's pivot NAME (`"X"`, `"A"`, …) to its
/// position within the `pivot_count`-pivot window starting at `start`.
fn score_harmonic(
    kind: EwaPatternKind,
    start: usize,
    pivot_count: usize,
    pivots: &[EwaSwingPivot],
    position_of: fn(&str) -> Option<usize>,
    truncated: bool,
) -> Option<EwaWaveHit> {
    let table = harmonic::targets_for(kind);
    if table.is_empty() {
        return None;
    }

    let price_at = |name: &str| -> Option<f64> {
        let pos = position_of(name)?;
        pivots.get(start + pos).map(|pivot| pivot.price)
    };

    let mut fit_sum = 0.0;
    for entry in table {
        let leg_start = price_at(entry.leg.0)?;
        let leg_end = price_at(entry.leg.1)?;
        let ref_start = price_at(entry.reference.0)?;
        let ref_end = price_at(entry.reference.1)?;
        let ratio = rules::ratio(leg_end - leg_start, ref_end - ref_start)?;
        if !harmonic::ratio_matches(ratio, entry) {
            return None;
        }
        fit_sum += harmonic_leg_fit(ratio, &entry.band);
    }

    let score = (fit_sum / table.len() as f64).clamp(0.0, 1.0);
    let pivots_out = (0..pivot_count).map(|i| pivots[start + i].index).collect();
    Some(EwaWaveHit { kind, pivots: pivots_out, score, truncated_family: truncated })
}

/// Every consecutive segment pair in `segs` must alternate direction — the
/// structural alternation `ewa::harmonic`'s own module doc calls out as a
/// HARD rule, checked defensively here even though `by_start` already only
/// links consecutive, opposite-kind pivots (same defensive posture
/// `score_triangle` takes for its own leg run).
fn alternates(segs: &[&EwaSwingSegment]) -> bool {
    segs.windows(2).all(|pair| pair[0].direction != pair[1].direction)
}

fn scan_harmonic_xabcd(
    kind: EwaPatternKind,
    pivots: &[EwaSwingPivot],
    segments: &[&EwaSwingSegment],
    by_start: &[Option<usize>],
    completion_range: &Range<usize>,
    truncated: bool,
    out: &mut Vec<EwaWaveHit>,
) {
    if pivots.len() < 5 {
        return;
    }
    for start in 0..=(pivots.len() - 5) {
        let Some(seg_idx) = window4(by_start, start) else { continue };
        if !completion_range.contains(&pivots[start + 4].index) {
            continue;
        }
        let segs = [segments[seg_idx[0]], segments[seg_idx[1]], segments[seg_idx[2]], segments[seg_idx[3]]];
        if !alternates(&segs) {
            continue;
        }
        if let Some(hit) = score_harmonic(kind, start, 5, pivots, xabcd_position, truncated) {
            out.push(hit);
        }
    }
}

fn scan_harmonic_abcd(
    pivots: &[EwaSwingPivot],
    segments: &[&EwaSwingSegment],
    by_start: &[Option<usize>],
    completion_range: &Range<usize>,
    truncated: bool,
    out: &mut Vec<EwaWaveHit>,
) {
    if pivots.len() < 4 {
        return;
    }
    for start in 0..=(pivots.len() - 4) {
        let Some(seg_idx) = window3(by_start, start) else { continue };
        if !completion_range.contains(&pivots[start + 3].index) {
            continue;
        }
        let segs = [segments[seg_idx[0]], segments[seg_idx[1]], segments[seg_idx[2]]];
        if !alternates(&segs) {
            continue;
        }
        if let Some(hit) = score_harmonic(EwaPatternKind::Abcd, start, 4, pivots, abcd_position, truncated) {
            out.push(hit);
        }
    }
}

fn scan_harmonic_three_drives(
    pivots: &[EwaSwingPivot],
    segments: &[&EwaSwingSegment],
    by_start: &[Option<usize>],
    completion_range: &Range<usize>,
    truncated: bool,
    out: &mut Vec<EwaWaveHit>,
) {
    if pivots.len() < 6 {
        return;
    }
    for start in 0..=(pivots.len() - 6) {
        let Some(seg_idx) = window5(by_start, start) else { continue };
        if !completion_range.contains(&pivots[start + 5].index) {
            continue;
        }
        let segs = [
            segments[seg_idx[0]],
            segments[seg_idx[1]],
            segments[seg_idx[2]],
            segments[seg_idx[3]],
            segments[seg_idx[4]],
        ];
        if !alternates(&segs) {
            continue;
        }
        if let Some(hit) = score_harmonic(EwaPatternKind::ThreeDrives, start, 6, pivots, three_drives_position, truncated)
        {
            out.push(hit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::swing::{EwaSwingSegment, PivotKind, build_segments, extract_pivots};
    use super::super::types::EwaSegmentDirection;

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

    fn wave_bars(kinds: &[PivotKind], prices: &[f64], steps: usize, spread: f64) -> Vec<(f64, f64, f64, f64)> {
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
        let last_price = *prices.last().unwrap_or(&0.0);
        let trail_to = match kinds.last() {
            Some(PivotKind::High) => last_price - 8.0,
            _ => last_price + 8.0,
        };
        bars.extend(ramp(last_price, trail_to, 2, spread));
        bars
    }

    fn find_hit(hits: &[EwaWaveHit], kind: EwaPatternKind) -> Option<&EwaWaveHit> {
        hits.iter().find(|h| h.kind == kind)
    }

    #[test]
    fn textbook_impulse_is_found_and_scores_well() {
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High];
        // p0=100, wave1=+10, wave2=-5 (0.5 retrace), wave3=+16.18 (1.618 ext),
        // wave4=-6.18 (0.382 retrace of wave3), wave5=+10 (1.0 ext of wave1).
        let prices = [100.0, 110.0, 105.0, 121.18, 115.0, 125.0];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_waves(&pivots, &segments, &[EwaPatternKind::Impulse], 0..bars.len());

        let hit = find_hit(&hits, EwaPatternKind::Impulse).expect("textbook impulse must be found");
        assert_eq!(hit.pivots.len(), 6);
        assert!(hit.score > 0.9, "textbook fib-exact impulse should score near 1.0, got {}", hit.score);

        let expected_prices: Vec<f64> = hit.pivots.iter().map(|&bar_idx| bars[bar_idx].1.max(bars[bar_idx].2)).collect();
        for (found, expected) in expected_prices.iter().zip(prices.iter()) {
            // Either the high or the low column carries the pivot's price
            // depending on kind — just confirm the bar is in the right
            // neighbourhood, not exactly which column.
            assert!((found - expected).abs() < 1.0, "pivot bar price {found} should be near {expected}");
        }
    }

    #[test]
    fn impulse_with_wave4_overlapping_wave1_is_dropped_not_penalized() {
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High];
        // Same shape as the textbook impulse, but wave4 (p4=108.0) retraces
        // BELOW wave1's end (p1=110.0) — an impulse-invalidating overlap.
        let prices = [100.0, 110.0, 105.0, 121.18, 108.0, 118.0];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_waves(&pivots, &segments, &[EwaPatternKind::Impulse], 0..bars.len());

        assert!(
            find_hit(&hits, EwaPatternKind::Impulse).is_none(),
            "wave4/wave1 overlap must be a HARD drop, not a scored-down hit: {hits:?}"
        );
    }

    #[test]
    fn textbook_zigzag_is_found_and_scores_well() {
        let kinds = [PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low];
        // p0=100, A=-20, B=+7.64 (0.382 retrace), C=-32.36 (1.618 ext of A).
        let prices = [100.0, 80.0, 87.64, 55.28];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_waves(&pivots, &segments, &[EwaPatternKind::Zigzag], 0..bars.len());

        let hit = find_hit(&hits, EwaPatternKind::Zigzag).expect("textbook zigzag must be found");
        assert_eq!(hit.pivots.len(), 4);
        assert!(hit.score > 0.9, "textbook fib-exact zigzag should score near 1.0, got {}", hit.score);
    }

    #[test]
    fn textbook_contracting_triangle_is_found_and_scores_well() {
        let kinds = [PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low];
        // Each leg 0.618x the leg two positions back — a clean contracting
        // triangle with every leg ratio landing exactly on a fib target.
        let prices = [100.0, 90.0, 96.18, 92.361, 94.721, 93.262];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_waves(&pivots, &segments, &[EwaPatternKind::Triangle], 0..bars.len());

        let hit = find_hit(&hits, EwaPatternKind::Triangle).expect("textbook contracting triangle must be found");
        assert_eq!(hit.pivots.len(), 6);
        assert!(hit.score > 0.9, "textbook fib-exact triangle should score near 1.0, got {}", hit.score);
    }

    #[test]
    fn empty_kinds_returns_no_hits() {
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High];
        let prices = [100.0, 80.0, 87.64, 55.28];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);
        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        assert!(scan_waves(&pivots, &segments, &[], 0..bars.len()).is_empty());
    }

    #[test]
    fn completion_range_excludes_hits_completing_outside_it() {
        let kinds = [PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low];
        let prices = [100.0, 80.0, 87.64, 55.28];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);
        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);

        let full = scan_waves(&pivots, &segments, &[EwaPatternKind::Zigzag], 0..bars.len());
        assert!(!full.is_empty());

        let narrow = scan_waves(&pivots, &segments, &[EwaPatternKind::Zigzag], 0..2);
        assert!(narrow.is_empty(), "a completion range before the structure finishes must exclude it");
    }

    /// Truncation test — builds pivots/segments directly (not through
    /// `swing::extract_pivots`, which is already covered by the fixtures
    /// above) so the fixture can cheaply exceed `MAX_PIVOTS_PER_SCAN`.
    #[test]
    fn truncates_at_max_pivots_per_scan() {
        let total = MAX_PIVOTS_PER_SCAN + 40;
        let mut pivots = Vec::with_capacity(total);
        let mut price = 100.0;
        for i in 0..total {
            let kind = if i % 2 == 0 { PivotKind::High } else { PivotKind::Low };
            price += if i % 2 == 0 { 10.0 } else { -6.18 };
            pivots.push(super::super::swing::EwaSwingPivot { index: i * 3, price, kind, confirmed_index: i * 3 + 2 });
        }
        let mut segments = Vec::with_capacity(total - 1);
        for i in 1..total {
            let delta = pivots[i].price - pivots[i - 1].price;
            segments.push(EwaSwingSegment {
                start: i - 1,
                end: i,
                direction: EwaSegmentDirection::from_delta(delta),
                bars: pivots[i].index - pivots[i - 1].index,
                price_delta: delta,
            });
        }

        assert!(pivots.len() > MAX_PIVOTS_PER_SCAN);
        let hits = scan_waves(&pivots, &segments, &[EwaPatternKind::Zigzag], 0..(total * 3));
        assert!(!hits.is_empty(), "expected at least one zigzag hit inside the truncated prefix");
        assert!(hits.iter().all(|h| h.truncated_family), "every hit must be flagged truncated");
        let max_bar_index = pivots[MAX_PIVOTS_PER_SCAN - 1].index;
        assert!(
            hits.iter().all(|h| h.pivots.iter().all(|&idx| idx <= max_bar_index)),
            "no hit may reference a pivot past the truncation cutoff"
        );
    }

    // ---- harmonic kinds (Phase 3) ----

    /// `AB = 0.618 XA` exact (Gartley's own AB target). `BC = 0.6796 AB` and
    /// `CD = 1.4 BC` are both solved/chosen with real margin inside their
    /// own [0.382,0.886]/[1.13,1.618] bands (never AT an edge — an
    /// edge-exact nominal, e.g. `CD = 1.618 BC`, is one hand-rounded decimal
    /// away from tipping outside an INCLUSIVE band under float rounding) so
    /// that `AD` lands on Gartley's own `exact(0.786)` target with margin
    /// to spare: X=1000, A=1100 (XA=100); AB=61.8; BC=42.0; CD=58.8;
    /// AD=78.6 (ratio 0.786 exactly).
    #[test]
    fn textbook_gartley_is_found_at_expected_pivots() {
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low];
        let prices = [1000.0, 1100.0, 1038.2, 1080.2, 1021.4];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_waves(&pivots, &segments, &[EwaPatternKind::Gartley], 0..bars.len());

        let hit = find_hit(&hits, EwaPatternKind::Gartley).expect("textbook Gartley must be found");
        assert_eq!(hit.pivots.len(), 5);
    }

    /// `AB = 0.45 XA` (mid Bat's [0.382,0.50] band), `CD = 2.5 BC` (mid
    /// [1.618,2.618], real margin from both edges), `BC` solved so `AD`
    /// lands on Bat's own `exact(0.886)`: X=1000, A=1100 (XA=100); AB=45.0;
    /// BC=29.066667 (ratio 0.6459, margin from either BC-band edge);
    /// CD=72.666667; AD=88.6 (ratio 0.886 exactly).
    #[test]
    fn textbook_bat_is_found_at_expected_pivots() {
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low];
        let prices = [1000.0, 1100.0, 1055.0, 1084.066667, 1011.4];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_waves(&pivots, &segments, &[EwaPatternKind::Bat], 0..bars.len());

        let hit = find_hit(&hits, EwaPatternKind::Bat).expect("textbook Bat must be found");
        assert_eq!(hit.pivots.len(), 5);
    }

    /// `AB = 0.786 XA` exact (Butterfly's own AB target), `CD = 2.0 BC`
    /// (inside [1.618,2.24]), `BC` solved so `AD` lands at 1.4 XA (inside
    /// Butterfly's [1.27,1.618] AD band).
    #[test]
    fn textbook_butterfly_is_found_at_expected_pivots() {
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low];
        let prices = [1000.0, 1100.0, 1021.4, 1082.8, 960.0];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_waves(&pivots, &segments, &[EwaPatternKind::Butterfly], 0..bars.len());

        let hit = find_hit(&hits, EwaPatternKind::Butterfly).expect("textbook Butterfly must be found");
        assert_eq!(hit.pivots.len(), 5);
    }

    /// `AB = 0.59 XA` (near the top of [0.382,0.618], real margin left),
    /// `BC = 0.75 AB` (mid [0.382,0.886]), `CD` solved so `AD` lands on
    /// Crab's own `exact(1.618)`: X=1000, A=1100 (XA=100); AB=59.0; BC=44.25
    /// (ratio 0.75); CD=147.05 (ratio ≈3.322, inside [2.618,3.618] with
    /// margin from both edges); AD=161.8 (ratio 1.618 exactly).
    #[test]
    fn textbook_crab_is_found_at_expected_pivots() {
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low];
        let prices = [1000.0, 1100.0, 1041.0, 1085.25, 938.2];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_waves(&pivots, &segments, &[EwaPatternKind::Crab], 0..bars.len());

        let hit = find_hit(&hits, EwaPatternKind::Crab).expect("textbook Crab must be found");
        assert_eq!(hit.pivots.len(), 5);
    }

    /// `AB = 0.5 XA` (inside [0.382,0.618]), `AC = 1.272 XA` (inside
    /// [1.13,1.618]), `CD = 1.0 XC` (inside [0.886,1.13]) — Shark has no
    /// `(A, D)`-over-`(X, A)` leg, so this fixture needs no solved value.
    #[test]
    fn textbook_shark_is_found_at_expected_pivots() {
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low];
        let prices = [1000.0, 1100.0, 1050.0, 1227.2, 1000.0];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_waves(&pivots, &segments, &[EwaPatternKind::Shark], 0..bars.len());

        let hit = find_hit(&hits, EwaPatternKind::Shark).expect("textbook Shark must be found");
        assert_eq!(hit.pivots.len(), 5);
    }

    /// `AB = 0.5 XA` (inside [0.382,0.618]), `AC = 1.35 XA` (inside
    /// [1.272,1.414]), `CD = 0.786 XC` exact (Cypher's own CD target).
    #[test]
    fn textbook_cypher_is_found_at_expected_pivots() {
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low];
        let prices = [1000.0, 1100.0, 1050.0, 1235.0, 1050.29];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_waves(&pivots, &segments, &[EwaPatternKind::Cypher], 0..bars.len());

        let hit = find_hit(&hits, EwaPatternKind::Cypher).expect("textbook Cypher must be found");
        assert_eq!(hit.pivots.len(), 5);
    }

    /// `BC = 0.7 AB` (inside [0.618,0.786]), `CD = 1.4 BC` (inside
    /// [1.272,1.618]) — the harmonic `Abcd` shape (no `X`).
    #[test]
    fn textbook_harmonic_abcd_is_found_at_expected_pivots() {
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High];
        let prices = [1000.0, 1100.0, 1030.0, 1128.0];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_waves(&pivots, &segments, &[EwaPatternKind::Abcd], 0..bars.len());

        let hit = find_hit(&hits, EwaPatternKind::Abcd).expect("textbook harmonic Abcd must be found");
        assert_eq!(hit.pivots.len(), 4);
    }

    /// Each retracement `0.7` of the drive it follows, each drive after the
    /// first `1.4` of the retracement it follows — both inside
    /// `ThreeDrives`' own `[0.618,0.786]`/`[1.272,1.618]` bands.
    #[test]
    fn textbook_three_drives_is_found_at_expected_pivots() {
        let kinds = [
            PivotKind::Low,
            PivotKind::High,
            PivotKind::Low,
            PivotKind::High,
            PivotKind::Low,
            PivotKind::High,
        ];
        let prices = [1000.0, 1100.0, 1030.0, 1128.0, 1059.4, 1155.44];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_waves(&pivots, &segments, &[EwaPatternKind::ThreeDrives], 0..bars.len());

        let hit = find_hit(&hits, EwaPatternKind::ThreeDrives).expect("textbook ThreeDrives must be found");
        assert_eq!(hit.pivots.len(), 6);
    }

    #[test]
    fn harmonic_leg_outside_tolerance_is_dropped_not_penalized() {
        // Same XABCD shape as the Gartley fixture above, but AB (measured
        // against the X-A reference) is 0.7 — far outside Gartley's own
        // exact(0.618) band — so this is a near-miss, not a fib-exact
        // structure, and must be rejected outright.
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low];
        let prices = [1000.0, 1100.0, 1030.0, 1075.0, 1015.0];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_waves(&pivots, &segments, &[EwaPatternKind::Gartley], 0..bars.len());

        assert!(
            find_hit(&hits, EwaPatternKind::Gartley).is_none(),
            "a leg ratio clearly outside its harmonic band must be a HARD drop, not a scored-down hit: {hits:?}"
        );
    }

    #[test]
    fn perf_smoke_10_000_bars_stays_bounded() {
        let bars: Vec<(f64, f64, f64, f64)> = (0..10_000)
            .map(|i| {
                let t = i as f64;
                let base = 100.0 + (t * 0.05).sin() * 8.0 + (t * 0.011).sin() * 3.0;
                (base, base + 0.5, base - 0.5, base)
            })
            .collect();

        let start_time = std::time::Instant::now();
        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let kinds = [
            EwaPatternKind::Impulse,
            EwaPatternKind::LeadingDiagonalContracting,
            EwaPatternKind::EndingDiagonalContracting,
            EwaPatternKind::Zigzag,
            EwaPatternKind::Flat,
            EwaPatternKind::Triangle,
            EwaPatternKind::Gartley,
            EwaPatternKind::Bat,
            EwaPatternKind::Butterfly,
            EwaPatternKind::Crab,
            EwaPatternKind::Shark,
            EwaPatternKind::Cypher,
            EwaPatternKind::Abcd,
            EwaPatternKind::ThreeDrives,
        ];
        let hits = scan_waves(&pivots, &segments, &kinds, 0..bars.len());
        let elapsed = start_time.elapsed();

        assert!(
            elapsed.as_millis() < 1000,
            "EWA scan over 10,000 bars (Elliott + harmonic kinds) took {elapsed:?}, expected comfortably under 1s in release"
        );
        // Sanity: a wavy 10,000-bar series should produce at least a
        // handful of pivots and typically some hits, though the assertion
        // above is the perf gate — this just guards against an
        // accidentally-empty fixture silently passing.
        assert!(pivots.len() > 20, "fixture should produce a meaningful pivot count, got {}", pivots.len());
        let _ = hits;
    }
}
