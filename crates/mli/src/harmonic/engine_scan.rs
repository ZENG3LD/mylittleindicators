use crate::ewa::model::EwaPivot;
use crate::ewa::ratios::{
    EXTENSION_TARGETS, RETRACEMENT_TARGETS, ratio_to_targets, weighted_ratio_penalty,
};
use crate::ewa::types::{EwaRatio, EwaSegment};

use super::types::{HarmonicHit, HarmonicKind};

pub fn kinds() -> &'static [HarmonicKind] {
    &[
        HarmonicKind::Xabcd,
        HarmonicKind::Abcd,
        HarmonicKind::Cypher,
        HarmonicKind::Gartley,
        HarmonicKind::Bat,
        HarmonicKind::Butterfly,
        HarmonicKind::Crab,
        HarmonicKind::Shark,
        HarmonicKind::ThreeDrives,
    ]
}

pub fn scan(
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    ratio_error_scale: f64,
) -> Vec<HarmonicHit> {
    let mut out = Vec::new();
    let segment_by_start = build_segment_index(segments, pivots.len());

    if pivots.len() >= 4 && segments.len() >= 3 {
        for start in 0..=(pivots.len() - 4) {
            let Some(segment_indices) = window_segment_indices::<3>(&segment_by_start, start) else {
                continue;
            };
            out.push(score_abcd(start, &segment_indices, segments, ratio_error_scale));
        }
    }

    if pivots.len() >= 5 && segments.len() >= 4 {
        for start in 0..=(pivots.len() - 5) {
            let Some(segment_indices) = window_segment_indices::<4>(&segment_by_start, start) else {
                continue;
            };
            out.push(score_xabcd(
                start,
                &segment_indices,
                pivots,
                segments,
                ratio_error_scale,
            ));
            out.push(score_cypher(
                start,
                &segment_indices,
                pivots,
                segments,
                ratio_error_scale,
            ));
            out.push(score_gartley(
                start,
                &segment_indices,
                pivots,
                segments,
                ratio_error_scale,
            ));
            out.push(score_bat(
                start,
                &segment_indices,
                pivots,
                segments,
                ratio_error_scale,
            ));
            out.push(score_butterfly(
                start,
                &segment_indices,
                pivots,
                segments,
                ratio_error_scale,
            ));
            out.push(score_crab(
                start,
                &segment_indices,
                pivots,
                segments,
                ratio_error_scale,
            ));
            out.push(score_shark(
                start,
                &segment_indices,
                pivots,
                segments,
                ratio_error_scale,
            ));
        }
    }

    out
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

fn score_abcd(
    start_pivot: usize,
    segment_indices: &[usize],
    segments: &[EwaSegment],
    ratio_error_scale: f64,
) -> HarmonicHit {
    let ab = &segments[segment_indices[0]];
    let bc = &segments[segment_indices[1]];
    let cd = &segments[segment_indices[2]];

    let mut ratios = Vec::new();
    let mut hard_violations = Vec::new();

    if ab.direction == bc.direction || bc.direction == cd.direction || ab.direction != cd.direction {
        hard_violations.push("ABCD legs must alternate with AB and CD in the same direction".into());
    }

    if let Some(r) = ratio_to_targets(
        "BC/AB retracement",
        bc.abs_price_delta,
        ab.abs_price_delta,
        RETRACEMENT_TARGETS,
        1.0,
    ) {
        ratios.push(r);
    }
    if let Some(r) = ratio_to_targets(
        "CD/BC extension",
        cd.abs_price_delta,
        bc.abs_price_delta,
        EXTENSION_TARGETS,
        1.25,
    ) {
        ratios.push(r);
    }
    if let Some(r) = ratio_to_targets(
        "CD/AB symmetry",
        cd.abs_price_delta,
        ab.abs_price_delta,
        &[1.0, 1.272, 1.618],
        1.0,
    ) {
        ratios.push(r);
    }

    hit(HarmonicKind::Abcd, start_pivot, 4, ratios, &hard_violations, ratio_error_scale)
}

fn score_xabcd(
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    ratio_error_scale: f64,
) -> HarmonicHit {
    let xa = &segments[segment_indices[0]];
    let ab = &segments[segment_indices[1]];
    let bc = &segments[segment_indices[2]];
    let cd = &segments[segment_indices[3]];

    let mut ratios = Vec::new();
    let mut hard_violations = Vec::new();

    for pair in segment_indices.windows(2) {
        if segments[pair[0]].direction == segments[pair[1]].direction {
            hard_violations.push("XABCD legs must alternate direction".into());
            break;
        }
    }

    if let Some(r) = ratio_to_targets(
        "AB/XA retracement",
        ab.abs_price_delta,
        xa.abs_price_delta,
        &[0.382, 0.5, 0.618, 0.786, 0.886],
        1.25,
    ) {
        ratios.push(r);
    }
    if let Some(r) = ratio_to_targets(
        "BC/AB retracement",
        bc.abs_price_delta,
        ab.abs_price_delta,
        RETRACEMENT_TARGETS,
        1.0,
    ) {
        ratios.push(r);
    }
    if let Some(r) = ratio_to_targets(
        "CD/BC extension",
        cd.abs_price_delta,
        bc.abs_price_delta,
        EXTENSION_TARGETS,
        1.25,
    ) {
        ratios.push(r);
    }

    let x = pivots[start_pivot].price;
    let a = pivots[start_pivot + 1].price;
    let d = pivots[start_pivot + 4].price;
    if let Some(r) = ratio_to_targets(
        "XD/XA completion",
        (d - x).abs(),
        (a - x).abs(),
        &[0.786, 0.886, 1.0, 1.272, 1.618],
        1.5,
    ) {
        ratios.push(r);
    }

    hit(HarmonicKind::Xabcd, start_pivot, 5, ratios, &hard_violations, ratio_error_scale)
}

fn score_cypher(
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    ratio_error_scale: f64,
) -> HarmonicHit {
    let xa = &segments[segment_indices[0]];
    let ab = &segments[segment_indices[1]];

    let mut ratios = Vec::new();
    let mut hard_violations = Vec::new();

    for pair in segment_indices.windows(2) {
        if segments[pair[0]].direction == segments[pair[1]].direction {
            hard_violations.push("Cypher legs must alternate direction".into());
            break;
        }
    }

    if let Some(r) = ratio_to_targets(
        "AB/XA retracement",
        ab.abs_price_delta,
        xa.abs_price_delta,
        &[0.382, 0.5, 0.618],
        1.25,
    ) {
        ratios.push(r);
    }

    let x = pivots[start_pivot].price;
    let a = pivots[start_pivot + 1].price;
    let c = pivots[start_pivot + 3].price;
    let d = pivots[start_pivot + 4].price;
    if let Some(r) = ratio_to_targets(
        "XC/XA extension",
        (c - x).abs(),
        (a - x).abs(),
        &[1.272, 1.414],
        1.5,
    ) {
        ratios.push(r);
    }
    if let Some(r) = ratio_to_targets(
        "CD/XC retracement",
        (d - c).abs(),
        (c - x).abs(),
        &[0.786],
        1.5,
    ) {
        ratios.push(r);
    }

    if (c - x).abs() <= (a - x).abs() {
        hard_violations.push("Cypher C should extend beyond XA".into());
    }

    hit(HarmonicKind::Cypher, start_pivot, 5, ratios, &hard_violations, ratio_error_scale)
}

fn score_gartley(
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    ratio_error_scale: f64,
) -> HarmonicHit {
    score_harmonic_xabcd(
        HarmonicKind::Gartley,
        start_pivot,
        segment_indices,
        pivots,
        segments,
        ratio_error_scale,
        HarmonicTargets {
            ab_xa: &[0.618],
            bc_ab: &[0.382, 0.5, 0.618, 0.786, 0.886],
            cd_bc: &[1.272, 1.618],
            xd_xa: &[0.786],
        },
    )
}

fn score_bat(
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    ratio_error_scale: f64,
) -> HarmonicHit {
    score_harmonic_xabcd(
        HarmonicKind::Bat,
        start_pivot,
        segment_indices,
        pivots,
        segments,
        ratio_error_scale,
        HarmonicTargets {
            ab_xa: &[0.382, 0.5],
            bc_ab: &[0.382, 0.5, 0.618, 0.786, 0.886],
            cd_bc: &[1.618, 2.0, 2.618],
            xd_xa: &[0.886],
        },
    )
}

fn score_butterfly(
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    ratio_error_scale: f64,
) -> HarmonicHit {
    score_harmonic_xabcd(
        HarmonicKind::Butterfly,
        start_pivot,
        segment_indices,
        pivots,
        segments,
        ratio_error_scale,
        HarmonicTargets {
            ab_xa: &[0.786],
            bc_ab: &[0.382, 0.5, 0.618, 0.786, 0.886],
            cd_bc: &[1.618, 2.0, 2.24],
            xd_xa: &[1.272, 1.618],
        },
    )
}

fn score_crab(
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    ratio_error_scale: f64,
) -> HarmonicHit {
    score_harmonic_xabcd(
        HarmonicKind::Crab,
        start_pivot,
        segment_indices,
        pivots,
        segments,
        ratio_error_scale,
        HarmonicTargets {
            ab_xa: &[0.382, 0.5, 0.618],
            bc_ab: &[0.382, 0.5, 0.618, 0.786, 0.886],
            cd_bc: &[2.24, 2.618, 3.14, 3.618],
            xd_xa: &[1.618],
        },
    )
}

fn score_shark(
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    ratio_error_scale: f64,
) -> HarmonicHit {
    score_harmonic_xabcd(
        HarmonicKind::Shark,
        start_pivot,
        segment_indices,
        pivots,
        segments,
        ratio_error_scale,
        HarmonicTargets {
            ab_xa: &[0.5, 0.618, 0.786, 0.886],
            bc_ab: &[1.13, 1.272, 1.414, 1.618],
            cd_bc: &[1.618, 2.0, 2.24],
            xd_xa: &[0.886, 1.13],
        },
    )
}

struct HarmonicTargets<'a> {
    ab_xa: &'a [f64],
    bc_ab: &'a [f64],
    cd_bc: &'a [f64],
    xd_xa: &'a [f64],
}

