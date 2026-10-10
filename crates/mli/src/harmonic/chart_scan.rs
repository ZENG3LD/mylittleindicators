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
//! The 11 harmonic kinds (`Gartley`/`Bat`/`AltBat`/`Butterfly`/`Crab`/
//! `DeepCrab`/`Shark`/`Cypher`/`Abcd`/`ThreeDrives`/`FiveZero`) are scored
//! differently, over the SAME pivot/segment/window machinery: each leg of
//! `ewa::harmonic::targets_for(kind)`'s published ratio table is a HARD
//! requirement
//! (`ewa::harmonic::ratio_matches`, already carrying
//! `DEFAULT_HARMONIC_TOLERANCE`) — a harmonic pattern IS its ratio table,
//! unlike an Elliott wave's softer fib preference, so a leg landing outside
//! its band drops the candidate outright rather than merely lowering
//! `score`. `score` instead measures how close each satisfied leg sits to
//! its band's center, for ranking only. `HarmonicKind::Xabcd` (the
//! unclassified five-point container) is deliberately never scanned here —
//! `ewa::harmonic::targets_for` returns an empty table for it by design, so
//! it is simply absent from `scan_waves`'s harmonic match arms, the same
//! "unclassified is not a scan target" reasoning the manual `XabcdPattern`
//! tool (mlc) already encodes.
//!
//! Phase 1 (MVP) kind coverage — single-degree families only, no
//! composition: `Impulse`; the 4 diagonal kinds (`Leading`/`Ending` ×
//! `Contracting`/`Expanding`); `Zigzag`; `Flat`; `Triangle`; the 11 harmonic
//! kinds above. A requested kind outside this set produces no hits — a
//! documented subset of the full `HarmonicKind` taxonomy, not a silent
//! refusal;
//! W-X-Y combo families extend this module in a later phase without
//! touching `scan_waves`'s signature.

use std::ops::Range;

use super::ratios;
use super::types::HarmonicKind;
use crate::ewa::chart_swing::{EwaSwingPivot, EwaSwingSegment};
use crate::ewa::rules;

/// Hard bound on how many pivots a single `scan_waves` call examines.
/// Enumeration truncates — never silently drops bars — at this many
/// pivots (the oldest-first prefix is kept); every hit produced while
/// truncated carries `truncated_family = true`, so a caller always knows
/// whether the survey saw the whole pivot list.
pub const MAX_PIVOTS_PER_SCAN: usize = 500;

