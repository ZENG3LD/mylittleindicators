use std::collections::HashMap;

use crate::core::types::Bar;
use crate::indicators::swing::SwingDetection;

use super::config::{EwaAlgorithmPassConfig, EwaConfig};
use super::engine::{
    diagnostic_candidate_confidence, enrich_candidate_for_world, refresh_analysis_derived,
    refresh_analysis_derived_affected_profiled, refresh_analysis_derived_profiled,
};
use super::ratios::{FIB_MATRIX_MAX_GAP, build_fib_relations_from_previous};
use super::scanner::CpuEwaScanner;
use super::swing::{build_segments, normalize_pivots, pivot_price, resolve_pivot_index};
use super::model::{
    EwaAffectedRange, EwaAnalysis, EwaHypothesisSource, EwaPivot, EwaPivotKind,
    EwaRefreshTimings, EwaWorldAnalysis,
};
use super::types::EwaPatternKind;

#[derive(Debug, Clone)]
pub struct EwaRuntimeHypothesis {
    pub pattern: EwaPatternKind,
    pub confidence: f64,
    pub source: EwaHypothesisSource,
}

#[derive(Debug, Clone)]
pub struct EwaRuntimeWorldUpdate {
    pub world: String,
    pub pivot_count: usize,
    pub segment_count: usize,
    pub changed_from_pivot: usize,
    pub changed_from_segment: usize,
    pub rescanned_candidates: usize,
    pub strict_candidates: usize,
    pub hypotheses: Vec<EwaRuntimeHypothesis>,
    pub affected: EwaAffectedRange,
}

#[derive(Debug, Clone)]
pub struct EwaRuntimeUpdate {
    pub bar_index: usize,
    pub changed: bool,
    pub worlds: Vec<EwaRuntimeWorldUpdate>,
}

#[derive(Debug, Clone)]
struct EwaRuntimeWorld {
    pass: EwaAlgorithmPassConfig,
    detector: SwingDetection,
    search_start: usize,
    analysis: EwaWorldAnalysis,
}

#[derive(Debug, Clone)]
pub struct EwaRuntime {
    config: EwaConfig,
    bars: Vec<Bar>,
    worlds: Vec<EwaRuntimeWorld>,
    scanner: CpuEwaScanner,
    pending_affected: Vec<EwaAffectedRange>,
}

impl EwaRuntime {
    pub fn new(config: EwaConfig) -> Self {
        let config = config.with_cpu_composition_fallback(false);
        let worlds = config
            .analysis_passes()
            .into_iter()
            .map(|pass| EwaRuntimeWorld {
                detector: SwingDetection::new(pass.swing.mode),
                search_start: 0,
                analysis: EwaWorldAnalysis {
                    label: pass.label.clone(),
                    rule_settings: pass.rule_settings.clone(),
                    pivots: Vec::new(),
                    segments: Vec::new(),
                    fib_relations: Vec::new(),
                    candidates: Vec::new(),
                },
                pass,
            })
            .collect();
        Self {
            config,
            bars: Vec::new(),
            worlds,
            scanner: CpuEwaScanner,
            pending_affected: Vec::new(),
        }
    }

    pub fn from_bars(config: EwaConfig, bars: &[Bar]) -> Self {
        let mut runtime = Self::new(config);
        for &bar in bars {
            runtime.push_bar(bar);
        }
        runtime
    }

