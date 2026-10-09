use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

use super::config::EwaConfig;
use super::grammar::is_scenario_pattern;
use super::types::{
    CandidateId, EwaCandidate, EwaPatternKind, EwaPivot, EwaProofStatus, EwaScenario,
    EwaScenarioGroup, EwaScenarioStatus, EwaWorldAnalysis, ScenarioId, WorldId,
};

pub fn rank_candidates(
    candidates: Vec<EwaCandidate>,
    worlds: &[EwaWorldAnalysis],
    config: &EwaConfig,
) -> Vec<EwaScenario> {
    let groups = rank_candidate_groups(candidates, worlds, config);
    consolidate_scenarios(&groups, config)
}

pub fn rank_candidate_groups(
    candidates: Vec<EwaCandidate>,
    worlds: &[EwaWorldAnalysis],
    config: &EwaConfig,
) -> Vec<EwaScenarioGroup> {
    rank_candidate_group_refs(&candidates, worlds, config)
}

pub fn rank_candidate_group_refs(
    candidates: &[EwaCandidate],
    worlds: &[EwaWorldAnalysis],
    config: &EwaConfig,
) -> Vec<EwaScenarioGroup> {
    rank_candidate_ref_iter(candidates.iter(), worlds, config)
}

pub fn rank_world_candidate_groups(
    worlds: &[EwaWorldAnalysis],
    config: &EwaConfig,
) -> Vec<EwaScenarioGroup> {
    rank_candidate_ref_iter(
        worlds.iter().flat_map(|world| world.candidates.iter()),
        worlds,
        config,
    )
}

pub fn rank_world_candidate_groups_affected(
    worlds: &[EwaWorldAnalysis],
    config: &EwaConfig,
    affected_bar_start: usize,
) -> Vec<EwaScenarioGroup> {
    rank_candidate_ref_iter(
        worlds
            .iter()
            .flat_map(|world| world.candidates.iter())
            .filter(|candidate| {
                bar_span(candidate, worlds)
                    .map(|(_, bar_end)| bar_end >= affected_bar_start)
                    .unwrap_or(false)
            }),
        worlds,
        config,
    )
}

