use crate::core::types::Bar;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use rayon::prelude::*;

use super::config::EwaConfig;
use super::grammar::{EwaGrammarClass, EwaWavePosition, allowed_in_position, matches_class, slots_for};
use super::priors::prior_for;
use super::ranker::{
    consolidate_scenarios, normalize_scenario_group_ids, rank_world_candidate_groups,
    rank_world_candidate_groups_affected,
};
use super::reinterpretation::build_reinterpretations;
use super::scanner::{CpuEwaScanner, EwaScanner};
use super::swing::EwaSwingExtractor;
use super::types::{
    CandidateId, CountNodeId, EwaAffectedRange, EwaAnalysis, EwaCandidate, EwaCountNode,
    EwaFibRelation, EwaHypothesisSource, EwaNestingRelation, EwaPatternKind, EwaProofStatus,
    EwaRefreshTimings, EwaScenario, EwaScenarioStatus, EwaSegment, EwaSubdivisionMatch,
    EwaSwingCoverage, EwaSwingFailure, EwaSwingHypothesis, EwaWorldAnalysis, ScenarioId,
    WorldId,
};

#[derive(Debug, Clone)]
pub struct EwaEngine<S = CpuEwaScanner> {
    config: EwaConfig,
    scanner: S,
}

impl Default for EwaEngine<CpuEwaScanner> {
    fn default() -> Self {
        Self::new(EwaConfig::default())
    }
}

impl EwaEngine<CpuEwaScanner> {
    pub fn new(config: EwaConfig) -> Self {
        Self {
            config,
            scanner: CpuEwaScanner,
        }
    }
}

impl<S> EwaEngine<S>
where
    S: EwaScanner,
{
    pub fn with_scanner(config: EwaConfig, scanner: S) -> Self {
        Self { config, scanner }
    }

    pub fn config(&self) -> &EwaConfig {
        &self.config
    }

    pub fn analyze(&self, bars: &[Bar]) -> EwaAnalysis {
        let mut analysis = self.analyze_primitives(bars);
        refresh_analysis_derived(&mut analysis, &self.config);
        analysis
    }

    pub fn analyze_primitives(&self, bars: &[Bar]) -> EwaAnalysis {
        let extractor = EwaSwingExtractor::new(self.config.clone());
        let mut worlds = extractor.extract_worlds(bars);

        for world in &mut worlds {
            world.candidates = self
                .scanner
                .scan(&world.pivots, &world.segments, &world.rule_settings);
            for candidate in &mut world.candidates {
                candidate.source_world = world.label.clone();
                enrich_candidate_confluence(
                    candidate,
                    &world.fib_relations,
                    &world.segments,
                    &world.pivots,
                );
            }
        }
        let pivots = worlds
            .iter()
            .flat_map(|world| world.pivots.iter().cloned())
            .collect();
        let segments = worlds
            .iter()
            .flat_map(|world| world.segments.iter().cloned())
            .collect();
        EwaAnalysis {
            pivots,
            segments,
            scenario_groups: Vec::new(),
            scenarios: Vec::new(),
            nesting_relations: Vec::new(),
            count_nodes: Vec::new(),
            subdivision_matches: Vec::new(),
            reinterpretations: Vec::new(),
            swing_coverage: Vec::new(),
            worlds,
        }
    }
}

pub fn apply_count_proof_to_scenarios(
    scenarios: &mut [EwaScenario],
    count_nodes: &[EwaCountNode],
    provisional_multiplier: f64,
) {
    for scenario in scenarios.iter_mut() {
        let proof_status = count_nodes
            .iter()
            .find(|node| {
                node.parent_id.is_none()
                    && node.scenario_id == scenario.id
                    && node.position == EwaWavePosition::Root
            })
            .map(|node| node.proof_status)
            .unwrap_or(EwaProofStatus::Unresolved);
        let was_confirmed = scenario.status == EwaScenarioStatus::Confirmed;
        scenario.proof_status = proof_status;
        scenario.status = if proof_status == EwaProofStatus::Proven {
            EwaScenarioStatus::Confirmed
        } else {
            EwaScenarioStatus::Provisional
        };
        if was_confirmed && scenario.status == EwaScenarioStatus::Provisional {
            scenario.confidence *= provisional_multiplier;
        }
    }
    scenarios.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let total = scenarios
        .iter()
        .map(|scenario| scenario.confidence.max(0.0))
        .sum::<f64>();
    let fallback_probability = 1.0 / scenarios.len().max(1) as f64;
    for (index, scenario) in scenarios.iter_mut().enumerate() {
        scenario.rank = index + 1;
        scenario.probability = if total > f64::EPSILON {
            scenario.confidence.max(0.0) / total
        } else {
            fallback_probability
        };
    }
}

pub fn enrich_candidate_for_world(
    candidate: &mut EwaCandidate,
    world: &EwaWorldAnalysis,
) {
    candidate.source_world = world.label.clone();
    enrich_candidate_confluence(
        candidate,
        &world.fib_relations,
        &world.segments,
        &world.pivots,
    );
}

pub fn refresh_analysis_derived(analysis: &mut EwaAnalysis, config: &EwaConfig) {
    let _ = refresh_analysis_derived_profiled(analysis, config);
}

pub fn refresh_analysis_derived_profiled(
    analysis: &mut EwaAnalysis,
    config: &EwaConfig,
) -> EwaRefreshTimings {
    let total_started = Instant::now();
    let mut timings = EwaRefreshTimings::default();

    let started = Instant::now();
    analysis.subdivision_matches = validate_subdivision_scores(&mut analysis.worlds);
    timings.subdivision_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    analysis.scenario_groups =
        rank_world_candidate_groups(&analysis.worlds, config);
    analysis.scenarios = consolidate_scenarios(&analysis.scenario_groups, config);
    timings.ranking_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    analysis.count_nodes = build_count_nodes(
        &analysis.worlds,
        &analysis.scenarios,
        &analysis.subdivision_matches,
    );
    timings.count_tree_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    apply_count_proof_to_scenarios(
        &mut analysis.scenarios,
        &analysis.count_nodes,
        config.provisional_confidence_multiplier,
    );
    timings.proof_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    analysis.nesting_relations =
        build_nesting_relations(&analysis.worlds, &analysis.scenarios, 64);
    timings.nesting_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    analysis.reinterpretations =
        build_reinterpretations(&analysis.worlds, &analysis.scenarios, 8);
    timings.reinterpretation_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    analysis.swing_coverage = build_swing_coverage(&analysis.worlds);
    timings.swing_coverage_ms = started.elapsed().as_secs_f64() * 1000.0;
    timings.total_ms = total_started.elapsed().as_secs_f64() * 1000.0;
    timings
}

pub fn refresh_analysis_derived_affected(
    analysis: &mut EwaAnalysis,
    config: &EwaConfig,
    affected: &[EwaAffectedRange],
) {
    let _ = refresh_analysis_derived_affected_profiled(analysis, config, affected);
}

pub fn refresh_analysis_derived_affected_profiled(
    analysis: &mut EwaAnalysis,
    config: &EwaConfig,
    affected: &[EwaAffectedRange],
) -> EwaRefreshTimings {
    let Some(affected_bar_start) = affected.iter().map(|range| range.bar_start).min() else {
        return EwaRefreshTimings::default();
    };
    let total_started = Instant::now();
    let mut timings = EwaRefreshTimings::default();

    let started = Instant::now();
    analysis.subdivision_matches = validate_subdivision_scores_affected(
        &mut analysis.worlds,
        &analysis.subdivision_matches,
        affected_bar_start,
    );
    timings.subdivision_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    analysis
        .scenario_groups
        .retain(|group| group.bar_end < affected_bar_start);
    analysis.scenario_groups.extend(rank_world_candidate_groups_affected(
        &analysis.worlds,
        config,
        affected_bar_start,
    ));
    analysis.scenario_groups.sort_by(|left, right| {
        right
            .bar_end
            .cmp(&left.bar_end)
            .then_with(|| {
                (right.bar_end - right.bar_start)
                    .cmp(&(left.bar_end - left.bar_start))
            })
    });
    normalize_scenario_group_ids(&mut analysis.scenario_groups);
    analysis.scenarios = consolidate_scenarios(&analysis.scenario_groups, config);
    timings.ranking_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    analysis.count_nodes = build_count_nodes(
        &analysis.worlds,
        &analysis.scenarios,
        &analysis.subdivision_matches,
    );
    timings.count_tree_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    apply_count_proof_to_scenarios(
        &mut analysis.scenarios,
        &analysis.count_nodes,
        config.provisional_confidence_multiplier,
    );
    timings.proof_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    analysis.nesting_relations =
        build_nesting_relations(&analysis.worlds, &analysis.scenarios, 64);
    timings.nesting_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    analysis.reinterpretations =
        build_reinterpretations(&analysis.worlds, &analysis.scenarios, 8);
    timings.reinterpretation_ms = started.elapsed().as_secs_f64() * 1000.0;

    let started = Instant::now();
    analysis
        .swing_coverage
        .retain(|swing| swing.bar_end < affected_bar_start);
    analysis
        .swing_coverage
        .extend(build_swing_coverage_affected(&analysis.worlds, affected_bar_start));
    analysis.swing_coverage.sort_by(|left, right| {
        left.world
            .cmp(&right.world)
            .then_with(|| left.segment_index.cmp(&right.segment_index))
    });
    timings.swing_coverage_ms = started.elapsed().as_secs_f64() * 1000.0;
    timings.total_ms = total_started.elapsed().as_secs_f64() * 1000.0;
    timings
}