/// One harmonic structure `scan_harmonics` found and validated.
#[derive(Debug, Clone, PartialEq)]
pub struct HarmonicWaveHit {
    pub kind: HarmonicKind,
    /// Bar indices, ordered, one per pivot in the structure (4 for the
    /// simple-correction family, 6 for impulse / diagonal / triangle).
    pub pivots: Vec<usize>,
    /// 0..=1 mean per-leg fit against each band's center. Ranking only.
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
///
/// Thin call-through to [`scan_waves_with_harmonic_tolerance`] at
/// `harmonic_tolerance_delta = 0.0` — today's exact-band behaviour,
/// byte-identical to every existing caller.
pub fn scan_harmonics(
    pivots: &[EwaSwingPivot],
    segments: &[EwaSwingSegment],
    kinds: &[HarmonicKind],
    completion_range: Range<usize>,
) -> Vec<HarmonicWaveHit> {
    scan_waves_with_harmonic_tolerance(pivots, segments, kinds, completion_range, 0.0)
}

/// Same contract as [`scan_waves`], plus `harmonic_tolerance_delta`: widens
/// (or, negative, narrows) every harmonic leg's target band via
/// `ewa::harmonic::widen_band` before `score_harmonic`'s hard band check —
/// the SAME widening `XabcdPattern`'s own per-drawing `harmonic_tolerance`
/// field already applies on the mlc side, so a caller can make
/// `score_harmonic`'s band check as loose or tight as a per-drawing
/// `tolerance` setting demands. `0.0` reproduces `ewa::harmonic::targets_for`'s
/// own unwidened bands exactly. Inert for the non-harmonic kinds this same
/// call also enumerates (Elliott's `soft_ratio_score` path never reads it).
pub fn scan_waves_with_harmonic_tolerance(
    pivots: &[EwaSwingPivot],
    segments: &[EwaSwingSegment],
    kinds: &[HarmonicKind],
    completion_range: Range<usize>,
    harmonic_tolerance_delta: f64,
) -> Vec<HarmonicWaveHit> {
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
            HarmonicKind::Gartley
            | HarmonicKind::Bat
            | HarmonicKind::AltBat
            | HarmonicKind::Butterfly
            | HarmonicKind::Crab
            | HarmonicKind::DeepCrab
            | HarmonicKind::Shark
            | HarmonicKind::Cypher => scan_harmonic_xabcd(
                kind,
                scan_pivots,
                &scan_segments,
                &by_start,
                &completion_range,
                harmonic_tolerance_delta,
                truncated,
                &mut hits,
            ),
            HarmonicKind::Abcd => scan_harmonic_abcd(
                scan_pivots,
                &scan_segments,
                &by_start,
                &completion_range,
                harmonic_tolerance_delta,
                truncated,
                &mut hits,
            ),
            HarmonicKind::ThreeDrives => scan_harmonic_three_drives(
                scan_pivots,
                &scan_segments,
                &by_start,
                &completion_range,
                harmonic_tolerance_delta,
                truncated,
                &mut hits,
            ),
            HarmonicKind::FiveZero => scan_harmonic_five_zero(
                scan_pivots,
                &scan_segments,
                &by_start,
                &completion_range,
                harmonic_tolerance_delta,
                truncated,
                &mut hits,
            ),
            HarmonicKind::Xabcd => {}
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

/// Pivot-name -> in-window position for `FiveZero`: `0, X, A, B, C, D`.
fn five_zero_position(name: &str) -> Option<usize> {
    match name {
        "0" => Some(0),
        "X" => Some(1),
        "A" => Some(2),
        "B" => Some(3),
        "C" => Some(4),
        "D" => Some(5),
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
/// must satisfy `harmonic::ratio_matches` (HARD — see module doc), evaluated
/// against `entry.band` widened by `harmonic_tolerance_delta` (`0.0`
/// reproduces the table's own unwidened bands exactly), or the whole
/// candidate is dropped; `score` is the mean per-leg fit quality, measured
/// against the table's own UNWIDENED band center/half-width (see
/// `harmonic_leg_fit`) so a wider tolerance never inflates a near-miss's
/// score, only whether it survives the hard drop at all. `position_of` maps
/// a table leg's pivot NAME (`"X"`, `"A"`, …) to its position within the
/// `pivot_count`-pivot window starting at `start`.
fn score_harmonic(
    kind: HarmonicKind,
    start: usize,
    pivot_count: usize,
    pivots: &[EwaSwingPivot],
    position_of: fn(&str) -> Option<usize>,
    harmonic_tolerance_delta: f64,
    truncated: bool,
) -> Option<HarmonicWaveHit> {
    let table = ratios::targets_for(kind);
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
        let widened = ratios::HarmonicLeg { band: ratios::widen_band(entry.band, harmonic_tolerance_delta), ..*entry };
        if !ratios::ratio_matches(ratio, &widened) {
            return None;
        }
        fit_sum += harmonic_leg_fit(ratio, &entry.band);
    }

    let score = (fit_sum / table.len() as f64).clamp(0.0, 1.0);
    let pivots_out = (0..pivot_count).map(|i| pivots[start + i].index).collect();
    Some(HarmonicWaveHit { kind, pivots: pivots_out, score, truncated_family: truncated })
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
    kind: HarmonicKind,
    pivots: &[EwaSwingPivot],
    segments: &[&EwaSwingSegment],
    by_start: &[Option<usize>],
    completion_range: &Range<usize>,
    harmonic_tolerance_delta: f64,
    truncated: bool,
    out: &mut Vec<HarmonicWaveHit>,
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
        if let Some(hit) = score_harmonic(kind, start, 5, pivots, xabcd_position, harmonic_tolerance_delta, truncated) {
            out.push(hit);
        }
    }
}

fn scan_harmonic_abcd(
    pivots: &[EwaSwingPivot],
    segments: &[&EwaSwingSegment],
    by_start: &[Option<usize>],
    completion_range: &Range<usize>,
    harmonic_tolerance_delta: f64,
    truncated: bool,
    out: &mut Vec<HarmonicWaveHit>,
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
        if let Some(hit) =
            score_harmonic(HarmonicKind::Abcd, start, 4, pivots, abcd_position, harmonic_tolerance_delta, truncated)
        {
            out.push(hit);
        }
    }
}

fn scan_harmonic_three_drives(
    pivots: &[EwaSwingPivot],
    segments: &[&EwaSwingSegment],
    by_start: &[Option<usize>],
    completion_range: &Range<usize>,
    harmonic_tolerance_delta: f64,
    truncated: bool,
    out: &mut Vec<HarmonicWaveHit>,
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
        if let Some(hit) = score_harmonic(
            HarmonicKind::ThreeDrives,
            start,
            6,
            pivots,
            three_drives_position,
            harmonic_tolerance_delta,
            truncated,
        ) {
            out.push(hit);
        }
    }
}