    pub fn push_bar(&mut self, bar: Bar) -> EwaRuntimeUpdate {
        let bar_index = self.bars.len();
        self.bars.push(bar);
        let mut updates = Vec::new();

        for world in &mut self.worlds {
            let signal: i8 = match world.detector.detect(
                bar.open,
                bar.high,
                bar.low,
                bar.close,
                bar.volume,
            ) {
                Some((_, crate::Direction::Up)) => 1,
                Some((_, crate::Direction::Down)) => -1,
                _ => 0,
            };
            let Some(kind) = EwaPivotKind::from_signal(signal) else {
                continue;
            };
            let pivot_index = resolve_pivot_index(
                world.pass.swing.mode,
                &self.bars,
                world.search_start,
                bar_index,
                kind,
            );
            let pivot_bar = self.bars[pivot_index];
            let pivot = EwaPivot {
                index: pivot_index,
                time: pivot_bar.time,
                price: pivot_price(world.pass.swing.mode, &pivot_bar, kind),
                kind,
                confirmed_index: bar_index,
                confirmed_time: bar.time,
                source_pass: world.pass.label.clone(),
            };
            world.search_start = pivot_index
                .saturating_add(1)
                .min(bar_index.saturating_add(1));

            let previous_len = world.analysis.pivots.len();
            let previous_last = world.analysis.pivots.last().cloned();
            let mut pivots = std::mem::take(&mut world.analysis.pivots);
            pivots.push(pivot);
            world.analysis.pivots = normalize_pivots(pivots);
            if previous_len == world.analysis.pivots.len()
                && same_pivot(previous_last.as_ref(), world.analysis.pivots.last())
            {
                continue;
            }

            let changed_from_pivot = previous_len.saturating_sub(1);
            let changed_from_segment = changed_from_pivot.saturating_sub(1);
            world
                .analysis
                .segments
                .retain(|segment| segment.start_pivot < changed_from_segment);
            let changed_segment_index = world.analysis.segments.len();
            let mut rebuilt_segments = build_segments(
                &world.analysis.pivots[changed_from_segment..],
                world.pass.min_segment_bars,
                world.pass.min_segment_abs_change,
            );
            for segment in &mut rebuilt_segments {
                segment.start_pivot += changed_from_segment;
                segment.end_pivot += changed_from_segment;
            }
            world.analysis.segments.extend(rebuilt_segments);

            let fib_rebuild_start = changed_segment_index.saturating_sub(FIB_MATRIX_MAX_GAP);
            world
                .analysis
                .fib_relations
                .retain(|relation| relation.previous_segment < fib_rebuild_start);
            world.analysis.fib_relations.extend(build_fib_relations_from_previous(
                &world.analysis.segments,
                fib_rebuild_start,
            ));

            const MAX_LEAF_PIVOTS: usize = 6;
            let rescan_start = changed_from_pivot.saturating_sub(MAX_LEAF_PIVOTS - 1);
            let candidate_cutoff = world.analysis.candidates.partition_point(|candidate| {
                candidate
                    .pivot_indices
                    .first()
                    .copied()
                    .map(|start| start < rescan_start)
                    .unwrap_or(false)
            });
            world.analysis.candidates.truncate(candidate_cutoff);
            let mut rescanned = self.scanner.scan_from(
                &world.analysis.pivots,
                &world.analysis.segments,
                &world.analysis.rule_settings,
                rescan_start,
            );
            for candidate in &mut rescanned {
                enrich_candidate_for_world(candidate, &world.analysis);
            }
            rescanned.sort_by(|left, right| {
                left.pivot_indices
                    .first()
                    .cmp(&right.pivot_indices.first())
                    .then_with(|| (left.pattern as usize).cmp(&(right.pattern as usize)))
            });
            let strict_candidates = rescanned
                .iter()
                .filter(|candidate| candidate.is_rule_valid() && candidate.score > 0.0)
                .count();
            let rescanned_candidates = rescanned.len();
            let hypotheses = runtime_hypotheses(&rescanned);
            world.analysis.candidates.extend(rescanned);
            let affected = EwaAffectedRange {
                world_id: world.analysis.id(),
                pivot_start: changed_from_pivot,
                segment_start: changed_segment_index,
                bar_start: world
                    .analysis
                    .pivots
                    .get(changed_from_pivot)
                    .map(|pivot| pivot.index)
                    .unwrap_or(bar_index),
                bar_end: bar_index,
            };
            self.pending_affected.push(affected);

            updates.push(EwaRuntimeWorldUpdate {
                world: world.analysis.label.clone(),
                pivot_count: world.analysis.pivots.len(),
                segment_count: world.analysis.segments.len(),
                changed_from_pivot,
                changed_from_segment: changed_segment_index,
                rescanned_candidates,
                strict_candidates,
                hypotheses,
                affected,
            });
        }

        EwaRuntimeUpdate {
            bar_index,
            changed: !updates.is_empty(),
            worlds: updates,
        }
    }

    pub fn bars(&self) -> &[Bar] {
        &self.bars
    }

    pub fn worlds(&self) -> impl Iterator<Item = &EwaWorldAnalysis> {
        self.worlds.iter().map(|world| &world.analysis)
    }

    pub fn pending_affected(&self) -> &[EwaAffectedRange] {
        &self.pending_affected
    }

