use std::collections::HashMap;

use super::grammar::{
    EwaWavePosition, allowed_in_position, is_corrective, is_flat,
    is_specific_simple_correction, is_triangle, is_zigzag, matches_class, slots_for,
};
use super::ratios::{
    EXTENSION_TARGETS, RETRACEMENT_TARGETS, ratio_to_targets, weighted_ratio_penalty,
};
use super::rules::{
    EXPANDED_FLAT_B_A, FLAT_B_A, REGULAR_FLAT_B_A, ZIGZAG_B_A, beyond, beyond_origin,
    not_beyond_origin, progresses, ratio, retracement_stays_above_origin,
};
use super::model::{EwaCandidate, EwaChildPattern, EwaPivot, EwaRuleSettings};
use super::types::{EwaPatternKind, EwaRatio, EwaSegment, EwaSegmentDirection};

pub trait EwaScanner {
    fn scan(
        &self,
        pivots: &[EwaPivot],
        segments: &[EwaSegment],
        settings: &EwaRuleSettings,
    ) -> Vec<EwaCandidate>;
}

#[derive(Debug, Default, Clone)]
pub struct CpuEwaScanner;

impl EwaScanner for CpuEwaScanner {
    fn scan(
        &self,
        pivots: &[EwaPivot],
        segments: &[EwaSegment],
        settings: &EwaRuleSettings,
    ) -> Vec<EwaCandidate> {
        self.scan_from(pivots, segments, settings, 0)
    }
}