fn scan_harmonic_five_zero(
    pivots: &[EwaSwingPivot],
    segments: &[&EwaSwingSegment],
    by_start: &[Option<usize>],
    completion_range: &Range<usize>,
    harmonic_tolerance_delta: f64,
    truncated: bool,
    out: &mut Vec<HarmonicWaveHit>,
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
        if let Some(hit) = score_harmonic(
            HarmonicKind::FiveZero,
            start,
            6,
            pivots,
            five_zero_position,
            harmonic_tolerance_delta,
            truncated,
        ) {
            out.push(hit);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::ewa::chart_swing::{PivotKind, build_segments, extract_pivots};

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

    fn find_hit(hits: &[HarmonicWaveHit], kind: HarmonicKind) -> Option<&HarmonicWaveHit> {
        hits.iter().find(|h| h.kind == kind)
    }
    #[test]
    fn textbook_gartley_is_found_at_expected_pivots() {
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low];
        let prices = [1000.0, 1100.0, 1038.2, 1080.2, 1021.4];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_harmonics(&pivots, &segments, &[HarmonicKind::Gartley], 0..bars.len());

        let hit = find_hit(&hits, HarmonicKind::Gartley).expect("textbook Gartley must be found");
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
        let hits = scan_harmonics(&pivots, &segments, &[HarmonicKind::Bat], 0..bars.len());

        let hit = find_hit(&hits, HarmonicKind::Bat).expect("textbook Bat must be found");
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
        let hits = scan_harmonics(&pivots, &segments, &[HarmonicKind::Butterfly], 0..bars.len());

        let hit = find_hit(&hits, HarmonicKind::Butterfly).expect("textbook Butterfly must be found");
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
        let hits = scan_harmonics(&pivots, &segments, &[HarmonicKind::Crab], 0..bars.len());

        let hit = find_hit(&hits, HarmonicKind::Crab).expect("textbook Crab must be found");
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
        let hits = scan_harmonics(&pivots, &segments, &[HarmonicKind::Shark], 0..bars.len());

        let hit = find_hit(&hits, HarmonicKind::Shark).expect("textbook Shark must be found");
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
        let hits = scan_harmonics(&pivots, &segments, &[HarmonicKind::Cypher], 0..bars.len());

        let hit = find_hit(&hits, HarmonicKind::Cypher).expect("textbook Cypher must be found");
        assert_eq!(hit.pivots.len(), 5);
    }

    /// `AB = 0.382 XA` exact (Alt Bat's own AB target), `BC = 0.8 AB`
    /// (inside [0.382,0.886], real margin from both edges), `BC` solved so
    /// `AD` lands on Alt Bat's own `exact(1.13)`: X=1000, A=1100 (XA=100);
    /// AB=38.2; BC=30.56 (ratio 0.8); CD=105.36 (ratio ≈3.4477, inside
    /// [2.0,3.618] with margin from both edges); AD=113.0 (ratio 1.13
    /// exactly, D beyond X as Alt Bat's own deeper AD target implies).
    #[test]
    fn textbook_alt_bat_is_found_at_expected_pivots() {
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low];
        let prices = [1000.0, 1100.0, 1061.8, 1092.36, 987.0];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_harmonics(&pivots, &segments, &[HarmonicKind::AltBat], 0..bars.len());

        let hit = find_hit(&hits, HarmonicKind::AltBat).expect("textbook Alt Bat must be found");
        assert_eq!(hit.pivots.len(), 5);
    }

    /// `AB = 0.886 XA` exact (Deep Crab's own AB target), `BC = 0.55 AB`
    /// (inside [0.382,0.886]), `BC` solved so `AD` lands on Deep Crab's own
    /// `exact(1.618)`: X=1000, A=1100 (XA=100); AB=88.6; BC=48.73 (ratio
    /// 0.55); CD=121.93 (ratio ≈2.5023, inside [2.24,3.618] with margin
    /// from both edges); AD=161.8 (ratio 1.618 exactly).
    #[test]
    fn textbook_deep_crab_is_found_at_expected_pivots() {
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low];
        let prices = [1000.0, 1100.0, 1011.4, 1060.13, 938.2];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_harmonics(&pivots, &segments, &[HarmonicKind::DeepCrab], 0..bars.len());

        let hit = find_hit(&hits, HarmonicKind::DeepCrab).expect("textbook Deep Crab must be found");
        assert_eq!(hit.pivots.len(), 5);
    }

    /// The 6-pivot `0-X-A-B-C-D` shape: `AB = 1.35 XA` (mid [1.13,1.618]),
    /// `BC = 1.9 AB` (mid [1.618,2.24]), `CD = 0.5 BC` exact (5-0's own
    /// target). Leading `0` point (1050.0) anchors the window per the
    /// pattern's own 6-point definition but carries no ratio of its own.
    /// X=1000, A=1100 (XA=100); AB=135.0 (B=965.0, beyond X, matching 5-0's
    /// own deep-retracement B); BC=256.5 (C=1221.5); CD=128.25 (D=1093.25,
    /// ratio 0.5 exactly).
    #[test]
    fn textbook_five_zero_is_found_at_expected_pivots() {
        let kinds = [
            PivotKind::High,
            PivotKind::Low,
            PivotKind::High,
            PivotKind::Low,
            PivotKind::High,
            PivotKind::Low,
        ];
        let prices = [1050.0, 1000.0, 1100.0, 965.0, 1221.5, 1093.25];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_harmonics(&pivots, &segments, &[HarmonicKind::FiveZero], 0..bars.len());

        let hit = find_hit(&hits, HarmonicKind::FiveZero).expect("textbook 5-0 must be found");
        assert_eq!(hit.pivots.len(), 6);
    }

    #[test]
    fn five_zero_leg_outside_tolerance_is_dropped_not_penalized() {
        // Same shape as the textbook 5-0 fixture above, but CD (measured
        // against BC) is 0.7 -- far outside 5-0's own exact(0.5) band -- so
        // this is a near-miss, not a fib-exact structure, and must be
        // rejected outright. BC stays 256.5 (C=1221.5); CD=0.7*256.5=179.55
        // puts D at 1041.95 instead of the textbook fixture's 1093.25.
        let kinds = [
            PivotKind::High,
            PivotKind::Low,
            PivotKind::High,
            PivotKind::Low,
            PivotKind::High,
            PivotKind::Low,
        ];
        let prices = [1050.0, 1000.0, 1100.0, 965.0, 1221.5, 1041.95];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);
        let hits = scan_harmonics(&pivots, &segments, &[HarmonicKind::FiveZero], 0..bars.len());

        assert!(
            find_hit(&hits, HarmonicKind::FiveZero).is_none(),
            "a 5-0 CD ratio clearly outside its exact(0.5) band must be a HARD drop, not a scored-down hit: {hits:?}"
        );
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
        let hits = scan_harmonics(&pivots, &segments, &[HarmonicKind::Abcd], 0..bars.len());

        let hit = find_hit(&hits, HarmonicKind::Abcd).expect("textbook harmonic Abcd must be found");
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
        let hits = scan_harmonics(&pivots, &segments, &[HarmonicKind::ThreeDrives], 0..bars.len());

        let hit = find_hit(&hits, HarmonicKind::ThreeDrives).expect("textbook ThreeDrives must be found");
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
        let hits = scan_harmonics(&pivots, &segments, &[HarmonicKind::Gartley], 0..bars.len());

        assert!(
            find_hit(&hits, HarmonicKind::Gartley).is_none(),
            "a leg ratio clearly outside its harmonic band must be a HARD drop, not a scored-down hit: {hits:?}"
        );
    }

    /// `AB = 0.628 XA` — just outside Gartley's own `exact(0.618)` band
    /// (`[0.613, 0.623]`, `DEFAULT_HARMONIC_TOLERANCE = 0.005`) but inside a
    /// `0.02`-widened one (`[0.598, 0.643]`). `AD` stays exactly on its own
    /// `exact(0.786)` target (independent of `B`) so only the `AB` leg gates
    /// this fixture: X=1000, A=1100 (XA=100); AB=62.8 (B=1037.2); BC=43.96
    /// (ratio 0.7 of AB, C=1081.16); CD=59.76 (ratio ≈1.359 of BC, inside
    /// [1.13,1.618]); AD=78.6 (D=1021.4, ratio 0.786 exactly).
    #[test]
    fn wider_harmonic_tolerance_finds_a_near_miss_the_default_rejects() {
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low];
        let prices = [1000.0, 1100.0, 1037.2, 1081.16, 1021.4];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);

        let default_hits = scan_harmonics(&pivots, &segments, &[HarmonicKind::Gartley], 0..bars.len());
        assert!(
            find_hit(&default_hits, HarmonicKind::Gartley).is_none(),
            "AB ratio 0.628 sits outside Gartley's default exact(0.618) band and must be dropped at delta 0.0: {default_hits:?}"
        );

        let widened_hits =
            scan_waves_with_harmonic_tolerance(&pivots, &segments, &[HarmonicKind::Gartley], 0..bars.len(), 0.02);
        let hit = find_hit(&widened_hits, HarmonicKind::Gartley)
            .expect("a 0.02-widened tolerance must find the near-miss Gartley the default rejects");
        assert_eq!(hit.pivots.len(), 5);
    }

    /// `AB = 0.622 XA` — inside Gartley's own default `exact(0.618)` band
    /// (`[0.613, 0.623]`, margin `0.001` from the top edge) but outside a
    /// `-0.002`-narrowed one (`[0.615, 0.621]`). Same construction as the
    /// widened-tolerance fixture above: X=1000, A=1100 (XA=100); AB=62.2
    /// (B=1037.8); BC=43.54 (ratio 0.7 of AB, C=1081.34); CD=59.94 (ratio
    /// ≈1.3766 of BC, inside [1.13,1.618]); AD=78.6 (D=1021.4, ratio 0.786
    /// exactly — its own narrowed band `[0.783, 0.789]` still contains it).
    #[test]
    fn narrower_harmonic_tolerance_rejects_a_match_the_default_accepts() {
        let kinds = [PivotKind::Low, PivotKind::High, PivotKind::Low, PivotKind::High, PivotKind::Low];
        let prices = [1000.0, 1100.0, 1037.8, 1081.34, 1021.4];
        let bars = wave_bars(&kinds, &prices, 3, 0.2);

        let pivots = extract_pivots(&bars, 2);
        let segments = build_segments(&bars, &pivots);

        let default_hits = scan_harmonics(&pivots, &segments, &[HarmonicKind::Gartley], 0..bars.len());
        let hit = find_hit(&default_hits, HarmonicKind::Gartley)
            .expect("AB ratio 0.622 sits inside Gartley's default exact(0.618) band and must be found at delta 0.0");
        assert_eq!(hit.pivots.len(), 5);

        let narrowed_hits =
            scan_waves_with_harmonic_tolerance(&pivots, &segments, &[HarmonicKind::Gartley], 0..bars.len(), -0.002);
        assert!(
            find_hit(&narrowed_hits, HarmonicKind::Gartley).is_none(),
            "a -0.002-narrowed tolerance must reject the AB=0.622 Gartley the default accepts: {narrowed_hits:?}"
        );
    }
}