    pub fn refresh_snapshot_incremental(
        &mut self,
        analysis: &mut EwaAnalysis,
    ) -> EwaRefreshTimings {
        if self.pending_affected.is_empty() && !analysis.scenario_groups.is_empty() {
            return EwaRefreshTimings::default();
        }
        let previous_subdivision_scores = analysis
            .worlds
            .iter()
            .flat_map(|world| {
                world
                    .candidates
                    .iter()
                    .enumerate()
                    .filter_map(|(candidate_index, candidate)| {
                        Some((
                            world.candidate_id(candidate_index)?,
                            candidate.subdivision_score,
                        ))
                    })
            })
            .collect::<HashMap<_, _>>();
        analysis.worlds = self
            .worlds
            .iter()
            .map(|world| world.analysis.clone())
            .collect();
        for world in &mut analysis.worlds {
            let world_id = world.id();
            for candidate in &mut world.candidates {
                let candidate_id =
                    super::model::CandidateId::new(
                        world_id,
                        candidate.pattern,
                        &candidate.pivot_indices,
                    );
                if let Some(score) = previous_subdivision_scores.get(&candidate_id) {
                    candidate.subdivision_score = *score;
                }
            }
        }
        analysis.pivots = analysis
            .worlds
            .iter()
            .flat_map(|world| world.pivots.iter().cloned())
            .collect();
        analysis.segments = analysis
            .worlds
            .iter()
            .flat_map(|world| world.segments.iter().cloned())
            .collect();
        let timings = if analysis.scenario_groups.is_empty() {
            refresh_analysis_derived_profiled(analysis, &self.config)
        } else {
            refresh_analysis_derived_affected_profiled(
                analysis,
                &self.config,
                &self.pending_affected,
            )
        };
        self.pending_affected.clear();
        timings
    }

    pub fn acknowledge_affected(&mut self) {
        self.pending_affected.clear();
    }

    pub fn snapshot(&self) -> EwaAnalysis {
        analysis_from_worlds(
            self.worlds
                .iter()
                .map(|world| world.analysis.clone())
                .collect(),
            &self.config,
        )
    }

    pub fn into_analysis(self) -> EwaAnalysis {
        analysis_from_worlds(
            self.worlds
                .into_iter()
                .map(|world| world.analysis)
                .collect(),
            &self.config,
        )
    }
}

fn analysis_from_worlds(worlds: Vec<EwaWorldAnalysis>, config: &EwaConfig) -> EwaAnalysis {
    let pivots = worlds
        .iter()
        .flat_map(|world| world.pivots.iter().cloned())
        .collect();
    let segments = worlds
        .iter()
        .flat_map(|world| world.segments.iter().cloned())
        .collect();
    let mut analysis = EwaAnalysis {
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
    };
    refresh_analysis_derived(&mut analysis, config);
    analysis
}

fn same_pivot(left: Option<&EwaPivot>, right: Option<&EwaPivot>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => {
            left.index == right.index
                && left.confirmed_index == right.confirmed_index
                && left.kind == right.kind
                && left.price.to_bits() == right.price.to_bits()
        }
        (None, None) => true,
        _ => false,
    }
}