impl CpuEwaScanner {
    pub fn scan_from(
        &self,
        pivots: &[EwaPivot],
        segments: &[EwaSegment],
        settings: &EwaRuleSettings,
        min_start_pivot: usize,
    ) -> Vec<EwaCandidate> {
        let mut out = Vec::new();
        let scan_impulse = settings.max_candidate_pivots >= 6;
        let segment_by_start = build_segment_index(segments, pivots.len());

        if pivots.len() >= 4 && segments.len() >= 3 {
            out.reserve(segments.len());
            for start in min_start_pivot..=(pivots.len() - 4) {
                if let Some(segment_indices) =
                    window_segment_indices::<3>(&segment_by_start, start)
                {
                    if settings.is_enabled(EwaPatternKind::Correction) {
                        out.push(score_correction(start, &segment_indices, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::Zigzag) {
                        out.push(score_zigzag(start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::RunningZigzag) {
                        out.push(score_running_zigzag(start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::Flat) {
                        out.push(score_flat(start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::RegularFlat) {
                        out.push(score_flat_variant(EwaPatternKind::RegularFlat, start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::ExpandedFlat) {
                        out.push(score_flat_variant(EwaPatternKind::ExpandedFlat, start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::RunningFlat) {
                        out.push(score_flat_variant(EwaPatternKind::RunningFlat, start, &segment_indices, pivots, segments, settings));
                    }
                }
            }
        }

        if scan_impulse && pivots.len() >= 6 && segments.len() >= 5 {
            out.reserve(segments.len().saturating_mul(2));
            for start in min_start_pivot..=(pivots.len() - 6) {
                if let Some(segment_indices) =
                    window_segment_indices::<5>(&segment_by_start, start)
                {
                    if settings.is_enabled(EwaPatternKind::Impulse) {
                        out.push(score_impulse(start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::ImpulseExtendedWave1) {
                        out.push(score_impulse_extension(EwaPatternKind::ImpulseExtendedWave1, start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::ImpulseExtendedWave3) {
                        out.push(score_impulse_extension(EwaPatternKind::ImpulseExtendedWave3, start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::ImpulseExtendedWave5) {
                        out.push(score_impulse_extension(EwaPatternKind::ImpulseExtendedWave5, start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::TruncatedImpulse) {
                        out.push(score_truncated_impulse(start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::LeadingDiagonalContracting) {
                        out.push(score_diagonal(EwaPatternKind::LeadingDiagonalContracting, start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::LeadingDiagonalExpanding) {
                        out.push(score_diagonal(EwaPatternKind::LeadingDiagonalExpanding, start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::EndingDiagonalContracting) {
                        out.push(score_diagonal(EwaPatternKind::EndingDiagonalContracting, start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::EndingDiagonalExpanding) {
                        out.push(score_diagonal(EwaPatternKind::EndingDiagonalExpanding, start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::Triangle) {
                        out.push(score_triangle(start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::ContractingTriangle) {
                        out.push(score_triangle_variant(EwaPatternKind::ContractingTriangle, start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::BarrierTriangle) {
                        out.push(score_triangle_variant(EwaPatternKind::BarrierTriangle, start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::ExpandingTriangle) {
                        out.push(score_triangle_variant(EwaPatternKind::ExpandingTriangle, start, &segment_indices, pivots, segments, settings));
                    }
                    if settings.is_enabled(EwaPatternKind::RunningTriangle) {
                        out.push(score_triangle_variant(EwaPatternKind::RunningTriangle, start, &segment_indices, pivots, segments, settings));
                    }
                }
            }
        }

        if settings.compose_corrections_on_cpu {
            compose_corrections(&mut out, pivots, settings);
        }

        out
    }
}

fn build_segment_index(segments: &[EwaSegment], pivot_count: usize) -> Vec<Option<usize>> {
    let mut by_start = vec![None; pivot_count.saturating_sub(1)];

    for (idx, segment) in segments.iter().enumerate() {
        if segment.end_pivot == segment.start_pivot + 1 && segment.start_pivot < by_start.len() {
            by_start[segment.start_pivot] = Some(idx);
        }
    }

    by_start
}

fn window_segment_indices<const N: usize>(
    segment_by_start: &[Option<usize>],
    start_pivot: usize,
) -> Option<[usize; N]> {
    let mut indices = [0usize; N];
    let mut i = 0;
    while i < N {
        indices[i] = segment_by_start.get(start_pivot + i).copied().flatten()?;
        i += 1;
    }
    Some(indices)
}

fn score_correction(
    start_pivot: usize,
    segment_indices: &[usize],
    segments: &[EwaSegment],
    settings: &EwaRuleSettings,
) -> EwaCandidate {
    let a = &segments[segment_indices[0]];
    let b = &segments[segment_indices[1]];
    let c = &segments[segment_indices[2]];

    let mut ratios = Vec::new();
    let mut hard_violations = Vec::new();

    if a.direction == b.direction || b.direction == c.direction || a.direction != c.direction {
        hard_violations
            .push("ABC directions must alternate with A and C in the same direction".into());
    }

    if let Some(r) = ratio_to_targets(
        "B/A retracement",
        b.abs_price_delta,
        a.abs_price_delta,
        RETRACEMENT_TARGETS,
        1.0,
    ) {
        ratios.push(r);
    }
    if let Some(r) = ratio_to_targets(
        "C/A extension",
        c.abs_price_delta,
        a.abs_price_delta,
        EXTENSION_TARGETS,
        1.25,
    ) {
        ratios.push(r);
    }

    candidate(
        EwaPatternKind::Correction,
        start_pivot,
        4,
        segment_indices,
        ratios,
        hard_violations,
        settings,
    )
}

fn score_triangle(
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    settings: &EwaRuleSettings,
) -> EwaCandidate {
    let mut ratios = Vec::new();
    let mut hard_violations = Vec::new();

    for pair in segment_indices.windows(2) {
        if segments[pair[0]].direction == segments[pair[1]].direction {
            hard_violations.push("Triangle legs must alternate direction".into());
            break;
        }
    }
    let geometry = triangle_geometry(start_pivot, pivots);
    if !(geometry.contracting || geometry.barrier || geometry.expanding) {
        hard_violations.push(
            "Triangle boundaries must contract, form a barrier, or expand across A-C-E and B-D"
                .into(),
        );
    }

    for (offset, pair) in segment_indices.windows(2).enumerate() {
        let current = &segments[pair[1]];
        let previous = &segments[pair[0]];
        if let Some(r) = ratio_to_targets(
            format!("triangle leg {} retracement", offset + 2),
            current.abs_price_delta,
            previous.abs_price_delta,
            RETRACEMENT_TARGETS,
            0.75,
        ) {
            ratios.push(r);
        }
    }

    candidate(
        EwaPatternKind::Triangle,
        start_pivot,
        6,
        segment_indices,
        ratios,
        hard_violations,
        settings,
    )
}

fn score_triangle_variant(
    pattern: EwaPatternKind,
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    settings: &EwaRuleSettings,
) -> EwaCandidate {
    let mut candidate = score_triangle(start_pivot, segment_indices, pivots, segments, settings);
    candidate.pattern = pattern;

    let geometry = triangle_geometry(start_pivot, pivots);
    candidate.hard_violations.retain(|violation| {
        violation
            != "Triangle boundaries must contract, form a barrier, or expand across A-C-E and B-D"
    });

    match pattern {
        EwaPatternKind::ContractingTriangle => {
            if !geometry.contracting {
                candidate.hard_violations.push(
                    "Contracting triangle must have converging A-C-E and B-D boundaries".into(),
                );
            }
            if geometry.running {
                candidate.hard_violations.push(
                    "Regular contracting triangle B must not exceed A origin".into(),
                );
            }
        }
        EwaPatternKind::BarrierTriangle => {
            if !geometry.barrier {
                candidate.hard_violations.push(
                    "Barrier triangle must keep one boundary horizontal while the opposite boundary contracts"
                        .into(),
                );
            }
        }
        EwaPatternKind::ExpandingTriangle => {
            if !geometry.expanding {
                candidate.hard_violations.push(
                    "Expanding triangle must have diverging A-C-E and B-D boundaries".into(),
                );
            }
        }
        EwaPatternKind::RunningTriangle => {
            if !geometry.running {
                candidate.hard_violations.push("Running triangle B should exceed A origin".into());
            }
            if !(geometry.contracting || geometry.barrier) {
                candidate.hard_violations.push(
                    "Running triangle must otherwise retain contracting or barrier geometry".into(),
                );
            }
        }
        _ => {}
    }

    recompute_candidate_score(&mut candidate, settings);
    candidate
}

#[derive(Debug, Clone, Copy)]
struct TriangleGeometry {
    contracting: bool,
    barrier: bool,
    expanding: bool,
    running: bool,
}

fn triangle_geometry(start: usize, pivots: &[EwaPivot]) -> TriangleGeometry {
    let p0 = pivots[start].price;
    let p1 = pivots[start + 1].price;
    let p2 = pivots[start + 2].price;
    let p3 = pivots[start + 3].price;
    let p4 = pivots[start + 4].price;
    let p5 = pivots[start + 5].price;
    let a_down = p1 < p0;
    let tolerance = (p1 - p0).abs().max(f64::EPSILON) * 0.12;

    let (actionary_contracts, reactionary_contracts, actionary_expands, reactionary_expands) =
        if a_down {
            (p1 < p3 && p3 < p5, p2 > p4, p1 > p3 && p3 > p5, p2 < p4)
        } else {
            (p1 > p3 && p3 > p5, p2 < p4, p1 < p3 && p3 < p5, p2 > p4)
        };
    let actionary_barrier = (p5 - p3).abs() <= tolerance;
    let reactionary_barrier = (p4 - p2).abs() <= tolerance;

    TriangleGeometry {
        contracting: actionary_contracts && reactionary_contracts,
        barrier: (actionary_barrier && reactionary_contracts)
            || (reactionary_barrier && actionary_contracts),
        expanding: actionary_expands && reactionary_expands,
        running: beyond_origin(p0, p1, p2),
    }
}

fn score_zigzag(
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    settings: &EwaRuleSettings,
) -> EwaCandidate {
    let a = &segments[segment_indices[0]];
    let b = &segments[segment_indices[1]];
    let c = &segments[segment_indices[2]];

    let mut ratios = Vec::new();
    let mut hard_violations = Vec::new();

    if a.direction == b.direction || b.direction == c.direction || a.direction != c.direction {
        hard_violations.push("Zigzag requires A and C trend legs with B retracing between them".into());
    }
    let b_a = ratio(b.abs_price_delta, a.abs_price_delta).unwrap_or(f64::INFINITY);
    if !ZIGZAG_B_A.contains(b_a) {
        hard_violations.push("Zigzag B must retrace less than 90% of A".into());
    }

    let p0 = pivots[start_pivot].price;
    let p1 = pivots[start_pivot + 1].price;
    let p2 = pivots[start_pivot + 2].price;
    let p3 = pivots[start_pivot + 3].price;
    if !not_beyond_origin(p0, p1, p2) {
        hard_violations.push("Zigzag B must not terminate beyond A origin".into());
    }
    if !beyond(p0, p1, p3) {
        hard_violations.push("Zigzag C must terminate beyond A endpoint".into());
    }

    if let Some(r) = ratio_to_targets(
        "B/A zigzag retracement",
        b.abs_price_delta,
        a.abs_price_delta,
        &[0.382, 0.5, 0.618, 0.786],
        1.25,
    ) {
        ratios.push(r);
    }
    if let Some(r) = ratio_to_targets(
        "C/A zigzag projection",
        c.abs_price_delta,
        a.abs_price_delta,
        &[0.618, 1.0, 1.272, 1.618],
        1.5,
    ) {
        ratios.push(r);
    }

    candidate(
        EwaPatternKind::Zigzag,
        start_pivot,
        4,
        segment_indices,
        ratios,
        hard_violations,
        settings,
    )
}

fn score_running_zigzag(
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    settings: &EwaRuleSettings,
) -> EwaCandidate {
    let mut candidate = score_zigzag(start_pivot, segment_indices, pivots, segments, settings);
    candidate.pattern = EwaPatternKind::RunningZigzag;

    let p0 = pivots[start_pivot].price;
    let p1 = pivots[start_pivot + 1].price;
    let p3 = pivots[start_pivot + 3].price;
    if !same_side_or_short_of(p0, p1, p3) {
        candidate
            .hard_violations
            .push("Running zigzag C should fail to exceed the end of A".into());
    }
    candidate
        .hard_violations
        .retain(|violation| violation != "Zigzag C must terminate beyond A endpoint");
    recompute_candidate_score(&mut candidate, settings);
    candidate
}

fn score_flat(
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    settings: &EwaRuleSettings,
) -> EwaCandidate {
    let a = &segments[segment_indices[0]];
    let b = &segments[segment_indices[1]];
    let c = &segments[segment_indices[2]];

    let mut ratios = Vec::new();
    let mut hard_violations = Vec::new();

    if a.direction == b.direction || b.direction == c.direction || a.direction != c.direction {
        hard_violations.push("Flat requires A and C in the same corrective direction".into());
    }
    let b_a = ratio(b.abs_price_delta, a.abs_price_delta).unwrap_or(0.0);
    if !FLAT_B_A.contains(b_a) {
        hard_violations.push("Flat B must retrace at least 90% of A".into());
    }

    let p0 = pivots[start_pivot].price;
    let p1 = pivots[start_pivot + 1].price;
    let p2 = pivots[start_pivot + 2].price;
    if !near_origin(p0, p1, p2, 0.20) && !beyond_origin(p0, p1, p2) {
        hard_violations.push("Flat B must terminate near or beyond A origin".into());
    }

    if let Some(r) = ratio_to_targets(
        "B/A flat retracement",
        b.abs_price_delta,
        a.abs_price_delta,
        &[0.886, 1.0, 1.13, 1.236],
        1.5,
    ) {
        ratios.push(r);
    }
    if let Some(r) = ratio_to_targets(
        "C/A flat projection",
        c.abs_price_delta,
        a.abs_price_delta,
        &[1.0, 1.236, 1.382, 1.618],
        1.25,
    ) {
        ratios.push(r);
    }

    candidate(
        EwaPatternKind::Flat,
        start_pivot,
        4,
        segment_indices,
        ratios,
        hard_violations,
        settings,
    )
}

fn score_flat_variant(
    pattern: EwaPatternKind,
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    settings: &EwaRuleSettings,
) -> EwaCandidate {
    let a = &segments[segment_indices[0]];
    let b = &segments[segment_indices[1]];
    let c = &segments[segment_indices[2]];
    let mut ratios = Vec::new();
    let mut hard_violations = Vec::new();

    if a.direction == b.direction || b.direction == c.direction || a.direction != c.direction {
        hard_violations.push(format!("{pattern:?} requires an A-B-C alternation"));
    }
    let b_a = ratio(b.abs_price_delta, a.abs_price_delta).unwrap_or(0.0);
    let valid_b_ratio = match pattern {
        EwaPatternKind::RegularFlat => REGULAR_FLAT_B_A.contains(b_a),
        EwaPatternKind::ExpandedFlat | EwaPatternKind::RunningFlat => {
            EXPANDED_FLAT_B_A.contains(b_a)
        }
        _ => FLAT_B_A.contains(b_a),
    };
    if !valid_b_ratio {
        hard_violations.push(format!("{pattern:?} has an invalid B/A retracement"));
    }

    let (b_targets, c_targets) = match pattern {
        EwaPatternKind::RegularFlat => (&[0.886, 1.0, 1.05][..], &[0.886, 1.0, 1.13][..]),
        EwaPatternKind::ExpandedFlat => (&[1.05, 1.13, 1.236, 1.382][..], &[1.236, 1.382, 1.618][..]),
        EwaPatternKind::RunningFlat => (&[1.05, 1.13, 1.236, 1.382][..], &[0.618, 0.786, 0.886][..]),
        _ => (RETRACEMENT_TARGETS, EXTENSION_TARGETS),
    };

    if let Some(r) = ratio_to_targets("B/A flat subtype", b.abs_price_delta, a.abs_price_delta, b_targets, 1.75) {
        ratios.push(r);
    }
    if let Some(r) = ratio_to_targets("C/A flat subtype", c.abs_price_delta, a.abs_price_delta, c_targets, 1.5) {
        ratios.push(r);
    }

    let p0 = pivots[start_pivot].price;
    let p1 = pivots[start_pivot + 1].price;
    let p2 = pivots[start_pivot + 2].price;
    let p3 = pivots[start_pivot + 3].price;
    match pattern {
        EwaPatternKind::RegularFlat => {
            if !near_origin(p0, p1, p2, 0.20) {
                hard_violations.push("Regular flat B should terminate near A origin".into());
            }
            if same_side_or_short_of(p0, p1, p3) {
                hard_violations.push("Regular flat C should slightly exceed A end".into());
            }
        }
        EwaPatternKind::ExpandedFlat => {
            if !exceeds_origin(p0, p1, p2) {
                hard_violations.push("Expanded flat B should exceed A origin".into());
            }
            if same_side_or_short_of(p0, p1, p3) {
                hard_violations.push("Expanded flat C should exceed A end".into());
            }
        }
        EwaPatternKind::RunningFlat => {
            if !exceeds_origin(p0, p1, p2) {
                hard_violations.push("Running flat B should exceed A origin".into());
            }
            if !same_side_or_short_of(p0, p1, p3) {
                hard_violations.push("Running flat C should fail to exceed A end".into());
            }
        }
        _ => {}
    }

    candidate(pattern, start_pivot, 4, segment_indices, ratios, hard_violations, settings)
}

fn score_impulse(
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    settings: &EwaRuleSettings,
) -> EwaCandidate {
    let w1 = &segments[segment_indices[0]];
    let w2 = &segments[segment_indices[1]];
    let w3 = &segments[segment_indices[2]];
    let w4 = &segments[segment_indices[3]];
    let w5 = &segments[segment_indices[4]];

    let mut ratios = Vec::new();
    let mut hard_violations = Vec::new();

    if !(w1.direction == w3.direction
        && w3.direction == w5.direction
        && w2.direction == w4.direction
        && w1.direction != w2.direction)
    {
        hard_violations.push(
            "Impulse requires motive waves 1/3/5 and corrective waves 2/4 to alternate".into(),
        );
    }

    let motive_lengths = [w1.abs_price_delta, w3.abs_price_delta, w5.abs_price_delta];
    if w3.abs_price_delta < motive_lengths[0].min(motive_lengths[2]) {
        hard_violations.push("Wave 3 must not be the shortest motive wave".into());
    }

    let p0 = pivots[start_pivot].price;
    let p1 = pivots[start_pivot + 1].price;
    let p2 = pivots[start_pivot + 2].price;
    let p3 = pivots[start_pivot + 3].price;
    let p4 = pivots[start_pivot + 4].price;
    let p5 = pivots[start_pivot + 5].price;
    if !retracement_stays_above_origin(w1.direction, p0, p2) {
        hard_violations.push("Wave 2 retraced beyond wave 1 origin".into());
    }
    if !progresses(w1.direction, p1, p3) {
        hard_violations.push("Wave 3 must terminate beyond wave 1 endpoint".into());
    }
    if !retracement_stays_above_origin(w1.direction, p2, p4) {
        hard_violations.push("Wave 4 retraced beyond wave 2 endpoint".into());
    }
    if !progresses(w1.direction, p1, p4) {
        hard_violations.push("Impulse wave 4 must not overlap wave 1 territory".into());
    }
    if !progresses(w1.direction, p3, p5) {
        hard_violations.push("Impulse wave 5 must terminate beyond wave 3 endpoint".into());
    }

    if let Some(r) = ratio_to_targets(
        "wave2/wave1 retracement",
        w2.abs_price_delta,
        w1.abs_price_delta,
        RETRACEMENT_TARGETS,
        1.0,
    ) {
        ratios.push(r);
    }
    if let Some(r) = ratio_to_targets(
        "wave3/wave1 extension",
        w3.abs_price_delta,
        w1.abs_price_delta,
        EXTENSION_TARGETS,
        1.5,
    ) {
        ratios.push(r);
    }
    if let Some(r) = ratio_to_targets(
        "wave4/wave3 retracement",
        w4.abs_price_delta,
        w3.abs_price_delta,
        RETRACEMENT_TARGETS,
        1.0,
    ) {
        ratios.push(r);
    }
    if let Some(r) = ratio_to_targets(
        "wave5/wave1 extension",
        w5.abs_price_delta,
        w1.abs_price_delta,
        EXTENSION_TARGETS,
        1.0,
    ) {
        ratios.push(r);
    }

    candidate(
        EwaPatternKind::Impulse,
        start_pivot,
        6,
        segment_indices,
        ratios,
        hard_violations,
        settings,
    )
}

fn score_impulse_extension(
    pattern: EwaPatternKind,
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    settings: &EwaRuleSettings,
) -> EwaCandidate {
    let mut candidate = score_impulse(start_pivot, segment_indices, pivots, segments, settings);
    candidate.pattern = pattern;

    let w1 = segments[segment_indices[0]].abs_price_delta;
    let w3 = segments[segment_indices[2]].abs_price_delta;
    let w5 = segments[segment_indices[4]].abs_price_delta;
    let target = match pattern {
        EwaPatternKind::ImpulseExtendedWave1 => (w1, w3.max(w5), "wave1 extension"),
        EwaPatternKind::ImpulseExtendedWave3 => (w3, w1.max(w5), "wave3 extension"),
        EwaPatternKind::ImpulseExtendedWave5 => (w5, w1.max(w3), "wave5 extension"),
        _ => (w3, w1.max(w5), "extension"),
    };

    if target.0 <= target.1 * 1.236 {
        candidate.hard_violations.push(format!("{} should dominate other actionary waves", target.2));
    }
    if let Some(r) = ratio_to_targets(target.2, target.0, target.1, &[1.618, 2.0, 2.618, 3.618], 1.75) {
        candidate.ratios.push(r);
    }
    recompute_candidate_score(&mut candidate, settings);
    candidate
}

fn score_truncated_impulse(
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    settings: &EwaRuleSettings,
) -> EwaCandidate {
    let mut candidate = score_impulse(start_pivot, segment_indices, pivots, segments, settings);
    candidate.pattern = EwaPatternKind::TruncatedImpulse;
    candidate
        .hard_violations
        .retain(|violation| violation != "Impulse wave 5 must terminate beyond wave 3 endpoint");

    let p0 = pivots[start_pivot].price;
    let p3 = pivots[start_pivot + 3].price;
    let p5 = pivots[start_pivot + 5].price;
    if !same_side_or_short_of(p0, p3, p5) {
        candidate.hard_violations.push("Truncated fifth should fail to exceed wave 3 extreme".into());
    }
    recompute_candidate_score(&mut candidate, settings);
    candidate
}

fn score_diagonal(
    pattern: EwaPatternKind,
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    settings: &EwaRuleSettings,
) -> EwaCandidate {
    let w1 = &segments[segment_indices[0]];
    let w2 = &segments[segment_indices[1]];
    let w3 = &segments[segment_indices[2]];
    let w4 = &segments[segment_indices[3]];
    let w5 = &segments[segment_indices[4]];
    let mut ratios = Vec::new();
    let mut hard_violations = Vec::new();

    if !(w1.direction == w3.direction
        && w3.direction == w5.direction
        && w2.direction == w4.direction
        && w1.direction != w2.direction)
    {
        hard_violations.push("Diagonal requires five alternating waves in motive direction".into());
    }

    if !wave4_overlaps_wave1(start_pivot, pivots, w1.direction) {
        hard_violations.push("Diagonal wave 4 should overlap wave 1 price territory".into());
    }
    if w3.abs_price_delta < w1.abs_price_delta.min(w5.abs_price_delta) {
        hard_violations.push("Diagonal wave 3 must not be the shortest actionary wave".into());
    }

    let p0 = pivots[start_pivot].price;
    let p1 = pivots[start_pivot + 1].price;
    let p2 = pivots[start_pivot + 2].price;
    let p3 = pivots[start_pivot + 3].price;
    let p4 = pivots[start_pivot + 4].price;
    if !retracement_stays_above_origin(w1.direction, p0, p2) {
        hard_violations.push("Diagonal wave 2 retraced beyond wave 1 origin".into());
    }
    if !progresses(w1.direction, p1, p3) {
        hard_violations.push("Diagonal wave 3 must terminate beyond wave 1 endpoint".into());
    }
    if !retracement_stays_above_origin(w1.direction, p2, p4) {
        hard_violations.push("Diagonal wave 4 retraced beyond wave 2 endpoint".into());
    }

    let contracting = matches!(
        pattern,
        EwaPatternKind::LeadingDiagonalContracting | EwaPatternKind::EndingDiagonalContracting
    );
    let initial_width = (p1 - p0).abs();
    let later_width = (p3 - p2).abs();
    if contracting {
        if later_width >= initial_width {
            hard_violations.push("Contracting diagonal boundaries must converge".into());
        }
    } else if later_width <= initial_width {
        hard_violations.push("Expanding diagonal boundaries must diverge".into());
    }

    if let Some(r) = ratio_to_targets("wave2/wave1 diagonal retracement", w2.abs_price_delta, w1.abs_price_delta, RETRACEMENT_TARGETS, 1.0) {
        ratios.push(r);
    }
    if let Some(r) = ratio_to_targets("wave4/wave3 diagonal retracement", w4.abs_price_delta, w3.abs_price_delta, RETRACEMENT_TARGETS, 1.0) {
        ratios.push(r);
    }

    candidate(pattern, start_pivot, 6, segment_indices, ratios, hard_violations, settings)
}

fn compose_corrections(
    out: &mut Vec<EwaCandidate>,
    pivots: &[EwaPivot],
    settings: &EwaRuleSettings,
) {
    let leaves = out
        .iter()
        .filter(|candidate| {
            candidate.is_rule_valid() && is_specific_simple_correction(candidate.pattern)
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut by_start = HashMap::<usize, Vec<usize>>::new();
    for (index, candidate) in leaves.iter().enumerate() {
        let Some(&start) = candidate.pivot_indices.first() else {
            continue;
        };
        by_start.entry(start).or_default().push(index);
    }

    if settings.max_candidate_pivots >= 10 && pivots.len() >= 10 {
        for start in 0..pivots.len() {
            if settings.is_enabled(EwaPatternKind::DoubleZigzag) {
                if let Some(composite) =
                    compose_double_zigzag(start, pivots, &leaves, &by_start, settings)
                {
                    out.push(composite);
                }
            }
            if settings.is_enabled(EwaPatternKind::DoubleThree) {
                if let Some(composite) =
                    compose_double_three(start, pivots, &leaves, &by_start, settings)
                {
                    out.push(composite);
                }
            }
        }
    }

    if settings.max_candidate_pivots >= 16 && pivots.len() >= 16 {
        for start in 0..pivots.len() {
            if settings.is_enabled(EwaPatternKind::TripleZigzag) {
                if let Some(composite) =
                    compose_triple_zigzag(start, pivots, &leaves, &by_start, settings)
                {
                    out.push(composite);
                }
            }
            if settings.is_enabled(EwaPatternKind::TripleThree) {
                if let Some(composite) =
                    compose_triple_three(start, pivots, &leaves, &by_start, settings)
                {
                    out.push(composite);
                }
            }
        }
    }
}

fn compose_double_zigzag(
    start: usize,
    pivots: &[EwaPivot],
    leaves: &[EwaCandidate],
    by_start: &HashMap<usize, Vec<usize>>,
    settings: &EwaRuleSettings,
) -> Option<EwaCandidate> {
    let mut best = None;
    for w in leaves_starting(leaves, by_start, start).filter(|c| is_zigzag(c.pattern)) {
        let w_end = *w.pivot_indices.last()?;
        for x in leaves_starting(leaves, by_start, w_end).filter(|c| is_corrective(c.pattern)) {
            let x_end = *x.pivot_indices.last()?;
            for y in leaves_starting(leaves, by_start, x_end).filter(|c| is_zigzag(c.pattern)) {
                if span_too_large(start, y, settings) {
                    continue;
                }
                if let Some(candidate) = compose_correction(
                    EwaPatternKind::DoubleZigzag,
                    start,
                    pivots,
                    &[
                        (w, EwaWavePosition::W),
                        (x, EwaWavePosition::X),
                        (y, EwaWavePosition::Y),
                    ],
                    true,
                    settings,
                ) {
                    keep_best(&mut best, candidate);
                }
            }
        }
    }
    best
}

fn compose_triple_zigzag(
    start: usize,
    pivots: &[EwaPivot],
    leaves: &[EwaCandidate],
    by_start: &HashMap<usize, Vec<usize>>,
    settings: &EwaRuleSettings,
) -> Option<EwaCandidate> {
    let mut best = None;
    for w in leaves_starting(leaves, by_start, start).filter(|c| is_zigzag(c.pattern)) {
        for x1 in next_leaves(leaves, by_start, w).filter(|c| is_corrective(c.pattern)) {
            for y in next_leaves(leaves, by_start, x1).filter(|c| is_zigzag(c.pattern)) {
                for x2 in next_leaves(leaves, by_start, y).filter(|c| is_corrective(c.pattern)) {
                    for z in next_leaves(leaves, by_start, x2).filter(|c| is_zigzag(c.pattern)) {
                        if span_too_large(start, z, settings) {
                            continue;
                        }
                        if let Some(candidate) = compose_correction(
                            EwaPatternKind::TripleZigzag,
                            start,
                            pivots,
                            &[
                                (w, EwaWavePosition::W),
                                (x1, EwaWavePosition::X),
                                (y, EwaWavePosition::Y),
                                (x2, EwaWavePosition::X),
                                (z, EwaWavePosition::Z),
                            ],
                            true,
                            settings,
                        ) {
                            keep_best(&mut best, candidate);
                        }
                    }
                }
            }
        }
    }
    best
}

fn compose_double_three(
    start: usize,
    pivots: &[EwaPivot],
    leaves: &[EwaCandidate],
    by_start: &HashMap<usize, Vec<usize>>,
    settings: &EwaRuleSettings,
) -> Option<EwaCandidate> {
    let mut best = None;
    for w in leaves_starting(leaves, by_start, start) {
        if is_triangle(w.pattern) {
            continue;
        }
        for x in next_leaves(leaves, by_start, w) {
            for y in next_leaves(leaves, by_start, x) {
                if span_too_large(start, y, settings) {
                    continue;
                }
                if !valid_combination_components(&[w, y]) {
                    continue;
                }
                let Some(candidate) = compose_correction(
                    EwaPatternKind::DoubleThree,
                    start,
                    pivots,
                    &[
                        (w, EwaWavePosition::W),
                        (x, EwaWavePosition::X),
                        (y, EwaWavePosition::Y),
                    ],
                    false,
                    settings,
                ) else {
                    continue;
                };
                keep_best(&mut best, candidate);
            }
        }
    }
    best
}

fn compose_triple_three(
    start: usize,
    pivots: &[EwaPivot],
    leaves: &[EwaCandidate],
    by_start: &HashMap<usize, Vec<usize>>,
    settings: &EwaRuleSettings,
) -> Option<EwaCandidate> {
    let mut best = None;
    for w in leaves_starting(leaves, by_start, start) {
        if is_triangle(w.pattern) {
            continue;
        }
        for x1 in next_leaves(leaves, by_start, w) {
            for y in next_leaves(leaves, by_start, x1) {
                if is_triangle(y.pattern) {
                    continue;
                }
                for x2 in next_leaves(leaves, by_start, y) {
                    for z in next_leaves(leaves, by_start, x2) {
                        if span_too_large(start, z, settings) {
                            continue;
                        }
                        if !valid_combination_components(&[w, y, z]) {
                            continue;
                        }
                        let Some(candidate) = compose_correction(
                            EwaPatternKind::TripleThree,
                            start,
                            pivots,
                            &[
                                (w, EwaWavePosition::W),
                                (x1, EwaWavePosition::X),
                                (y, EwaWavePosition::Y),
                                (x2, EwaWavePosition::X),
                                (z, EwaWavePosition::Z),
                            ],
                            false,
                            settings,
                        ) else {
                            continue;
                        };
                        keep_best(&mut best, candidate);
                    }
                }
            }
        }
    }
    best
}

fn compose_correction(
    pattern: EwaPatternKind,
    start: usize,
    pivots: &[EwaPivot],
    children: &[(&EwaCandidate, EwaWavePosition)],
    directional: bool,
    settings: &EwaRuleSettings,
) -> Option<EwaCandidate> {
    let mut hard_violations = Vec::new();
    let mut ratios = Vec::new();
    let component_spans = children
        .iter()
        .map(|(child, _)| {
            let start = *child.pivot_indices.first()?;
            let end = *child.pivot_indices.last()?;
            Some((start, end))
        })
        .collect::<Option<Vec<_>>>()?;

    let component_moves = component_spans
        .iter()
        .map(|&(component_start, component_end)| {
            pivots[component_end].price - pivots[component_start].price
        })
        .collect::<Vec<_>>();

    for (offset, pair) in component_moves.windows(2).enumerate() {
        if pair[0].signum() == pair[1].signum() {
            hard_violations.push(format!(
                "{pattern:?} component {} must reverse through an X connector",
                offset + 2
            ));
        }
        let targets = if offset % 2 == 0 {
            RETRACEMENT_TARGETS
        } else {
            &[0.618, 1.0, 1.272, 1.618]
        };
        if let Some(relation) = ratio_to_targets(
            format!("{pattern:?} component {} relation", offset + 2),
            pair[1].abs(),
            pair[0].abs(),
            targets,
            1.0,
        ) {
            ratios.push(relation);
        }
    }

    let actionary_moves = component_moves.iter().step_by(2).copied().collect::<Vec<_>>();
    if actionary_moves
        .windows(2)
        .any(|pair| pair[0].signum() != pair[1].signum())
    {
        hard_violations.push(format!("{pattern:?} W/Y/Z components must share direction"));
    }
    if directional {
        let first_end = component_spans[0].1;
        let final_end = component_spans.last()?.1;
        if !beyond(
            pivots[start].price,
            pivots[first_end].price,
            pivots[final_end].price,
        ) {
            hard_violations.push(format!("{pattern:?} must deepen the price correction"));
        }
    }

    let segment_indices = children
        .iter()
        .flat_map(|(child, _)| child.segment_indices.iter().copied())
        .collect::<Vec<_>>();
    let mut composite = candidate(
        pattern,
        start,
        component_spans.last()?.1 - start + 1,
        &segment_indices,
        ratios,
        hard_violations,
        settings,
    );
    composite.children = children
        .iter()
        .map(|(child, position)| EwaChildPattern {
            position: *position,
            pattern: child.pattern,
            pivot_indices: child.pivot_indices.clone(),
            score: child.score,
        })
        .collect();
    composite.subdivision_score = 1.0;
    let grammar_slots = slots_for(pattern);
    if grammar_slots.len() != composite.children.len() {
        composite
            .hard_violations
            .push(format!("{pattern:?} has incomplete grammar slots"));
    } else {
        for (child, slot) in composite.children.iter().zip(grammar_slots) {
            if child.position != slot.position
                || !matches_class(child.pattern, slot.class)
                || !allowed_in_position(child.pattern, slot.position)
            {
                composite.hard_violations.push(format!(
                    "{:?} is invalid in {:?} slot of {pattern:?}",
                    child.pattern, slot.position
                ));
            }
        }
    }
    let child_score = children
        .iter()
        .map(|(child, _)| child.score)
        .sum::<f64>()
        / children.len() as f64;
    composite.score *= child_score;
    if !composite.hard_violations.is_empty() {
        composite.score = 0.0;
    }
    Some(composite)
}

pub fn compose_correction_candidate(
    pattern: EwaPatternKind,
    pivots: &[EwaPivot],
    children: &[EwaCandidate],
    settings: &EwaRuleSettings,
) -> Option<EwaCandidate> {
    let slots = slots_for(pattern);
    if children.len() != slots.len() || !settings.is_enabled(pattern) {
        return None;
    }
    let start = *children.first()?.pivot_indices.first()?;
    let positioned = children
        .iter()
        .zip(slots)
        .map(|(child, slot)| (child, slot.position))
        .collect::<Vec<_>>();
    let directional = matches!(
        pattern,
        EwaPatternKind::DoubleZigzag | EwaPatternKind::TripleZigzag
    );
    let actionary = children.iter().step_by(2).collect::<Vec<_>>();
    if matches!(pattern, EwaPatternKind::DoubleThree | EwaPatternKind::TripleThree)
        && !valid_combination_components(&actionary)
    {
        return None;
    }
    compose_correction(
        pattern,
        start,
        pivots,
        &positioned,
        directional,
        settings,
    )
}

fn leaves_starting<'a>(
    leaves: &'a [EwaCandidate],
    by_start: &HashMap<usize, Vec<usize>>,
    start: usize,
) -> impl Iterator<Item = &'a EwaCandidate> {
    by_start
        .get(&start)
        .into_iter()
        .flatten()
        .filter_map(|&index| leaves.get(index))
}

fn next_leaves<'a>(
    leaves: &'a [EwaCandidate],
    by_start: &'a HashMap<usize, Vec<usize>>,
    current: &EwaCandidate,
) -> impl Iterator<Item = &'a EwaCandidate> {
    current
        .pivot_indices
        .last()
        .into_iter()
        .flat_map(move |&start| leaves_starting(leaves, by_start, start))
}

fn span_too_large(
    start: usize,
    final_child: &EwaCandidate,
    settings: &EwaRuleSettings,
) -> bool {
    final_child
        .pivot_indices
        .last()
        .map(|&end| end - start + 1 > settings.max_candidate_pivots)
        .unwrap_or(true)
}

fn valid_combination_components(components: &[&EwaCandidate]) -> bool {
    let zigzags = components
        .iter()
        .filter(|candidate| is_zigzag(candidate.pattern))
        .count();
    let triangles = components
        .iter()
        .filter(|candidate| is_triangle(candidate.pattern))
        .count();
    let alternating_forms = components.windows(2).all(|pair| {
        correction_form(pair[0].pattern) != correction_form(pair[1].pattern)
    });
    zigzags <= 1 && triangles <= 1 && alternating_forms
}

fn correction_form(pattern: EwaPatternKind) -> u8 {
    if is_zigzag(pattern) {
        1
    } else if is_flat(pattern) {
        2
    } else if is_triangle(pattern) {
        3
    } else {
        0
    }
}

fn keep_best(best: &mut Option<EwaCandidate>, candidate: EwaCandidate) {
    if best
        .as_ref()
        .map(|current| candidate.score > current.score)
        .unwrap_or(true)
    {
        *best = Some(candidate);
    }
}

fn candidate(
    pattern: EwaPatternKind,
    start_pivot: usize,
    pivot_count: usize,
    segment_indices: &[usize],
    ratios: Vec<EwaRatio>,
    hard_violations: Vec<String>,
    settings: &EwaRuleSettings,
) -> EwaCandidate {
    let soft_penalty = weighted_ratio_penalty(&ratios) * settings.ratio_error_scale.max(0.0);
    let score = if hard_violations.is_empty() {
        (1.0 - soft_penalty).clamp(0.0, 1.0)
    } else {
        0.0
    };

    EwaCandidate {
        source_world: String::new(),
        pattern,
        pivot_indices: (start_pivot..start_pivot + pivot_count).collect(),
        segment_indices: segment_indices.to_vec(),
        ratios,
        children: Vec::new(),
        subdivision_score: 0.0,
        technical_features: Default::default(),
        hard_violations,
        fib_confluence: 0.0,
        fib_matrix_score: 0.0,
        harmonic_confluence: 0.0,
        geometry_score: 0.0,
        prior_probability: 0.0,
        soft_penalty,
        score,
    }
}

fn recompute_candidate_score(candidate: &mut EwaCandidate, settings: &EwaRuleSettings) {
    candidate.soft_penalty = weighted_ratio_penalty(&candidate.ratios) * settings.ratio_error_scale.max(0.0);
    candidate.score = if candidate.hard_violations.is_empty() {
        (1.0 - candidate.soft_penalty).clamp(0.0, 1.0)
    } else {
        0.0
    };
}

fn same_side_or_short_of(origin: f64, actionary_end: f64, test: f64) -> bool {
    if actionary_end >= origin {
        test <= actionary_end
    } else {
        test >= actionary_end
    }
}

fn exceeds_origin(origin: f64, actionary_end: f64, test: f64) -> bool {
    if actionary_end >= origin {
        test < origin
    } else {
        test > origin
    }
}

fn near_origin(origin: f64, actionary_end: f64, test: f64, tolerance: f64) -> bool {
    let range = (actionary_end - origin).abs().max(f64::EPSILON);
    (test - origin).abs() / range <= tolerance
}

fn wave4_overlaps_wave1(start_pivot: usize, pivots: &[EwaPivot], direction: EwaSegmentDirection) -> bool {
    let p0 = pivots[start_pivot].price;
    let p1 = pivots[start_pivot + 1].price;
    let p4 = pivots[start_pivot + 4].price;
    match direction {
        EwaSegmentDirection::Up => p4 < p1 && p4 > p0,
        EwaSegmentDirection::Down => p4 > p1 && p4 < p0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ewa::model::EwaPivotKind;

    fn market(prices: &[f64]) -> (Vec<EwaPivot>, Vec<EwaSegment>) {
        let pivots = prices
            .iter()
            .enumerate()
            .map(|(index, &price)| EwaPivot {
                index,
                time: index as i64,
                price,
                kind: if index % 2 == 0 {
                    EwaPivotKind::High
                } else {
                    EwaPivotKind::Low
                },
                confirmed_index: index,
                confirmed_time: index as i64,
                source_pass: "test".into(),
            })
            .collect::<Vec<_>>();
        let segments = prices
            .windows(2)
            .enumerate()
            .map(|(index, pair)| {
                let delta = pair[1] - pair[0];
                EwaSegment {
                    start_pivot: index,
                    end_pivot: index + 1,
                    direction: EwaSegmentDirection::from_delta(delta),
                    bars: 1,
                    duration_ms: 1,
                    price_delta: delta,
                    abs_price_delta: delta.abs(),
                    slope_per_bar: delta,
                }
            })
            .collect();
        (pivots, segments)
    }

    fn scan_one(pattern: EwaPatternKind, prices: &[f64]) -> EwaCandidate {
        let (pivots, segments) = market(prices);
        let settings = EwaRuleSettings {
            enabled_patterns: vec![pattern],
            ..EwaRuleSettings::default()
        };
        CpuEwaScanner
            .scan(&pivots, &segments, &settings)
            .into_iter()
            .next()
            .expect("test window should produce one candidate")
    }

    #[test]
    fn flat_requires_ninety_percent_b_retracement() {
        let invalid = scan_one(EwaPatternKind::Flat, &[10.0, 0.0, 5.0, -2.0]);
        let valid = scan_one(EwaPatternKind::Flat, &[10.0, 0.0, 9.0, -2.0]);

        assert!(!invalid.is_rule_valid());
        assert!(valid.is_rule_valid());
    }

    #[test]
    fn zigzag_rejects_flat_sized_b_retracement() {
        let candidate = scan_one(EwaPatternKind::Zigzag, &[10.0, 0.0, 9.0, -2.0]);

        assert!(!candidate.is_rule_valid());
        assert!(candidate
            .hard_violations
            .iter()
            .any(|violation| violation.contains("less than 90%")));
    }

    #[test]
    fn impulse_rejects_wave_four_overlap() {
        let valid = scan_one(EwaPatternKind::Impulse, &[0.0, 10.0, 5.0, 20.0, 12.0, 25.0]);
        let overlap = scan_one(EwaPatternKind::Impulse, &[0.0, 10.0, 5.0, 20.0, 8.0, 25.0]);

        assert!(valid.is_rule_valid());
        assert!(!overlap.is_rule_valid());
        assert!(overlap
            .hard_violations
            .iter()
            .any(|violation| violation.contains("must not overlap")));
    }

    #[test]
    fn truncated_impulse_allows_fifth_to_stop_short() {
        let candidate =
            scan_one(EwaPatternKind::TruncatedImpulse, &[0.0, 10.0, 5.0, 20.0, 12.0, 18.0]);

        assert!(candidate.is_rule_valid());
    }

    #[test]
    fn contracting_and_running_triangles_are_distinct() {
        let contracting = scan_one(
            EwaPatternKind::ContractingTriangle,
            &[10.0, 0.0, 8.0, 2.0, 6.0, 4.0],
        );
        let running = scan_one(
            EwaPatternKind::RunningTriangle,
            &[10.0, 0.0, 12.0, 2.0, 8.0, 4.0],
        );

        assert!(contracting.is_rule_valid());
        assert!(running.is_rule_valid());
    }

    #[test]
    fn double_zigzag_is_composed_from_w_x_y_children() {
        let (pivots, segments) =
            market(&[100.0, 90.0, 95.0, 80.0, 90.0, 85.0, 95.0, 85.0, 90.0, 70.0]);
        let settings = EwaRuleSettings {
            enabled_patterns: vec![EwaPatternKind::Zigzag, EwaPatternKind::DoubleZigzag],
            ..EwaRuleSettings::default()
        };
        let candidate = CpuEwaScanner
            .scan(&pivots, &segments, &settings)
            .into_iter()
            .find(|candidate| candidate.pattern == EwaPatternKind::DoubleZigzag)
            .expect("strict W-X-Y composition should be found");

        assert!(candidate.is_rule_valid());
        assert_eq!(candidate.children.len(), 3);
        assert_eq!(
            candidate
                .children
                .iter()
                .map(|child| child.position)
                .collect::<Vec<_>>(),
            vec![
                EwaWavePosition::W,
                EwaWavePosition::X,
                EwaWavePosition::Y
            ]
        );
        assert!(candidate
            .children
            .iter()
            .all(|child| child.pattern == EwaPatternKind::Zigzag));
    }

    #[test]
    fn gpu_first_mode_skips_cpu_compositions() {
        let (pivots, segments) =
            market(&[100.0, 90.0, 95.0, 80.0, 90.0, 85.0, 95.0, 85.0, 90.0, 70.0]);
        let settings = EwaRuleSettings {
            compose_corrections_on_cpu: false,
            enabled_patterns: vec![EwaPatternKind::Zigzag, EwaPatternKind::DoubleZigzag],
            ..EwaRuleSettings::default()
        };
        let candidates = CpuEwaScanner.scan(&pivots, &segments, &settings);

        assert!(candidates.iter().any(|candidate| candidate.pattern == EwaPatternKind::Zigzag));
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.pattern != EwaPatternKind::DoubleZigzag)
        );
    }

    #[test]
    fn double_three_can_end_with_triangle_component() {
        let (pivots, segments) = market(&[
            100.0, 90.0, 99.0, 88.0, 98.0, 93.0, 100.0, 90.0, 98.0, 92.0, 96.0, 94.0,
        ]);
        let settings = EwaRuleSettings {
            enabled_patterns: vec![
                EwaPatternKind::Zigzag,
                EwaPatternKind::RegularFlat,
                EwaPatternKind::ContractingTriangle,
                EwaPatternKind::DoubleThree,
            ],
            ..EwaRuleSettings::default()
        };
        let candidate = CpuEwaScanner
            .scan(&pivots, &segments, &settings)
            .into_iter()
            .find(|candidate| candidate.pattern == EwaPatternKind::DoubleThree)
            .expect("flat-X-triangle combination should be found");

        assert!(candidate.is_rule_valid());
        assert_eq!(candidate.children.len(), 3);
        assert_eq!(
            candidate.children.last().map(|child| child.pattern),
            Some(EwaPatternKind::ContractingTriangle)
        );
    }
}