fn rank_candidate_ref_iter<'a>(
    candidates: impl IntoIterator<Item = &'a EwaCandidate>,
    worlds: &[EwaWorldAnalysis],
    config: &EwaConfig,
) -> Vec<EwaScenarioGroup> {
    let mut by_span = BTreeMap::<(usize, usize), Vec<&EwaCandidate>>::new();

    for candidate in candidates {
        if !candidate.is_rule_valid() || !is_scenario_pattern(candidate.pattern) {
            continue;
        }
        let Some(span) = bar_span(&candidate, worlds) else {
            continue;
        };
        by_span.entry(span).or_default().push(candidate);
    }

    let mut groups = Vec::with_capacity(by_span.len());
    for ((bar_start, bar_end), span_candidates) in by_span {
        let mut by_pattern = HashMap::<EwaPatternKind, Vec<&EwaCandidate>>::new();
        for candidate in span_candidates {
            by_pattern.entry(candidate.pattern).or_default().push(candidate);
        }

        let group_id = groups.len() + 1;
        let mut alternatives = Vec::with_capacity(by_pattern.len());
        for (_, pattern_candidates) in by_pattern {
            let supporting_worlds = pattern_candidates
                .iter()
                .map(|candidate| canonical_swing_world(&candidate.source_world))
                .collect::<HashSet<_>>()
                .len();
            let mean_confidence = pattern_candidates
                .iter()
                .map(|candidate| effective_confidence(candidate, config))
                .sum::<f64>()
                / pattern_candidates.len().max(1) as f64;
            let candidate = pattern_candidates
                .into_iter()
                .max_by(|a, b| {
                    effective_confidence(a, config)
                        .partial_cmp(&effective_confidence(b, config))
                        .unwrap_or(Ordering::Equal)
                })
                .expect("pattern candidate group cannot be empty");
            let support_bonus = ((supporting_worlds as f64).ln_1p() * 0.035).min(0.12);
            let proof_status = candidate_proof_status(
                candidate,
                config.confirmed_subdivision_threshold,
            );
            let status = if proof_status == EwaProofStatus::Proven {
                EwaScenarioStatus::Confirmed
            } else {
                EwaScenarioStatus::Provisional
            };
            let proof_multiplier = if status == EwaScenarioStatus::Confirmed {
                1.0
            } else {
                config.provisional_confidence_multiplier
            };
            let confidence =
                (0.80 * effective_confidence(candidate, config) + 0.20 * mean_confidence + support_bonus)
                    .clamp(0.0, 1.0)
                    * proof_multiplier;
            let invalidation_price = invalidation_price(candidate, worlds);
            let annotation = group_annotation(candidate, confidence, supporting_worlds);
            let candidate_id = CandidateId::from_candidate(
                WorldId::from_label(&candidate.source_world),
                candidate,
            );

            alternatives.push(EwaScenario {
                id: ScenarioId::new(candidate_id, bar_start, bar_end),
                candidate_id,
                rank: 0,
                candidate: candidate.clone(),
                confidence,
                probability: 0.0,
                status,
                proof_status,
                ambiguity_group: group_id,
                bar_start,
                bar_end,
                supporting_worlds,
                invalidation_price,
                annotation,
            });
        }

        alternatives.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    b.candidate
                        .pivot_indices
                        .len()
                        .cmp(&a.candidate.pivot_indices.len())
                })
        });
        alternatives.truncate(config.top_n.clamp(1, 4));
        for (rank, scenario) in alternatives.iter_mut().enumerate() {
            scenario.rank = rank + 1;
        }
        normalize_probabilities(&mut alternatives);

        groups.push(EwaScenarioGroup {
            id: group_id,
            bar_start,
            bar_end,
            alternatives,
        });
    }

    groups.sort_by(|a, b| {
        b.bar_end
            .cmp(&a.bar_end)
            .then_with(|| {
                (b.bar_end - b.bar_start)
                    .cmp(&(a.bar_end - a.bar_start))
            })
    });
    normalize_scenario_group_ids(&mut groups);
    groups
}

pub(crate) fn normalize_scenario_group_ids(groups: &mut [EwaScenarioGroup]) {
    for (index, group) in groups.iter_mut().enumerate() {
        group.id = index + 1;
        for alternative in &mut group.alternatives {
            alternative.ambiguity_group = group.id;
        }
    }
}

pub fn consolidate_scenarios(
    groups: &[EwaScenarioGroup],
    config: &EwaConfig,
) -> Vec<EwaScenario> {
    let Some(latest_end) = groups.iter().map(|group| group.bar_end).max() else {
        return Vec::new();
    };
    let frontier_slack = ((latest_end + 1) / 100).clamp(1, 8);
    let frontier_start = latest_end.saturating_sub(frontier_slack);
    let mut pool = groups
        .iter()
        .filter(|group| group.bar_end >= frontier_start)
        .flat_map(|group| group.alternatives.iter().cloned())
        .collect::<Vec<_>>();

    pool.sort_by(|a, b| {
        consolidated_confidence(b, latest_end)
            .partial_cmp(&consolidated_confidence(a, latest_end))
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.bar_end.cmp(&a.bar_end))
    });

    let mut selected = Vec::with_capacity(config.top_n.clamp(1, 4));
    let mut seen = HashSet::new();
    for mut scenario in pool {
        if !seen.insert(scenario.candidate.pattern) {
            continue;
        }
        scenario.confidence = consolidated_confidence(&scenario, latest_end);
        scenario.rank = selected.len() + 1;
        selected.push(scenario);
        if selected.len() >= config.top_n.clamp(1, 4) {
            break;
        }
    }

    normalize_probabilities(&mut selected);
    selected
}

fn canonical_swing_world(label: &str) -> &str {
    label.split('|').next().unwrap_or(label)
}

fn consolidated_confidence(scenario: &EwaScenario, latest_end: usize) -> f64 {
    let lag = latest_end.saturating_sub(scenario.bar_end) as f64;
    let recency = 1.0 / (1.0 + lag);
    let support = ((scenario.supporting_worlds as f64).ln_1p() * 0.025).min(0.08);
    (scenario.confidence * (0.90 + 0.10 * recency) + support).clamp(0.0, 1.0)
}