fn runtime_hypotheses(candidates: &[super::model::EwaCandidate]) -> Vec<EwaRuntimeHypothesis> {
    let mut by_pattern = HashMap::<EwaPatternKind, EwaRuntimeHypothesis>::new();
    for candidate in candidates {
        let strict = candidate.is_rule_valid() && candidate.score > 0.0;
        let hypothesis = EwaRuntimeHypothesis {
            pattern: candidate.pattern,
            confidence: diagnostic_candidate_confidence(candidate, strict),
            source: if strict {
                EwaHypothesisSource::StrictCandidate
            } else {
                EwaHypothesisSource::NearMatch
            },
        };
        if by_pattern
            .get(&candidate.pattern)
            .map(|current| hypothesis.confidence > current.confidence)
            .unwrap_or(true)
        {
            by_pattern.insert(candidate.pattern, hypothesis);
        }
    }
    let mut hypotheses = by_pattern.into_values().collect::<Vec<_>>();
    hypotheses.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| (left.pattern as usize).cmp(&(right.pattern as usize)))
    });
    hypotheses.truncate(4);
    for pattern in [
        EwaPatternKind::Zigzag,
        EwaPatternKind::Flat,
        EwaPatternKind::Triangle,
        EwaPatternKind::Impulse,
    ] {
        if hypotheses.len() >= 4 {
            break;
        }
        if hypotheses.iter().any(|item| item.pattern == pattern) {
            continue;
        }
        hypotheses.push(EwaRuntimeHypothesis {
            pattern,
            confidence: 0.001,
            source: EwaHypothesisSource::Baseline,
        });
    }
    hypotheses
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::ewa::EwaEngine;

    fn bars() -> Vec<Bar> {
        let seed = [
            100.0, 103.0, 107.0, 112.0, 108.0, 105.0, 111.0, 119.0, 126.0, 121.0,
            117.0, 124.0, 131.0, 137.0, 133.0, 129.0,
        ];
        (0..160)
            .map(|index| {
                let close = seed[index % seed.len()] + (index / seed.len()) as f64 * 6.0;
                Bar::new(
                    index as i64 * 60_000,
                    close,
                    close + 0.75,
                    close - 0.75,
                    close,
                    1.0,
                )
            })
            .collect()
    }

    #[test]
    fn incremental_worlds_match_batch_primitives() {
        let config = EwaConfig::dev_explore().with_cpu_composition_fallback(false);
        let bars = bars();
        let batch = EwaEngine::new(config.clone()).analyze_primitives(&bars);
        let mut runtime = EwaRuntime::new(config);
        for &bar in &bars {
            let update = runtime.push_bar(bar);
            assert!(update.worlds.iter().all(|world| {
                world.hypotheses.len() == 4
                    && world
                        .hypotheses
                        .iter()
                        .all(|hypothesis| hypothesis.confidence > 0.0)
                    && world
                        .hypotheses
                        .iter()
                        .map(|hypothesis| hypothesis.pattern)
                        .collect::<HashSet<_>>()
                        .len()
                        == 4
            }));
        }
        let incremental = runtime
            .worlds()
            .map(|world| (world.label.clone(), world))
            .collect::<std::collections::HashMap<_, _>>();

        for batch_world in &batch.worlds {
            let runtime_world = incremental[&batch_world.label];
            assert_eq!(runtime_world.pivots.len(), batch_world.pivots.len());
            assert_eq!(runtime_world.segments.len(), batch_world.segments.len());
            let batch_candidates = batch_world
                .candidates
                .iter()
                .map(|candidate| {
                    (
                        candidate.pattern,
                        candidate.pivot_indices.clone(),
                        candidate.is_rule_valid(),
                    )
                })
                .collect::<HashSet<_>>();
            let runtime_candidates = runtime_world
                .candidates
                .iter()
                .map(|candidate| {
                    (
                        candidate.pattern,
                        candidate.pivot_indices.clone(),
                        candidate.is_rule_valid(),
                    )
                })
                .collect::<HashSet<_>>();
            assert_eq!(runtime_candidates, batch_candidates);
        }
    }

    #[test]
    fn incremental_derived_refresh_matches_batch_semantics() {
        let config = EwaConfig::dev_explore().with_cpu_composition_fallback(false);
        let bars = bars();
        let split = 96;
        let engine = EwaEngine::new(config.clone());
        let mut runtime = EwaRuntime::from_bars(config, &bars[..split]);
        let mut incremental = runtime.snapshot();
        runtime.pending_affected.clear();

        for &bar in &bars[split..] {
            runtime.push_bar(bar);
        }
        assert!(!runtime.pending_affected().is_empty());
        runtime.refresh_snapshot_incremental(&mut incremental);

        let batch = engine.analyze(&bars);
        let incremental = incremental.semantic_snapshot();
        let batch = batch.semantic_snapshot();
        assert_eq!(incremental.worlds, batch.worlds, "world semantics differ");
        assert_eq!(incremental.scenarios, batch.scenarios, "scenario semantics differ");
        assert_eq!(
            incremental.count_nodes,
            batch.count_nodes,
            "count semantics differ"
        );
        assert_eq!(
            incremental.subdivision_edges.len(),
            batch.subdivision_edges.len(),
            "subdivision edge counts differ"
        );
        assert_eq!(
            incremental
                .subdivision_edges
                .iter()
                .zip(&batch.subdivision_edges)
                .position(|(left, right)| left != right),
            None,
            "subdivision edge semantics differ"
        );
    }

}
