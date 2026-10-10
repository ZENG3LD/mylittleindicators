use super::types::{EwaRatio, EwaSegment};
use crate::fib::{FibRelation, FibRelationKind};

pub const RETRACEMENT_TARGETS: &[f64] = &[0.236, 0.382, 0.5, 0.618, 0.786, 0.886];
pub const EXTENSION_TARGETS: &[f64] = &[1.0, 1.272, 1.414, 1.618, 2.0, 2.618, 3.618];
pub const FIB_MATRIX_MAX_GAP: usize = 8;

pub fn ratio_to_targets(
    name: impl Into<String>,
    numerator: f64,
    denominator: f64,
    targets: &[f64],
    weight: f64,
) -> Option<EwaRatio> {
    if denominator.abs() <= f64::EPSILON || targets.is_empty() {
        return None;
    }

    let actual = (numerator / denominator).abs();
    if !actual.is_finite() {
        return None;
    }

    let mut best_target = targets[0];
    let mut best_error = (actual - best_target).abs();

    for &target in &targets[1..] {
        let error = (actual - target).abs();
        if error < best_error {
            best_target = target;
            best_error = error;
        }
    }

    Some(EwaRatio {
        name: name.into(),
        actual,
        target: best_target,
        error: best_error,
        weight,
    })
}

pub fn weighted_ratio_penalty(ratios: &[EwaRatio]) -> f64 {
    let total_weight: f64 = ratios.iter().map(|r| r.weight).sum();
    if total_weight <= f64::EPSILON {
        return 0.0;
    }

    ratios.iter().map(|r| r.error * r.weight).sum::<f64>() / total_weight
}

pub fn build_fib_relations(segments: &[EwaSegment]) -> Vec<FibRelation> {
    build_fib_relations_from_previous(segments, 0)
}

pub fn build_fib_relations_from_previous(
    segments: &[EwaSegment],
    min_previous_segment: usize,
) -> Vec<FibRelation> {
    let mut relations = Vec::with_capacity(
        segments
            .len()
            .saturating_sub(1)
            .saturating_mul(FIB_MATRIX_MAX_GAP.min(segments.len())),
    );

    for previous_idx in min_previous_segment..segments.len() {
        let max_current = (previous_idx + FIB_MATRIX_MAX_GAP).min(segments.len().saturating_sub(1));
        for current_idx in (previous_idx + 1)..=max_current {
            let previous = &segments[previous_idx];
            let current = &segments[current_idx];
            if previous.abs_price_delta <= f64::EPSILON {
                continue;
            }

            let kind = if previous.direction == current.direction {
                FibRelationKind::Extension
            } else {
                FibRelationKind::Retracement
            };
            let ratio = current.abs_price_delta / previous.abs_price_delta;
            if !ratio.is_finite() {
                continue;
            }

            let targets = match kind {
                FibRelationKind::Retracement => RETRACEMENT_TARGETS,
                FibRelationKind::Extension => EXTENSION_TARGETS,
            };
            let (nearest_target, error) = nearest_target(ratio, targets);

            relations.push(FibRelation {
                previous_segment: previous_idx,
                current_segment: current_idx,
                kind,
                ratio,
                nearest_target,
                error,
            });
        }
    }

    relations
}

pub fn nearest_target(actual: f64, targets: &[f64]) -> (f64, f64) {
    let mut best_target = targets.first().copied().unwrap_or(0.0);
    let mut best_error = (actual - best_target).abs();

    for &target in &targets[1..] {
        let error = (actual - target).abs();
        if error < best_error {
            best_target = target;
            best_error = error;
        }
    }

    (best_target, best_error)
}