pub fn build_swing_coverage(worlds: &[EwaWorldAnalysis]) -> Vec<EwaSwingCoverage> {
    build_swing_coverage_from(worlds, None)
}

fn build_swing_coverage_affected(
    worlds: &[EwaWorldAnalysis],
    affected_bar_start: usize,
) -> Vec<EwaSwingCoverage> {
    build_swing_coverage_from(worlds, Some(affected_bar_start))
}

fn build_swing_coverage_from(
    worlds: &[EwaWorldAnalysis],
    affected_bar_start: Option<usize>,
) -> Vec<EwaSwingCoverage> {
    worlds
        .par_iter()
        .map(|world| {
        let mut candidates_by_segment = vec![Vec::<usize>::new(); world.segments.len()];
        for (candidate_index, candidate) in world.candidates.iter().enumerate() {
            for &segment_index in &candidate.segment_indices {
                if let Some(indices) = candidates_by_segment.get_mut(segment_index) {
                    indices.push(candidate_index);
                }
            }
        }
        let mut fib_by_segment = vec![Vec::<&EwaFibRelation>::new(); world.segments.len()];
        for relation in &world.fib_relations {
            if let Some(relations) = fib_by_segment.get_mut(relation.previous_segment) {
                relations.push(relation);
            }
            if relation.current_segment != relation.previous_segment {
                if let Some(relations) = fib_by_segment.get_mut(relation.current_segment) {
                    relations.push(relation);
                }
            }
        }
        world
            .segments
            .par_iter()
            .enumerate()
            .filter_map(|(segment_index, segment)| {
            if affected_bar_start
                .and_then(|bar_start| {
                    world
                        .pivots
                        .get(segment.end_pivot)
                        .map(|pivot| pivot.index < bar_start)
                })
                .unwrap_or(false)
            {
                return None;
            }
            let mut by_pattern = HashMap::<EwaPatternKind, EwaSwingHypothesis>::new();
            let mut failure_counts = HashMap::<String, usize>::new();
            let mut candidate_count = 0usize;
            let mut strict_candidate_count = 0usize;
            for &candidate_index in &candidates_by_segment[segment_index] {
                let candidate = &world.candidates[candidate_index];
                candidate_count += 1;
                let strict = candidate.is_rule_valid() && candidate.score > 0.0;
                if strict {
                    strict_candidate_count += 1;
                } else {
                    for violation in &candidate.hard_violations {
                        *failure_counts
                            .entry(classify_failure(violation).to_string())
                            .or_default() += 1;
                    }
                }
                let confidence = diagnostic_candidate_confidence(candidate, strict);
                let hypothesis = EwaSwingHypothesis {
                    pattern: candidate.pattern,
                    confidence,
                    source: if strict {
                        EwaHypothesisSource::StrictCandidate
                    } else {
                        EwaHypothesisSource::NearMatch
                    },
                    candidate_index: Some(candidate_index),
                    fib_score: candidate.fib_matrix_score,
                    geometry_score: candidate.geometry_score,
                    technical_score: candidate.technical_features.aggregate(),
                    reasons: if strict {
                        Vec::new()
                    } else if candidate.hard_violations.is_empty() {
                        vec!["candidate:zero_score".to_string()]
                    } else {
                        candidate.hard_violations.iter().take(4).cloned().collect()
                    },
                };
                let replace = by_pattern
                    .get(&candidate.pattern)
                    .map(|current| hypothesis.confidence > current.confidence)
                    .unwrap_or(true);
                if replace {
                    by_pattern.insert(candidate.pattern, hypothesis);
                }
            }

            let fib_relations = &fib_by_segment[segment_index];
            if fib_relations.is_empty() {
                failure_counts.insert("fib:no_relation".to_string(), 1);
            }
            if candidate_count == 0 {
                failure_counts.insert("candidate:no_window".to_string(), 1);
            }
            if strict_candidate_count == 0 {
                failure_counts.insert("strict:no_match".to_string(), 1);
            }

            let mut hypotheses = by_pattern.into_values().collect::<Vec<_>>();
            hypotheses.sort_by(|left, right| {
                right
                    .confidence
                    .partial_cmp(&left.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            hypotheses.truncate(4);
            fill_baseline_hypotheses(
                &mut hypotheses,
                segment,
                fib_relations,
                strict_candidate_count,
            );
            hypotheses.sort_by(|left, right| {
                right
                    .confidence
                    .partial_cmp(&left.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let Some(start) = world.pivots.get(segment.start_pivot) else {
                return None;
            };
            let Some(end) = world.pivots.get(segment.end_pivot) else {
                return None;
            };
            let mut failures = failure_counts
                .into_iter()
                .map(|(code, count)| EwaSwingFailure { code, count })
                .collect::<Vec<_>>();
            failures.sort_by(|left, right| {
                right.count.cmp(&left.count).then_with(|| left.code.cmp(&right.code))
            });
            Some(EwaSwingCoverage {
                world: world.label.clone(),
                segment_index,
                start_pivot: segment.start_pivot,
                end_pivot: segment.end_pivot,
                bar_start: start.index.min(end.index),
                bar_end: start.index.max(end.index),
                direction: segment.direction,
                candidate_count,
                strict_candidate_count,
                hypotheses,
                failures,
            })
        })
        .collect::<Vec<_>>()
    })
    .collect::<Vec<_>>()
    .into_iter()
    .flatten()
    .collect()
}

pub(crate) fn diagnostic_candidate_confidence(candidate: &EwaCandidate, strict: bool) -> f64 {
    if strict {
        return candidate.score.clamp(0.001, 1.0);
    }
    let ratio_score = if candidate.ratios.is_empty() {
        0.15
    } else {
        candidate
            .ratios
            .iter()
            .map(|ratio| (1.0 - ratio.error).clamp(0.0, 1.0))
            .sum::<f64>()
            / candidate.ratios.len() as f64
    };
    let evidence = 0.35 * ratio_score
        + 0.25 * candidate.fib_matrix_score
        + 0.20 * candidate.geometry_score
        + 0.20 * candidate.technical_features.aggregate();
    (0.01 + 0.49 * evidence / (1.0 + candidate.hard_violations.len() as f64))
        .clamp(0.001, 0.49)
}

fn fill_baseline_hypotheses(
    hypotheses: &mut Vec<EwaSwingHypothesis>,
    segment: &EwaSegment,
    fib_relations: &[&EwaFibRelation],
    strict_candidate_count: usize,
) {
    let fib_score = if fib_relations.is_empty() {
        0.0
    } else {
        fib_relations
            .iter()
            .map(|relation| (1.0 - relation.error).clamp(0.0, 1.0))
            .sum::<f64>()
            / fib_relations.len() as f64
    };
    let slope_strength =
        (segment.slope_per_bar.abs() / segment.abs_price_delta.max(f64::EPSILON)).clamp(0.0, 1.0);
    let baseline = [
        (EwaPatternKind::Impulse, 0.035 + 0.045 * slope_strength),
        (EwaPatternKind::Zigzag, 0.030 + 0.040 * fib_score),
        (EwaPatternKind::Flat, 0.025 + 0.030 * (1.0 - slope_strength)),
        (EwaPatternKind::Triangle, 0.020 + 0.025 * (1.0 - slope_strength)),
        (EwaPatternKind::LeadingDiagonalContracting, 0.015 + 0.020 * fib_score),
        (EwaPatternKind::DoubleThree, 0.010 + 0.015 * fib_score),
    ];
    for (pattern, confidence) in baseline {
        if hypotheses.len() >= 4 {
            break;
        }
        if hypotheses.iter().any(|item| item.pattern == pattern) {
            continue;
        }
        hypotheses.push(EwaSwingHypothesis {
            pattern,
            confidence: confidence.clamp(0.001, 0.10),
            source: EwaHypothesisSource::Baseline,
            candidate_index: None,
            fib_score,
            geometry_score: slope_strength,
            technical_score: 0.0,
            reasons: vec![if strict_candidate_count == 0 {
                "baseline:no_strict_pattern_for_swing".to_string()
            } else {
                "baseline:fill_distinct_top4".to_string()
            }],
        });
    }
}

fn classify_failure(violation: &str) -> &'static str {
    let violation = violation.to_ascii_lowercase();
    if violation.contains("ratio")
        || violation.contains("retrac")
        || violation.contains("extension")
        || violation.contains("percent")
    {
        "rule:fibonacci"
    } else if violation.contains("overlap") {
        "rule:overlap"
    } else if violation.contains("direction")
        || violation.contains("reverse")
        || violation.contains("progress")
    {
        "rule:direction"
    } else if violation.contains("grammar") || violation.contains("slot") {
        "rule:grammar"
    } else if violation.contains("triangle") {
        "rule:triangle"
    } else {
        "rule:structural"
    }
}

#[derive(Debug, Clone)]
struct SubdivisionEvidence {
    world_index: usize,
    candidate_index: usize,
    world_id: WorldId,
    candidate_id: CandidateId,
    pattern: EwaPatternKind,
    score: f64,
    bar_start: usize,
    bar_end: usize,
    start_price: f64,
    end_price: f64,
    granularity_bars: f64,
    technical_features: super::types::EwaTechnicalFeatures,
    world: String,
}

struct SubdivisionEvidenceIndex {
    evidence: Vec<SubdivisionEvidence>,
    by_class_direction: [Vec<usize>; 12],
}

impl SubdivisionEvidenceIndex {
    fn new(mut evidence: Vec<SubdivisionEvidence>) -> Self {
        evidence.sort_by_key(|item| item.bar_start);
        let mut by_class_direction: [Vec<usize>; 12] = std::array::from_fn(|_| Vec::new());
        let classes = [
            EwaGrammarClass::Motive,
            EwaGrammarClass::MotiveOrCorrective,
            EwaGrammarClass::Corrective,
            EwaGrammarClass::SimpleCorrection,
            EwaGrammarClass::Zigzag,
            EwaGrammarClass::Triangle,
        ];
        for (index, item) in evidence.iter().enumerate() {
            let direction = usize::from(item.end_price >= item.start_price);
            for (class_index, class) in classes.iter().enumerate() {
                if matches_class(item.pattern, *class) {
                    by_class_direction[class_index * 2 + direction].push(index);
                }
            }
        }
        Self {
            evidence,
            by_class_direction,
        }
    }

    fn candidates(
        &self,
        class: EwaGrammarClass,
        direction: f64,
        start_low: usize,
        start_high: usize,
    ) -> impl Iterator<Item = &SubdivisionEvidence> {
        let indexes = &self.by_class_direction[
            subdivision_class_index(class) * 2 + usize::from(direction >= 0.0)
        ];
        let first = indexes.partition_point(|index| self.evidence[*index].bar_start < start_low);
        indexes[first..]
            .iter()
            .take_while(move |index| self.evidence[**index].bar_start <= start_high)
            .map(|index| &self.evidence[*index])
    }
}

fn subdivision_class_index(class: EwaGrammarClass) -> usize {
    match class {
        EwaGrammarClass::Motive => 0,
        EwaGrammarClass::MotiveOrCorrective => 1,
        EwaGrammarClass::Corrective => 2,
        EwaGrammarClass::SimpleCorrection => 3,
        EwaGrammarClass::Zigzag => 4,
        EwaGrammarClass::Triangle => 5,
    }
}

fn validate_subdivision_scores(worlds: &mut [EwaWorldAnalysis]) -> Vec<EwaSubdivisionMatch> {
    validate_subdivision_scores_impl(worlds, None).1
}

fn validate_subdivision_scores_affected(
    worlds: &mut [EwaWorldAnalysis],
    previous_matches: &[EwaSubdivisionMatch],
    affected_bar_start: usize,
) -> Vec<EwaSubdivisionMatch> {
    let (affected_candidates, mut updated) =
        validate_subdivision_scores_impl(worlds, Some(affected_bar_start));
    let mut matches = previous_matches
        .iter()
        .filter(|item| !affected_candidates.contains(&item.parent_candidate_id))
        .cloned()
        .collect::<Vec<_>>();
    matches.append(&mut updated);
    matches.sort_by_key(|item| {
        (
            item.parent_world_id,
            item.parent_candidate_id,
            wave_position_order(item.position),
            item.child_world_id,
            item.child_candidate_id,
        )
    });
    matches
}

fn validate_subdivision_scores_impl(
    worlds: &mut [EwaWorldAnalysis],
    affected_bar_start: Option<usize>,
) -> (HashSet<CandidateId>, Vec<EwaSubdivisionMatch>) {
    let granularities = worlds
        .iter()
        .map(world_granularity_bars)
        .collect::<Vec<_>>();
    let evidence = worlds
        .iter()
        .enumerate()
        .flat_map(|(world_index, world)| {
            let granularity_bars = granularities[world_index];
            world
                .candidates
                .iter()
                .enumerate()
                .filter(|(_, candidate)| candidate.is_rule_valid())
                .filter_map(move |(candidate_index, candidate)| {
                    let first = *candidate.pivot_indices.first()?;
                    let last = *candidate.pivot_indices.last()?;
                    let bar_start = world.pivots.get(first)?.index;
                    let bar_end = world.pivots.get(last)?.index;
                    let start_price = world.pivots.get(first)?.price;
                    let end_price = world.pivots.get(last)?.price;
                    Some(SubdivisionEvidence {
                        world_index,
                        candidate_index,
                        world_id: world.id(),
                        candidate_id: world
                            .candidate_id(candidate_index)
                            .expect("enumerated candidate must have an id"),
                        pattern: candidate.pattern,
                        score: candidate.score,
                        bar_start: bar_start.min(bar_end),
                        bar_end: bar_start.max(bar_end),
                        start_price,
                        end_price,
                        granularity_bars,
                        technical_features: candidate.technical_features.clone(),
                        world: world.label.clone(),
                    })
                })
        })
        .collect::<Vec<_>>();
    let evidence = SubdivisionEvidenceIndex::new(evidence);

    let results = worlds
        .par_iter_mut()
        .enumerate()
        .map(|(world_index, world)| {
            let affected = world
                .candidates
                .iter()
                .enumerate()
                .filter_map(|(candidate_index, candidate)| {
                    let candidate_id = world.candidate_id(candidate_index)?;
                    let (_, bar_end) = candidate_span_in_world(candidate, world)?;
                    if affected_bar_start
                        .map(|bar_start| bar_end >= bar_start)
                        .unwrap_or(true)
                    {
                        Some(candidate_id)
                    } else {
                        None
                    }
                })
                .collect::<HashSet<_>>();
            let results = world
                .candidates
                .par_iter()
                .enumerate()
                .filter_map(|(candidate_index, candidate)| {
                    let candidate_id = world.candidate_id(candidate_index)?;
                    if !affected.contains(&candidate_id) {
                        return None;
                    }
                    Some((
                        candidate_index,
                        subdivision_proof(
                        world_index,
                        candidate_index,
                        candidate,
                        world,
                        &evidence,
                        granularities[world_index],
                        ),
                    ))
                })
                .collect::<Vec<_>>();
            let mut world_matches = Vec::new();
            for (candidate_index, (score, candidate_matches)) in results {
                world.candidates[candidate_index].subdivision_score = score;
                world_matches.extend(candidate_matches);
            }
            (affected, world_matches)
        })
        .collect::<Vec<_>>();
    let mut affected = HashSet::new();
    let mut matches = Vec::new();
    for (world_affected, mut world_matches) in results {
        affected.extend(world_affected);
        matches.append(&mut world_matches);
    }
    (affected, matches)
}

fn candidate_span_in_world(
    candidate: &EwaCandidate,
    world: &EwaWorldAnalysis,
) -> Option<(usize, usize)> {
    let first = world.pivots.get(*candidate.pivot_indices.first()?)?.index;
    let last = world.pivots.get(*candidate.pivot_indices.last()?)?.index;
    Some((first.min(last), first.max(last)))
}

fn subdivision_proof(
    world_index: usize,
    candidate_index: usize,
    candidate: &EwaCandidate,
    world: &EwaWorldAnalysis,
    evidence: &SubdivisionEvidenceIndex,
    parent_granularity: f64,
) -> (f64, Vec<EwaSubdivisionMatch>) {
    if !candidate.children.is_empty() {
        return (1.0, Vec::new());
    }
    let slots = slots_for(candidate.pattern);
    if slots.is_empty() || candidate.pivot_indices.len() != slots.len() + 1 {
        return (0.0, Vec::new());
    }

    let mut score = 0.0;
    let mut matches = Vec::new();
    let mut used = HashSet::new();
    for (pair, slot) in candidate.pivot_indices.windows(2).zip(slots) {
        let Some(first) = world.pivots.get(pair[0]) else {
            return (0.0, Vec::new());
        };
        let Some(last) = world.pivots.get(pair[1]) else {
            return (0.0, Vec::new());
        };
        let slot_start = first.index.min(last.index);
        let slot_end = first.index.max(last.index);
        let slot_price_range = (last.price - first.price).abs().max(f64::EPSILON);
        let slot_direction = (last.price - first.price).signum();
        let tolerance = ((slot_end - slot_start + 1) as f64 * 0.18).ceil() as usize;
        let tolerance = tolerance.clamp(2, 64);
        let start_low = slot_start.saturating_sub(tolerance);
        let start_high = slot_start.saturating_add(tolerance);
        let mut best: Option<(&SubdivisionEvidence, f64)> = None;
        for item in evidence.candidates(slot.class, slot_direction, start_low, start_high) {
            if item.world_index == world_index && item.candidate_index == candidate_index {
                continue;
            }
            if item.world_index == world_index || used.contains(&(item.world_index, item.candidate_index)) {
                continue;
            }
            let granularity_ratio =
                item.granularity_bars / parent_granularity.max(f64::EPSILON);
            if granularity_ratio >= 0.95 {
                continue;
            }
            if item.bar_end.abs_diff(slot_end) > tolerance {
                continue;
            }
            if (item.end_price - item.start_price).signum() != slot_direction {
                continue;
            }
            if !allowed_in_position(item.pattern, slot.position) {
                continue;
            }
            let similarity = span_similarity(
                slot_start,
                slot_end,
                item.bar_start,
                item.bar_end,
            );
            let boundary_similarity = boundary_similarity(
                first.price,
                last.price,
                item.start_price,
                item.end_price,
                slot_price_range,
            );
            if boundary_similarity < 0.35 {
                continue;
            }
            let granularity_score =
                (1.0 - (granularity_ratio - 0.50).abs() / 0.50).clamp(0.0, 1.0);
            let technical_context_score =
                position_technical_context_score(candidate, slot.position, item.pattern, &item.technical_features);
            let weighted = (
                0.30 * item.score
                    + 0.25 * similarity
                    + 0.25 * boundary_similarity
                    + 0.10 * granularity_score
                    + 0.10 * technical_context_score
            ).clamp(0.0, 1.0);
            if best
                .as_ref()
                .map(|(_, best_score)| weighted > *best_score)
                .unwrap_or(true)
            {
                best = Some((item, weighted));
            }
        }
        if let Some((item, weighted)) = best {
            used.insert((item.world_index, item.candidate_index));
            let granularity_ratio =
                item.granularity_bars / parent_granularity.max(f64::EPSILON);
            let boundary_similarity = boundary_similarity(
                first.price,
                last.price,
                item.start_price,
                item.end_price,
                slot_price_range,
            );
            let technical_context_score =
                position_technical_context_score(candidate, slot.position, item.pattern, &item.technical_features);
            score += weighted;
            matches.push(EwaSubdivisionMatch {
                parent_world_id: world.id(),
                parent_candidate_id: world
                    .candidate_id(candidate_index)
                    .expect("enumerated candidate must have an id"),
                parent_world: world.label.clone(),
                position: slot.position,
                expected_class: format!("{:?}", slot.class),
                child_world_id: item.world_id,
                child_candidate_id: item.candidate_id,
                child_world: item.world.clone(),
                child_pattern: item.pattern,
                span_similarity: span_similarity(
                    slot_start,
                    slot_end,
                    item.bar_start,
                    item.bar_end,
                ),
                boundary_similarity,
                granularity_ratio,
                technical_context_score,
                score: weighted,
            });
        }
    }
    (
        (score / slots.len() as f64).clamp(0.0, 1.0),
        matches,
    )
}

fn world_granularity_bars(world: &EwaWorldAnalysis) -> f64 {
    if world.segments.is_empty() {
        return f64::INFINITY;
    }
    let mut bars = world
        .segments
        .iter()
        .map(|segment| segment.bars.max(1))
        .collect::<Vec<_>>();
    bars.sort_unstable();
    bars[bars.len() / 2] as f64
}

fn world_granularity_rank(worlds: &[EwaWorldAnalysis], world_index: usize) -> usize {
    let target = world_granularity_bars(&worlds[world_index]);
    let mut granularities = worlds
        .iter()
        .map(world_granularity_bars)
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    granularities.sort_by(|left, right| {
        left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
    });
    granularities.dedup_by(|left, right| (*left - *right).abs() <= f64::EPSILON);
    granularities
        .iter()
        .position(|value| (*value - target).abs() <= f64::EPSILON)
        .unwrap_or(0)
}

fn boundary_similarity(
    slot_start: f64,
    slot_end: f64,
    child_start: f64,
    child_end: f64,
    scale: f64,
) -> f64 {
    let start_error = (child_start - slot_start).abs() / scale;
    let end_error = (child_end - slot_end).abs() / scale;
    (1.0 - 0.5 * (start_error + end_error)).clamp(0.0, 1.0)
}

fn position_technical_context_score(
    parent: &EwaCandidate,
    position: EwaWavePosition,
    child_pattern: EwaPatternKind,
    child: &super::types::EwaTechnicalFeatures,
) -> f64 {
    let child_base = child.pattern_score;
    let score = match position {
        EwaWavePosition::Wave3 => {
            0.45 * child_base + 0.55 * child.momentum_score
        }
        EwaWavePosition::Wave5 | EwaWavePosition::C => {
            0.45 * child_base
                + 0.25 * child.divergence_score
                + 0.20 * child.channel_score
                + 0.05 * child.throw_over_score
                + 0.05 * child.throw_under_score
        }
        EwaWavePosition::Wave2 | EwaWavePosition::B | EwaWavePosition::X => {
            0.55 * child_base + 0.45 * child.time_proportion_score
        }
        EwaWavePosition::Wave4 => {
            0.40 * child_base
                + 0.35 * parent.technical_features.alternation_score
                + 0.25 * child.time_proportion_score
        }
        EwaWavePosition::D | EwaWavePosition::E => {
            0.45 * child_base
                + 0.35 * child.momentum_score
                + 0.20 * child.time_proportion_score
        }
        EwaWavePosition::Wave1
        | EwaWavePosition::A
        | EwaWavePosition::W
        | EwaWavePosition::Y
        | EwaWavePosition::Z
        | EwaWavePosition::Root => child_base,
    };
    let grammar_bonus = if allowed_in_position(child_pattern, position) {
        0.05
    } else {
        0.0
    };
    (score + grammar_bonus).clamp(0.0, 1.0)
}

fn span_similarity(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> f64 {
    let overlap_start = a_start.max(b_start);
    let overlap_end = a_end.min(b_end);
    if overlap_end <= overlap_start {
        return 0.0;
    }
    let overlap = overlap_end - overlap_start;
    let union = a_end.max(b_end) - a_start.min(b_start);
    if union == 0 {
        1.0
    } else {
        overlap as f64 / union as f64
    }
}

pub fn build_count_nodes(
    worlds: &[EwaWorldAnalysis],
    scenarios: &[EwaScenario],
    subdivision_matches: &[EwaSubdivisionMatch],
) -> Vec<EwaCountNode> {
    let mut matches_by_parent = HashMap::<CandidateId, Vec<&EwaSubdivisionMatch>>::new();
    for item in subdivision_matches {
        matches_by_parent
            .entry(item.parent_candidate_id)
            .or_default()
            .push(item);
    }

    let mut nodes = Vec::new();
    for scenario in scenarios {
        let Some((world_index, candidate_index)) =
            locate_candidate(worlds, scenario.candidate_id)
        else {
            continue;
        };
        let mut path = HashSet::new();
        build_count_node_recursive(
            worlds,
            world_index,
            candidate_index,
            scenario.id,
            EwaWavePosition::Root,
            0,
            None,
            &matches_by_parent,
            &mut path,
            &mut nodes,
        );
    }
    nodes
}

#[allow(clippy::too_many_arguments)]
fn build_count_node_recursive(
    worlds: &[EwaWorldAnalysis],
    world_index: usize,
    candidate_index: usize,
    scenario_id: ScenarioId,
    position: EwaWavePosition,
    degree: usize,
    parent_id: Option<CountNodeId>,
    matches_by_parent: &HashMap<CandidateId, Vec<&EwaSubdivisionMatch>>,
    path: &mut HashSet<CandidateId>,
    nodes: &mut Vec<EwaCountNode>,
) -> Option<usize> {
    let world = worlds.get(world_index)?;
    let candidate = world.candidates.get(candidate_index)?;
    let candidate_id = world.candidate_id(candidate_index)?;
    if !path.insert(candidate_id) {
        return None;
    }
    let first = *candidate.pivot_indices.first()?;
    let last = *candidate.pivot_indices.last()?;
    let first_bar = world.pivots.get(first)?.index;
    let last_bar = world.pivots.get(last)?.index;
    let node_index = nodes.len();
    let node_id = CountNodeId(node_index);
    nodes.push(EwaCountNode {
        id: node_id,
        parent_id,
        scenario_id,
        world_id: world.id(),
        world: world.label.clone(),
        candidate_id,
        pattern: candidate.pattern,
        position,
        degree,
        granularity_rank: world_granularity_rank(worlds, world_index),
        bar_start: first_bar.min(last_bar),
        bar_end: first_bar.max(last_bar),
        proof_status: proof_status(candidate),
        subdivision_score: candidate.subdivision_score,
        context_valid: allowed_in_position(candidate.pattern, position),
        terminal_evidence: candidate.children.is_empty()
            && world_granularity_rank(worlds, world_index) == 0
            && candidate
                .pivot_indices
                .windows(2)
                .all(|pair| pair[0].abs_diff(pair[1]) == 1),
        granularity_bars: world_granularity_bars(world),
        position_context_score: candidate.technical_features.pattern_score,
        required_positions: slots_for(candidate.pattern)
            .iter()
            .map(|slot| slot.position)
            .collect(),
        covered_positions: Vec::new(),
        children: Vec::new(),
    });

    let required_positions = slots_for(candidate.pattern)
        .iter()
        .map(|slot| slot.position)
        .collect::<Vec<_>>();
    let mut covered_positions = Vec::new();
    for child in &candidate.children {
        let Some(child_candidate_index) = world.candidates.iter().position(|candidate| {
            candidate.pattern == child.pattern
                && candidate.pivot_indices == child.pivot_indices
                && candidate.is_rule_valid()
        }) else {
            continue;
        };
        let Some(child_id) = build_count_node_recursive(
            worlds,
            world_index,
            child_candidate_index,
            scenario_id,
            child.position,
            degree + 1,
            Some(node_id),
            matches_by_parent,
            path,
            nodes,
        ) else {
            continue;
        };
        nodes[child_id].position_context_score = position_technical_context_score(
            candidate,
            child.position,
            world.candidates[child_candidate_index].pattern,
            &world.candidates[child_candidate_index].technical_features,
        );
        covered_positions.push(child.position);
        nodes[node_index].children.push(CountNodeId(child_id));
    }

    if let Some(child_matches) = matches_by_parent.get(&candidate_id) {
        let mut ordered = child_matches.clone();
        ordered.sort_by_key(|item| wave_position_order(item.position));
        for child_match in ordered {
            let required_count = required_positions
                .iter()
                .filter(|position| **position == child_match.position)
                .count();
            let covered_count = covered_positions
                .iter()
                .filter(|position| **position == child_match.position)
                .count();
            if covered_count >= required_count {
                continue;
            }
            let Some(child_world_index) = worlds
                .iter()
                .position(|item| item.id() == child_match.child_world_id)
            else {
                continue;
            };
            let Some(child_candidate_index) =
                worlds[child_world_index].candidate_index(child_match.child_candidate_id)
            else {
                continue;
            };
            let Some(child_id) = build_count_node_recursive(
                worlds,
                child_world_index,
                child_candidate_index,
                scenario_id,
                child_match.position,
                degree + 1,
                Some(node_id),
                matches_by_parent,
                path,
                nodes,
            ) else {
                continue;
            };
            let child_candidate =
                &worlds[child_world_index].candidates[child_candidate_index];
            nodes[child_id].position_context_score = position_technical_context_score(
                candidate,
                child_match.position,
                child_candidate.pattern,
                &child_candidate.technical_features,
            );
            covered_positions.push(child_match.position);
            nodes[node_index].children.push(CountNodeId(child_id));
        }
    }

    covered_positions.sort_by_key(|position| wave_position_order(*position));
    nodes[node_index].covered_positions = covered_positions;
    nodes[node_index].proof_status = recursive_node_proof(node_index, nodes);
    path.remove(&candidate_id);
    Some(node_index)
}

fn recursive_node_proof(node_id: usize, nodes: &[EwaCountNode]) -> EwaProofStatus {
    let node = &nodes[node_id];
    if !node.context_valid || node.proof_status == EwaProofStatus::Invalid {
        return EwaProofStatus::Invalid;
    }
    let full_coverage = node.required_positions.iter().all(|position| {
        let required_count = node
            .required_positions
            .iter()
            .filter(|required| *required == position)
            .count();
        let covered_count = node
            .covered_positions
            .iter()
            .filter(|covered| *covered == position)
            .count();
        covered_count >= required_count
    });
    if !full_coverage {
        return if node.covered_positions.is_empty() && node.terminal_evidence {
            EwaProofStatus::Proven
        } else if node.covered_positions.is_empty() {
            EwaProofStatus::Unresolved
        } else {
            EwaProofStatus::Partial
        };
    }
    let child_statuses = node
        .children
        .iter()
        .map(|child_id| nodes[child_id.0].proof_status)
        .collect::<Vec<_>>();
    let technical_context_complete = node
        .children
        .iter()
        .all(|child_id| nodes[child_id.0].position_context_score >= 0.15);
    if child_statuses
        .iter()
        .any(|status| *status == EwaProofStatus::Invalid)
    {
        EwaProofStatus::Invalid
    } else if node.subdivision_score >= 0.8
        && technical_context_complete
        && child_statuses
            .iter()
            .all(|status| *status == EwaProofStatus::Proven)
    {
        EwaProofStatus::Proven
    } else {
        EwaProofStatus::Partial
    }
}

fn locate_candidate(
    worlds: &[EwaWorldAnalysis],
    target: CandidateId,
) -> Option<(usize, usize)> {
    worlds.iter().enumerate().find_map(|(world_index, world)| {
        world
            .candidate_index(target)
            .map(|candidate_index| (world_index, candidate_index))
    })
}

fn proof_status(candidate: &EwaCandidate) -> EwaProofStatus {
    if !candidate.is_rule_valid() {
        EwaProofStatus::Invalid
    } else if candidate.subdivision_score >= 0.8 {
        EwaProofStatus::Proven
    } else if candidate.subdivision_score > 0.0 {
        EwaProofStatus::Partial
    } else {
        EwaProofStatus::Unresolved
    }
}

fn wave_position_order(position: EwaWavePosition) -> usize {
    match position {
        EwaWavePosition::Root => 0,
        EwaWavePosition::Wave1 | EwaWavePosition::A | EwaWavePosition::W => 1,
        EwaWavePosition::Wave2 | EwaWavePosition::B | EwaWavePosition::X => 2,
        EwaWavePosition::Wave3 | EwaWavePosition::C | EwaWavePosition::Y => 3,
        EwaWavePosition::Wave4 | EwaWavePosition::D => 4,
        EwaWavePosition::Wave5 | EwaWavePosition::E | EwaWavePosition::Z => 5,
    }
}

fn enrich_candidate_confluence(
    candidate: &mut EwaCandidate,
    fib_relations: &[EwaFibRelation],
    segments: &[EwaSegment],
    pivots: &[super::types::EwaPivot],
) {
    if candidate.segment_indices.len() < 2 {
        return;
    }

    let mut adjacent_fib_score = 0.0;
    let mut harmonic_hits = 0usize;
    let mut adjacent_relation_count = 0usize;

    for pair in candidate.segment_indices.windows(2) {
        let Some(relation) = find_fib_relation(fib_relations, pair[0], pair[1])
        else {
            continue;
        };

        adjacent_relation_count += 1;
        adjacent_fib_score += (1.0 - relation.error.min(1.0)).max(0.0);
        if is_harmonic_ratio(relation.nearest_target) && relation.error <= 0.08 {
            harmonic_hits += 1;
        }
    }

    let mut matrix_score = 0.0;
    let mut matrix_relation_count = 0usize;
    for (offset, previous) in candidate.segment_indices.iter().enumerate() {
        for current in candidate.segment_indices.iter().skip(offset + 1) {
            let Some(relation) = find_fib_relation(fib_relations, *previous, *current)
            else {
                continue;
            };

            matrix_relation_count += 1;
            matrix_score += (1.0 - relation.error.min(1.0)).max(0.0);
        }
    }

    if adjacent_relation_count > 0 {
        candidate.fib_confluence = adjacent_fib_score / adjacent_relation_count as f64;
        candidate.harmonic_confluence = harmonic_hits as f64 / adjacent_relation_count as f64;
    }
    if matrix_relation_count > 0 {
        candidate.fib_matrix_score = matrix_score / matrix_relation_count as f64;
    }
    candidate.geometry_score = geometry_score(candidate, segments);
    candidate.technical_features = technical_features(candidate, segments, pivots);
    candidate.prior_probability = prior_for(candidate.pattern).base_probability;

    if !candidate.is_rule_valid() {
        candidate.score = 0.0;
        return;
    }

    let confluence_bonus = 0.08 * candidate.fib_confluence
        + 0.08 * candidate.fib_matrix_score
        + 0.04 * candidate.harmonic_confluence
        + 0.06 * candidate.geometry_score
        + 0.06 * candidate.technical_features.aggregate();
    candidate.score = (candidate.score + confluence_bonus).clamp(0.0, 1.0);
}

fn find_fib_relation(
    fib_relations: &[EwaFibRelation],
    previous_segment: usize,
    current_segment: usize,
) -> Option<&EwaFibRelation> {
    let key = (previous_segment, current_segment);
    let index = fib_relations.partition_point(|relation| {
        (relation.previous_segment, relation.current_segment) < key
    });
    fib_relations.get(index).filter(|relation| {
        relation.previous_segment == previous_segment
            && relation.current_segment == current_segment
    })
}

fn technical_features(
    candidate: &EwaCandidate,
    segments: &[EwaSegment],
    pivots: &[super::types::EwaPivot],
) -> super::types::EwaTechnicalFeatures {
    let candidate_segments = candidate
        .segment_indices
        .iter()
        .filter_map(|&index| segments.get(index))
        .collect::<Vec<_>>();
    if candidate_segments.is_empty() {
        return Default::default();
    }

    let time_proportion_score = adjacent_proportion_score(
        &candidate_segments
            .iter()
            .map(|segment| segment.bars.max(1) as f64)
            .collect::<Vec<_>>(),
    );
    let momentum = candidate_segments
        .iter()
        .map(|segment| {
            segment.abs_price_delta / (segment.bars.max(1) as f64).sqrt()
        })
        .collect::<Vec<_>>();
    let is_motive = matches!(
        candidate.pattern,
        EwaPatternKind::Impulse
            | EwaPatternKind::ImpulseExtendedWave1
            | EwaPatternKind::ImpulseExtendedWave3
            | EwaPatternKind::ImpulseExtendedWave5
            | EwaPatternKind::TruncatedImpulse
            | EwaPatternKind::LeadingDiagonalContracting
            | EwaPatternKind::LeadingDiagonalExpanding
            | EwaPatternKind::EndingDiagonalContracting
            | EwaPatternKind::EndingDiagonalExpanding
    );
    let is_triangle = matches!(
        candidate.pattern,
        EwaPatternKind::Triangle
            | EwaPatternKind::ContractingTriangle
            | EwaPatternKind::BarrierTriangle
            | EwaPatternKind::ExpandingTriangle
            | EwaPatternKind::RunningTriangle
    );
    let momentum_score = if is_motive && momentum.len() >= 5 {
        let reference = momentum[0].max(momentum[4]).max(f64::EPSILON);
        (momentum[2] / reference).min(1.0)
    } else if is_triangle && momentum.len() >= 5 {
        triangle_momentum_score(candidate.pattern, &momentum)
    } else {
        adjacent_proportion_score(&momentum)
    };
    let divergence_score = if is_motive && momentum.len() >= 5 && momentum[2] > f64::EPSILON {
        (1.0 - momentum[4] / momentum[2]).clamp(0.0, 1.0)
    } else if is_triangle && momentum.len() >= 5 && momentum[0] > f64::EPSILON {
        (1.0 - momentum[4] / momentum[0]).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let alternation_score = if is_motive && candidate_segments.len() >= 5 {
        let wave2 = candidate_segments[1].abs_price_delta
            / candidate_segments[0].abs_price_delta.max(f64::EPSILON);
        let wave4 = candidate_segments[3].abs_price_delta
            / candidate_segments[2].abs_price_delta.max(f64::EPSILON);
        let price_difference = (wave2 - wave4).abs() / wave2.max(wave4).max(f64::EPSILON);
        let time2 = candidate_segments[1].bars.max(1) as f64;
        let time4 = candidate_segments[3].bars.max(1) as f64;
        let time_difference = (time2 - time4).abs() / time2.max(time4);
        (0.65 * price_difference + 0.35 * time_difference).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let channel_score = channel_score(candidate, pivots);
    let (throw_over_score, throw_under_score) = if matches!(
        candidate.pattern,
        EwaPatternKind::LeadingDiagonalContracting
            | EwaPatternKind::LeadingDiagonalExpanding
            | EwaPatternKind::EndingDiagonalContracting
            | EwaPatternKind::EndingDiagonalExpanding
    ) {
        diagonal_throw_scores(candidate, pivots)
    } else {
        (0.0, 0.0)
    };

    let mut features = super::types::EwaTechnicalFeatures {
        alternation_score,
        channel_score,
        throw_over_score,
        throw_under_score,
        time_proportion_score,
        momentum_score,
        divergence_score,
        pattern_score: 0.0,
    };
    features.pattern_score = pattern_technical_score(candidate.pattern, &features);
    features
}

fn pattern_technical_score(
    pattern: EwaPatternKind,
    features: &super::types::EwaTechnicalFeatures,
) -> f64 {
    let score = match pattern {
        EwaPatternKind::Impulse
        | EwaPatternKind::ImpulseExtendedWave1
        | EwaPatternKind::ImpulseExtendedWave3
        | EwaPatternKind::ImpulseExtendedWave5
        | EwaPatternKind::TruncatedImpulse => {
            0.25 * features.alternation_score
                + 0.25 * features.channel_score
                + 0.25 * features.momentum_score
                + 0.15 * features.divergence_score
                + 0.10 * features.time_proportion_score
        }
        EwaPatternKind::LeadingDiagonalContracting
        | EwaPatternKind::LeadingDiagonalExpanding
        | EwaPatternKind::EndingDiagonalContracting
        | EwaPatternKind::EndingDiagonalExpanding => {
            0.30 * features.channel_score
                + 0.20 * features.throw_over_score
                + 0.10 * features.throw_under_score
                + 0.20 * features.momentum_score
                + 0.10 * features.divergence_score
                + 0.10 * features.time_proportion_score
        }
        EwaPatternKind::Triangle
        | EwaPatternKind::ContractingTriangle
        | EwaPatternKind::BarrierTriangle
        | EwaPatternKind::ExpandingTriangle
        | EwaPatternKind::RunningTriangle => {
            0.50 * features.momentum_score
                + 0.30 * features.time_proportion_score
                + 0.20 * features.divergence_score
        }
        EwaPatternKind::DoubleZigzag
        | EwaPatternKind::TripleZigzag
        | EwaPatternKind::DoubleCombo
        | EwaPatternKind::DoubleThree
        | EwaPatternKind::TripleCombo
        | EwaPatternKind::TripleThree => {
            0.55 * features.time_proportion_score
                + 0.30 * features.alternation_score
                + 0.15 * features.momentum_score
        }
        EwaPatternKind::Zigzag | EwaPatternKind::RunningZigzag => {
            0.60 * features.momentum_score + 0.40 * features.time_proportion_score
        }
        EwaPatternKind::Flat
        | EwaPatternKind::RegularFlat
        | EwaPatternKind::ExpandedFlat
        | EwaPatternKind::RunningFlat
        | EwaPatternKind::Correction => {
            0.50 * features.time_proportion_score
                + 0.30 * features.momentum_score
                + 0.20 * features.divergence_score
        }
        _ => {
            0.55 * features.time_proportion_score + 0.45 * features.momentum_score
        }
    };
    score.clamp(0.0, 1.0)
}

fn triangle_momentum_score(pattern: EwaPatternKind, momentum: &[f64]) -> f64 {
    let actionary = [momentum[0], momentum[2], momentum[4]];
    let expanding = pattern == EwaPatternKind::ExpandingTriangle;
    let ordered = actionary.windows(2).filter(|pair| {
        if expanding {
            pair[1] >= pair[0]
        } else {
            pair[1] <= pair[0]
        }
    }).count() as f64 / 2.0;
    let proportional = adjacent_proportion_score(&actionary);
    (0.75 * ordered + 0.25 * proportional).clamp(0.0, 1.0)
}

fn adjacent_proportion_score(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    values
        .windows(2)
        .map(|pair| pair[0].min(pair[1]) / pair[0].max(pair[1]).max(f64::EPSILON))
        .sum::<f64>()
        / values.len().saturating_sub(1) as f64
}

fn channel_score(candidate: &EwaCandidate, pivots: &[super::types::EwaPivot]) -> f64 {
    if candidate.pivot_indices.len() < 6 {
        return 0.0;
    }
    let points = candidate
        .pivot_indices
        .iter()
        .take(6)
        .filter_map(|&index| pivots.get(index))
        .collect::<Vec<_>>();
    if points.len() < 6 {
        return 0.0;
    }
    let range = (points[3].price - points[0].price)
        .abs()
        .max(f64::EPSILON);
    let wave5_line = line_value(
        points[1].index,
        points[1].price,
        points[3].index,
        points[3].price,
        points[5].index,
    );
    let parallel_wave4 = points[2].price
        + (wave5_line - points[1].price)
            * (points[4].index.saturating_sub(points[2].index) as f64
                / points[5].index.saturating_sub(points[1].index).max(1) as f64);
    let wave5_error = (points[5].price - wave5_line).abs() / range;
    let wave4_error = (points[4].price - parallel_wave4).abs() / range;
    (1.0 - 0.5 * (wave5_error + wave4_error)).clamp(0.0, 1.0)
}

fn diagonal_throw_scores(
    candidate: &EwaCandidate,
    pivots: &[super::types::EwaPivot],
) -> (f64, f64) {
    if candidate.pivot_indices.len() < 6 {
        return (0.0, 0.0);
    }
    let p1 = &pivots[candidate.pivot_indices[1]];
    let p3 = &pivots[candidate.pivot_indices[3]];
    let p5 = &pivots[candidate.pivot_indices[5]];
    let boundary = line_value(p1.index, p1.price, p3.index, p3.price, p5.index);
    let scale = (p3.price - p1.price).abs().max(f64::EPSILON);
    let overshoot = match segments_direction(p1.price, p3.price) {
        super::types::EwaSegmentDirection::Up => p5.price - boundary,
        super::types::EwaSegmentDirection::Down => boundary - p5.price,
    };
    if overshoot > 0.0 {
        (
            (1.0 - (overshoot / scale - 0.10).abs() / 0.25).clamp(0.0, 1.0),
            0.0,
        )
    } else {
        (
            0.0,
            (1.0 - ((-overshoot) / scale - 0.10).abs() / 0.25).clamp(0.0, 1.0),
        )
    }
}

fn line_value(x1: usize, y1: f64, x2: usize, y2: f64, x: usize) -> f64 {
    let dx = x2.saturating_sub(x1).max(1) as f64;
    y1 + (y2 - y1) / dx * x.saturating_sub(x1) as f64
}

fn segments_direction(start: f64, end: f64) -> super::types::EwaSegmentDirection {
    super::types::EwaSegmentDirection::from_delta(end - start)
}

fn is_harmonic_ratio(target: f64) -> bool {
    matches!(
        (target * 1000.0).round() as i64,
        382 | 500 | 618 | 786 | 886 | 1000 | 1272 | 1414 | 1618 | 2000 | 2618
    )
}

fn geometry_score(candidate: &EwaCandidate, segments: &[EwaSegment]) -> f64 {
    let candidate_segments: Vec<&EwaSegment> = candidate
        .segment_indices
        .iter()
        .filter_map(|&idx| segments.get(idx))
        .collect();
    if candidate_segments.len() < 2 {
        return 0.0;
    }

    match candidate.pattern {
        EwaPatternKind::Impulse
        | EwaPatternKind::ImpulseExtendedWave1
        | EwaPatternKind::ImpulseExtendedWave3
        | EwaPatternKind::ImpulseExtendedWave5
        | EwaPatternKind::TruncatedImpulse => motive_geometry_score(&candidate_segments),
        EwaPatternKind::LeadingDiagonalContracting
        | EwaPatternKind::EndingDiagonalContracting => diagonal_geometry_score(&candidate_segments, true),
        EwaPatternKind::LeadingDiagonalExpanding
        | EwaPatternKind::EndingDiagonalExpanding => diagonal_geometry_score(&candidate_segments, false),
        EwaPatternKind::Triangle
        | EwaPatternKind::ContractingTriangle
        | EwaPatternKind::BarrierTriangle
        | EwaPatternKind::ExpandingTriangle
        | EwaPatternKind::RunningTriangle
        | EwaPatternKind::Flat
        | EwaPatternKind::RegularFlat
        | EwaPatternKind::ExpandedFlat
        | EwaPatternKind::RunningFlat
        | EwaPatternKind::DoubleCombo
        | EwaPatternKind::DoubleThree
        | EwaPatternKind::TripleCombo
        | EwaPatternKind::TripleThree => sideways_geometry_score(&candidate_segments),
        _ => trend_leg_geometry_score(&candidate_segments),
    }
}

fn motive_geometry_score(segments: &[&EwaSegment]) -> f64 {
    if segments.len() < 5 {
        return 0.0;
    }
    let actionary = [&segments[0], &segments[2], &segments[4]];
    let corrective = [&segments[1], &segments[3]];
    let actionary_slope = actionary
        .iter()
        .map(|segment| segment.slope_per_bar.abs())
        .sum::<f64>()
        / actionary.len() as f64;
    let corrective_slope = corrective
        .iter()
        .map(|segment| segment.slope_per_bar.abs())
        .sum::<f64>()
        / corrective.len() as f64;
    let slope_dominance = ratio_score(actionary_slope, corrective_slope, 1.618);
    let wave3_strength = ratio_score(segments[2].abs_price_delta, segments[0].abs_price_delta, 1.0);
    (0.6 * slope_dominance + 0.4 * wave3_strength).clamp(0.0, 1.0)
}

fn diagonal_geometry_score(segments: &[&EwaSegment], contracting: bool) -> f64 {
    if segments.len() < 5 {
        return 0.0;
    }
    let a = segments[0].abs_price_delta;
    let c = segments[2].abs_price_delta;
    let e = segments[4].abs_price_delta;
    if contracting {
        ordering_score(a >= c, c >= e)
    } else {
        ordering_score(a <= c, c <= e)
    }
}

fn sideways_geometry_score(segments: &[&EwaSegment]) -> f64 {
    let net = segments.iter().map(|segment| segment.price_delta).sum::<f64>().abs();
    let gross = segments
        .iter()
        .map(|segment| segment.abs_price_delta)
        .sum::<f64>()
        .max(f64::EPSILON);
    (1.0 - (net / gross).min(1.0)).clamp(0.0, 1.0)
}

fn trend_leg_geometry_score(segments: &[&EwaSegment]) -> f64 {
    let same_direction_pairs = segments
        .windows(2)
        .filter(|pair| pair[0].direction == pair[1].direction)
        .count();
    1.0 - (same_direction_pairs as f64 / segments.len().saturating_sub(1).max(1) as f64)
}

fn ratio_score(numerator: f64, denominator: f64, target: f64) -> f64 {
    if denominator.abs() <= f64::EPSILON {
        return 0.0;
    }
    let ratio = (numerator / denominator).abs();
    (1.0 - ((ratio - target).abs() / target.max(f64::EPSILON)).min(1.0)).clamp(0.0, 1.0)
}

fn ordering_score(first: bool, second: bool) -> f64 {
    match (first, second) {
        (true, true) => 1.0,
        (true, false) | (false, true) => 0.5,
        (false, false) => 0.0,
    }
}

pub fn build_nesting_relations(
    worlds: &[EwaWorldAnalysis],
    scenarios: &[EwaScenario],
    per_parent_limit: usize,
) -> Vec<EwaNestingRelation> {
    let mut out = Vec::new();

    for scenario in scenarios {
        let Some((parent_start, parent_end)) = candidate_bar_span(&scenario.candidate, worlds)
        else {
            continue;
        };
        let mut children = Vec::new();

        for world in worlds {
            for (child_idx, child) in world.candidates.iter().enumerate() {
                if !child.is_rule_valid() {
                    continue;
                }
                let Some((child_start, child_end)) = candidate_bar_span(child, worlds) else {
                    continue;
                };
                if child_start <= parent_start || child_end >= parent_end {
                    continue;
                }
                if child_end <= child_start {
                    continue;
                }
                let Some((child_position, child_class)) = matching_parent_slot(
                    &scenario.candidate,
                    worlds,
                    child_start,
                    child_end,
                ) else {
                    continue;
                };
                if !matches_class(child.pattern, child_class)
                    || !allowed_in_position(child.pattern, child_position)
                {
                    continue;
                }

                children.push(EwaNestingRelation {
                    parent_scenario_id: scenario.id,
                    parent_world_id: WorldId::from_label(
                        &scenario.candidate.source_world,
                    ),
                    parent_world: scenario.candidate.source_world.clone(),
                    child_world_id: world.id(),
                    child_world: world.label.clone(),
                    child_candidate_id: world
                        .candidate_id(child_idx)
                        .expect("enumerated candidate must have an id"),
                    child_pattern: child.pattern,
                    child_position,
                    child_score: child.score,
                    parent_bar_start: parent_start,
                    parent_bar_end: parent_end,
                    child_bar_start: child_start,
                    child_bar_end: child_end,
                });
            }
        }

        children.sort_by(|a, b| {
            b.child_score
                .partial_cmp(&a.child_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        children.truncate(per_parent_limit);
        out.extend(children);
    }

    out
}

fn matching_parent_slot(
    parent: &EwaCandidate,
    worlds: &[EwaWorldAnalysis],
    child_start: usize,
    child_end: usize,
) -> Option<(EwaWavePosition, EwaGrammarClass)> {
    let world = worlds
        .iter()
        .find(|world| world.label == parent.source_world)?;
    let grammar_slots = slots_for(parent.pattern);
    if grammar_slots.is_empty() {
        return None;
    }

    let mut best = None;
    if !parent.children.is_empty() && parent.children.len() == grammar_slots.len() {
        for (child, slot) in parent.children.iter().zip(grammar_slots) {
            let first = *child.pivot_indices.first()?;
            let last = *child.pivot_indices.last()?;
            let slot_start = world.pivots.get(first)?.index;
            let slot_end = world.pivots.get(last)?.index;
            consider_slot(
                &mut best,
                slot.position,
                slot.class,
                slot_start,
                slot_end,
                child_start,
                child_end,
            );
        }
    } else if parent.pivot_indices.len() == grammar_slots.len() + 1 {
        for (pair, slot) in parent.pivot_indices.windows(2).zip(grammar_slots) {
            let slot_start = world.pivots.get(pair[0])?.index;
            let slot_end = world.pivots.get(pair[1])?.index;
            consider_slot(
                &mut best,
                slot.position,
                slot.class,
                slot_start,
                slot_end,
                child_start,
                child_end,
            );
        }
    }

    best.map(|(_, position, class)| (position, class))
}

fn consider_slot(
    best: &mut Option<(f64, EwaWavePosition, EwaGrammarClass)>,
    position: EwaWavePosition,
    class: EwaGrammarClass,
    slot_start: usize,
    slot_end: usize,
    child_start: usize,
    child_end: usize,
) {
    let slot_low = slot_start.min(slot_end);
    let slot_high = slot_start.max(slot_end);
    let overlap_start = slot_low.max(child_start);
    let overlap_end = slot_high.min(child_end);
    if overlap_end <= overlap_start {
        return;
    }
    let child_len = child_end.saturating_sub(child_start).max(1) as f64;
    let coverage = (overlap_end - overlap_start) as f64 / child_len;
    if coverage < 0.80 {
        return;
    }
    if best
        .as_ref()
        .map(|(best_coverage, _, _)| coverage > *best_coverage)
        .unwrap_or(true)
    {
        *best = Some((coverage, position, class));
    }
}

fn candidate_bar_span(
    candidate: &EwaCandidate,
    worlds: &[EwaWorldAnalysis],
) -> Option<(usize, usize)> {
    let world = worlds
        .iter()
        .find(|world| world.label == candidate.source_world)?;
    let first = candidate.pivot_indices.first().copied()?;
    let last = candidate.pivot_indices.last().copied()?;
    let start = world.pivots.get(first)?.index;
    let end = world.pivots.get(last)?.index;
    Some((start.min(end), start.max(end)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(i: usize, close: f64) -> Bar {
        Bar::new(i as i64, close, close + 0.75, close - 0.75, close, 1.0)
    }

    #[test]
    fn engine_runs_end_to_end() {
        let closes = [
            100.0, 105.0, 103.0, 111.0, 108.0, 116.0, 112.0, 120.0, 117.0, 124.0,
        ];
        let bars: Vec<_> = closes
            .iter()
            .enumerate()
            .map(|(i, &close)| bar(i, close))
            .collect();

        let engine = EwaEngine::new(EwaConfig::default().with_top_n(3));
        let analysis = engine.analyze(&bars);

        assert!(!analysis.pivots.is_empty());
        assert!(analysis.scenarios.len() <= 3);
        assert_eq!(analysis.swing_coverage.len(), analysis.segments.len());
        assert!(analysis.swing_coverage.iter().all(|swing| {
            swing.hypotheses.len() == 4
                && swing
                    .hypotheses
                    .iter()
                    .all(|hypothesis| hypothesis.confidence > 0.0)
                && swing
                    .hypotheses
                    .iter()
                    .map(|hypothesis| hypothesis.pattern)
                    .collect::<HashSet<_>>()
                    .len()
                    == 4
        }));
        for node in &analysis.count_nodes {
            if node.proof_status == EwaProofStatus::Proven {
                assert!(node.context_valid);
                let fully_covered = node
                    .required_positions
                    .iter()
                    .all(|position| {
                        node.covered_positions
                            .iter()
                            .filter(|covered| *covered == position)
                            .count()
                            >= node
                                .required_positions
                                .iter()
                                .filter(|required| *required == position)
                                .count()
                    });
                assert!(node.terminal_evidence || fully_covered);
                if fully_covered {
                    assert!(
                        node.children
                            .iter()
                            .all(|child| analysis.count_nodes[child.0].proof_status == EwaProofStatus::Proven)
                    );
                }
            }
        }
    }

    #[test]
    fn subdivision_matches_only_use_finer_worlds() {
        let seed = [
            100.0, 103.0, 107.0, 112.0, 108.0, 105.0, 111.0, 119.0, 126.0, 121.0,
            117.0, 124.0, 131.0, 137.0, 133.0, 129.0, 135.0, 141.0, 138.0, 134.0,
        ];
        let bars = (0..120)
            .map(|index| {
                let close = seed[index % seed.len()] + (index / seed.len()) as f64 * 7.0;
                bar(index, close)
            })
            .collect::<Vec<_>>();
        let analysis = EwaEngine::new(EwaConfig::dev_explore()).analyze(&bars);

        assert!(!analysis.subdivision_matches.is_empty());
        assert!(
            analysis
                .subdivision_matches
                .iter()
                .all(|item| item.granularity_ratio < 0.95)
        );
        assert!(
            analysis
                .subdivision_matches
                .iter()
                .all(|item| item.boundary_similarity >= 0.35)
        );
    }
}