fn group_annotation(
    candidate: &EwaCandidate,
    confidence: f64,
    supporting_worlds: usize,
) -> String {
    format!(
        "{:?} span_candidate confidence={:.3} worlds={} raw={:.3} fib={:.3} matrix={:.3} geometry={:.3} subdivisions={:.3} violations={}",
        candidate.pattern,
        confidence,
        supporting_worlds,
        candidate.score,
        candidate.fib_confluence,
        candidate.fib_matrix_score,
        candidate.geometry_score,
        candidate.subdivision_score,
        candidate.hard_violations.len()
    )
}

fn effective_confidence(candidate: &EwaCandidate, config: &EwaConfig) -> f64 {
    config.probability_model.probability(candidate)
}

fn candidate_proof_status(candidate: &EwaCandidate, threshold: f64) -> EwaProofStatus {
    if !candidate.is_rule_valid() {
        EwaProofStatus::Invalid
    } else if candidate.subdivision_score >= threshold {
        EwaProofStatus::Proven
    } else if candidate.subdivision_score > 0.0 {
        EwaProofStatus::Partial
    } else {
        EwaProofStatus::Unresolved
    }
}

fn bar_span(candidate: &EwaCandidate, worlds: &[EwaWorldAnalysis]) -> Option<(usize, usize)> {
    let pivots = world_pivots(candidate, worlds)?;
    let first = candidate.pivot_indices.first().copied()?;
    let last = candidate.pivot_indices.last().copied()?;
    let start = pivots.get(first)?.index;
    let end = pivots.get(last)?.index;
    Some((start.min(end), start.max(end)))
}

fn normalize_probabilities(scenarios: &mut [EwaScenario]) {
    let total: f64 = scenarios
        .iter()
        .map(|scenario| scenario.confidence.max(0.0))
        .sum();
    if total <= f64::EPSILON {
        let fallback = 1.0 / scenarios.len().max(1) as f64;
        for scenario in scenarios {
            scenario.probability = fallback;
        }
        return;
    }

    for scenario in scenarios {
        scenario.probability = scenario.confidence.max(0.0) / total;
    }
}

