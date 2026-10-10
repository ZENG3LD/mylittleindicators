//! Elliott Wave Analysis core.
//!
//! This module is intentionally backend-free: it owns the market-structure
//! pass, candidate model, CPU reference scanner, and ranking glue. Live data,
//! GPU compute backends, and validation harnesses belong in external crates
//! such as `crates/mli-validator`.

pub mod config;
pub mod engine;
pub mod grammar;
pub mod priors;
pub mod probability;
pub mod primitives;
pub mod ranker;
pub mod reinterpretation;
pub mod ratios;
pub mod rules;
pub mod runtime;
pub mod model;
pub mod scanner;
pub mod scan;
pub mod chart_swing;
pub mod swing;
pub mod types;

pub use config::{EwaAlgorithmPassConfig, EwaConfig, EwaExecutionProfile, EwaSwingPassConfig};
pub use engine::{
    EwaEngine, apply_count_proof_to_scenarios, build_count_nodes, build_nesting_relations,
    enrich_candidate_for_world, refresh_analysis_derived, refresh_analysis_derived_affected,
    refresh_analysis_derived_affected_profiled, refresh_analysis_derived_profiled,
};
pub use grammar::{
    EwaGrammarClass, EwaGrammarSlot, EwaWavePosition, allowed_in_position, matches_class,
    is_scenario_pattern, slots_for,
};
pub use priors::{EwaPatternPrior, pattern_priors, prior_for};
pub use probability::EwaProbabilityModel;
pub use primitives::{EwaPrimitiveFamily, EwaPrimitiveSpec, EwaSubdivision, all_ewa_primitives};
pub use ranker::{
    consolidate_scenarios, rank_candidate_group_refs, rank_candidate_groups, rank_candidates,
    rank_world_candidate_groups, rank_world_candidate_groups_affected,
};
pub use reinterpretation::{build_reinterpretations, reinterpretation_matrix};
pub use runtime::{
    EwaRuntime, EwaRuntimeHypothesis, EwaRuntimeUpdate, EwaRuntimeWorldUpdate,
};
pub use scanner::{CpuEwaScanner, EwaScanner, compose_correction_candidate};
pub use swing::EwaSwingExtractor;
pub use model::{
    CandidateId, CountNodeId, EwaAffectedRange, EwaAnalysis, EwaCandidate, EwaChildPattern,
    EwaCountNode, EwaHypothesisSource, EwaNestingRelation, EwaPivot, EwaPivotKind,
    EwaProofStatus, EwaRefreshTimings, EwaReinterpretation, EwaReinterpretationRule,
    EwaRuleSettings, EwaScenario, EwaScenarioGroup, EwaScenarioStatus,
    EwaSemanticCountNode, EwaSemanticScenario, EwaSemanticSnapshot,
    EwaSemanticSubdivisionEdge, EwaSemanticWorld, EwaSwingCoverage, EwaSwingFailure,
    EwaSwingHypothesis, EwaTechnicalFeatures, EwaWorldAnalysis, PivotId, ScenarioId,
    SegmentId, WorldId,
};
pub use types::{EwaPatternKind, EwaRatio, EwaSegment, EwaSegmentDirection};