fn score_harmonic_xabcd(
    kind: HarmonicKind,
    start_pivot: usize,
    segment_indices: &[usize],
    pivots: &[EwaPivot],
    segments: &[EwaSegment],
    ratio_error_scale: f64,
    targets: HarmonicTargets<'_>,
) -> HarmonicHit {
    let xa = &segments[segment_indices[0]];
    let ab = &segments[segment_indices[1]];
    let bc = &segments[segment_indices[2]];
    let cd = &segments[segment_indices[3]];

    let mut ratios = Vec::new();
    let mut hard_violations = Vec::new();

    for pair in segment_indices.windows(2) {
        if segments[pair[0]].direction == segments[pair[1]].direction {
            hard_violations.push(format!("{kind:?} legs must alternate direction"));
            break;
        }
    }

    if let Some(r) = ratio_to_targets(
        "AB/XA retracement",
        ab.abs_price_delta,
        xa.abs_price_delta,
        targets.ab_xa,
        1.5,
    ) {
        ratios.push(r);
    }
    if let Some(r) = ratio_to_targets(
        "BC/AB retracement",
        bc.abs_price_delta,
        ab.abs_price_delta,
        targets.bc_ab,
        1.0,
    ) {
        ratios.push(r);
    }
    if let Some(r) = ratio_to_targets(
        "CD/BC projection",
        cd.abs_price_delta,
        bc.abs_price_delta,
        targets.cd_bc,
        1.25,
    ) {
        ratios.push(r);
    }

    let x = pivots[start_pivot].price;
    let a = pivots[start_pivot + 1].price;
    let d = pivots[start_pivot + 4].price;
    if let Some(r) = ratio_to_targets(
        "XD/XA completion",
        (d - x).abs(),
        (a - x).abs(),
        targets.xd_xa,
        1.75,
    ) {
        ratios.push(r);
    }

    hit(kind, start_pivot, 5, ratios, &hard_violations, ratio_error_scale)
}

fn hit(
    kind: HarmonicKind,
    start_pivot: usize,
    pivot_count: usize,
    ratios: Vec<EwaRatio>,
    hard_violations: &[String],
    ratio_error_scale: f64,
) -> HarmonicHit {
    let soft_penalty = weighted_ratio_penalty(&ratios) * ratio_error_scale.max(0.0);
    let score = if hard_violations.is_empty() {
        (1.0 - soft_penalty).clamp(0.0, 1.0)
    } else {
        0.0
    };
    HarmonicHit {
        kind,
        pivot_indices: (start_pivot..start_pivot + pivot_count).collect(),
        score,
    }
}