fn invalidation_price(candidate: &EwaCandidate, worlds: &[EwaWorldAnalysis]) -> Option<f64> {
    let pivots = world_pivots(candidate, worlds)?;
    match candidate.pattern {
        EwaPatternKind::Impulse => candidate
            .pivot_indices
            .first()
            .and_then(|&idx| pivots.get(idx))
            .map(|p| p.price),
        EwaPatternKind::Correction | EwaPatternKind::Zigzag | EwaPatternKind::Flat => candidate
            .pivot_indices
            .get(1)
            .and_then(|&idx| pivots.get(idx))
            .map(|p| p.price),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{EwaPivotKind, EwaRatio};

    fn pivot(index: usize) -> EwaPivot {
        EwaPivot {
            index,
            time: index as i64,
            price: index as f64,
            kind: if index % 2 == 0 {
                EwaPivotKind::Low
            } else {
                EwaPivotKind::High
            },
            confirmed_index: index,
            confirmed_time: index as i64,
            source_pass: "test".to_string(),
        }
    }

    fn candidate(
        start: usize,
        end: usize,
        score: f64,
        pattern: EwaPatternKind,
    ) -> EwaCandidate {
        EwaCandidate {
            source_world: "test".to_string(),
            pattern,
            pivot_indices: (start..=end).collect(),
            segment_indices: Vec::new(),
            ratios: Vec::<EwaRatio>::new(),
            children: Vec::new(),
            subdivision_score: 0.0,
            technical_features: Default::default(),
            hard_violations: Vec::new(),
            fib_confluence: 0.0,
            fib_matrix_score: 0.0,
            harmonic_confluence: 0.0,
            geometry_score: 0.0,
            prior_probability: 0.0,
            soft_penalty: 1.0 - score,
            score,
        }
    }

    fn world(pivots: Vec<EwaPivot>) -> EwaWorldAnalysis {
        EwaWorldAnalysis {
            label: "test".to_string(),
            rule_settings: Default::default(),
            pivots,
            segments: Vec::new(),
            fib_relations: Vec::new(),
            candidates: Vec::new(),
        }
    }

    #[test]
    fn recency_breaks_score_ties() {
        let worlds = vec![world((0..20).map(pivot).collect())];
        let config = EwaConfig::default()
            .with_top_n(1)
            .with_selection_recency_weight(0.02);
        let scenarios = rank_candidates(
            vec![
                candidate(0, 3, 0.9, EwaPatternKind::Zigzag),
                candidate(10, 13, 0.9, EwaPatternKind::Zigzag),
            ],
            &worlds,
            &config,
        );

        assert_eq!(scenarios[0].candidate.pivot_indices[0], 10);
    }

    #[test]
    fn each_span_keeps_its_own_alternatives() {
        let worlds = vec![world((0..20).map(pivot).collect())];
        let config = EwaConfig::default().with_top_n(4);
        let groups = rank_candidate_groups(
            vec![
                candidate(4, 9, 0.90, EwaPatternKind::Impulse),
                candidate(4, 9, 0.89, EwaPatternKind::Zigzag),
                candidate(4, 9, 0.88, EwaPatternKind::DoubleZigzag),
                candidate(10, 15, 0.87, EwaPatternKind::Flat),
            ],
            &worlds,
            &config,
        );

        let span = groups
            .iter()
            .find(|group| group.bar_start == 4 && group.bar_end == 9)
            .unwrap();
        assert_eq!(span.alternatives.len(), 3);
        assert!(span.alternatives.iter().all(|scenario| {
            scenario.bar_start == 4 && scenario.bar_end == 9
        }));
    }

    #[test]
    fn consolidated_top_only_uses_current_frontier() {
        let worlds = vec![world((0..100).map(pivot).collect())];
        let config = EwaConfig::default().with_top_n(4);
        let scenarios = rank_candidates(
            vec![
                candidate(0, 5, 0.99, EwaPatternKind::Impulse),
                candidate(90, 95, 0.80, EwaPatternKind::Zigzag),
                candidate(91, 96, 0.78, EwaPatternKind::ExpandedFlat),
            ],
            &worlds,
            &config,
        );

        assert!(scenarios.iter().all(|scenario| scenario.bar_end >= 95));
    }

    #[test]
    fn hard_invalid_candidates_never_enter_scenario_groups() {
        let worlds = vec![world((0..20).map(pivot).collect())];
        let config = EwaConfig::default().with_top_n(4);
        let mut invalid = candidate(4, 9, 1.0, EwaPatternKind::Impulse);
        invalid.hard_violations.push("test invariant".into());

        let groups = rank_candidate_groups(
            vec![
                invalid,
                candidate(4, 9, 0.70, EwaPatternKind::Zigzag),
            ],
            &worlds,
            &config,
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].alternatives.len(), 1);
        assert_eq!(
            groups[0].alternatives[0].candidate.pattern,
            EwaPatternKind::Zigzag
        );
    }

    #[test]
    fn diagnostics_and_harmonics_do_not_occupy_ewa_scenario_slots() {
        let worlds = vec![world((0..20).map(pivot).collect())];
        let config = EwaConfig::default().with_top_n(4);
        let groups = rank_candidate_groups(
            vec![
                candidate(4, 7, 1.0, EwaPatternKind::Correction),
                candidate(4, 7, 1.0, EwaPatternKind::Abcd),
                candidate(4, 7, 0.70, EwaPatternKind::Zigzag),
            ],
            &worlds,
            &config,
        );

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].alternatives.len(), 1);
        assert_eq!(
            groups[0].alternatives[0].candidate.pattern,
            EwaPatternKind::Zigzag
        );
    }
}

fn world_pivots<'a>(
    candidate: &EwaCandidate,
    worlds: &'a [EwaWorldAnalysis],
) -> Option<&'a [EwaPivot]> {
    worlds
        .iter()
        .find(|world| world.label == candidate.source_world)
        .map(|world| world.pivots.as_slice())
}
