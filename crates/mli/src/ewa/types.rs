
use super::grammar::EwaWavePosition;

macro_rules! ewa_id {
    ($name:ident, $inner:ty) => {
        #[derive(
            Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash,
        )]
        pub struct $name(pub $inner);
    };
}

ewa_id!(WorldId, u64);
ewa_id!(PivotId, u64);
ewa_id!(SegmentId, u64);
ewa_id!(CandidateId, u64);
ewa_id!(ScenarioId, u64);
ewa_id!(CountNodeId, usize);

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn stable_hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn stable_hash_u64(hash: u64, value: u64) -> u64 {
    stable_hash_bytes(hash, &value.to_le_bytes())
}

impl WorldId {
    pub fn from_label(label: &str) -> Self {
        Self(stable_hash_bytes(FNV_OFFSET_BASIS, label.as_bytes()))
    }
}

impl PivotId {
    pub fn new(world: WorldId, pivot_index: usize, kind: EwaPivotKind) -> Self {
        let hash = stable_hash_u64(FNV_OFFSET_BASIS, world.0);
        let hash = stable_hash_u64(hash, pivot_index as u64);
        Self(stable_hash_u64(hash, kind.signal() as i64 as u64))
    }
}

impl SegmentId {
    pub fn new(world: WorldId, start_pivot: usize, end_pivot: usize) -> Self {
        let hash = stable_hash_u64(FNV_OFFSET_BASIS, world.0);
        let hash = stable_hash_u64(hash, start_pivot as u64);
        Self(stable_hash_u64(hash, end_pivot as u64))
    }
}

impl CandidateId {
    pub fn new(world: WorldId, pattern: EwaPatternKind, pivot_indices: &[usize]) -> Self {
        let hash = stable_hash_u64(FNV_OFFSET_BASIS, world.0);
        let hash = stable_hash_u64(hash, pattern as u64);
        Self(
            pivot_indices
                .iter()
                .fold(hash, |hash, pivot| stable_hash_u64(hash, *pivot as u64)),
        )
    }

    pub fn from_candidate(world: WorldId, candidate: &EwaCandidate) -> Self {
        let mut hash = Self::new(world, candidate.pattern, &candidate.pivot_indices).0;
        for segment in &candidate.segment_indices {
            hash = stable_hash_u64(hash, *segment as u64);
        }
        for child in &candidate.children {
            hash = stable_hash_u64(hash, child.position as u64);
            hash = stable_hash_u64(hash, child.pattern as u64);
            for pivot in &child.pivot_indices {
                hash = stable_hash_u64(hash, *pivot as u64);
            }
        }
        Self(hash)
    }
}

impl ScenarioId {
    pub fn new(candidate: CandidateId, bar_start: usize, bar_end: usize) -> Self {
        let hash = stable_hash_u64(FNV_OFFSET_BASIS, candidate.0);
        let hash = stable_hash_u64(hash, bar_start as u64);
        Self(stable_hash_u64(hash, bar_end as u64))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EwaAffectedRange {
    pub world_id: WorldId,
    pub pivot_start: usize,
    pub segment_start: usize,
    pub bar_start: usize,
    pub bar_end: usize,
}

impl EwaAffectedRange {
    pub fn intersects(self, bar_start: usize, bar_end: usize) -> bool {
        bar_end >= self.bar_start && bar_start <= self.bar_end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EwaPivotKind {
    High,
    Low,
}

impl EwaPivotKind {
    pub fn from_signal(signal: i8) -> Option<Self> {
        match signal {
            1 => Some(Self::High),
            -1 => Some(Self::Low),
            _ => None,
        }
    }

    pub fn signal(self) -> i8 {
        match self {
            Self::High => 1,
            Self::Low => -1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EwaPivot {
    pub index: usize,
    pub time: i64,
    pub price: f64,
    pub kind: EwaPivotKind,
    pub confirmed_index: usize,
    pub confirmed_time: i64,
    pub source_pass: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EwaSegmentDirection {
    Up,
    Down,
}

impl EwaSegmentDirection {
    pub fn from_delta(delta: f64) -> Self {
        if delta >= 0.0 { Self::Up } else { Self::Down }
    }
}

#[derive(Debug, Clone)]
pub struct EwaSegment {
    pub start_pivot: usize,
    pub end_pivot: usize,
    pub direction: EwaSegmentDirection,
    pub bars: usize,
    pub duration_ms: i64,
    pub price_delta: f64,
    pub abs_price_delta: f64,
    pub slope_per_bar: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EwaFibRelationKind {
    Retracement,
    Extension,
}

#[derive(Debug, Clone)]
pub struct EwaFibRelation {
    pub previous_segment: usize,
    pub current_segment: usize,
    pub kind: EwaFibRelationKind,
    pub ratio: f64,
    pub nearest_target: f64,
    pub error: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EwaPatternKind {
    Impulse,
    ImpulseExtendedWave1,
    ImpulseExtendedWave3,
    ImpulseExtendedWave5,
    TruncatedImpulse,
    LeadingDiagonalContracting,
    LeadingDiagonalExpanding,
    EndingDiagonalContracting,
    EndingDiagonalExpanding,
    Correction,
    Zigzag,
    RunningZigzag,
    DoubleZigzag,
    TripleZigzag,
    Flat,
    RegularFlat,
    ExpandedFlat,
    RunningFlat,
    Triangle,
    ContractingTriangle,
    BarrierTriangle,
    ExpandingTriangle,
    RunningTriangle,
    DoubleCombo,
    DoubleThree,
    TripleCombo,
    TripleThree,
    Xabcd,
    Abcd,
    Cypher,
    Gartley,
    Bat,
    Butterfly,
    Crab,
    Shark,
    ThreeDrives,
}

impl EwaPatternKind {
    pub fn required_pivots(self) -> usize {
        match self {
            Self::Correction
            | Self::Zigzag
            | Self::RunningZigzag
            | Self::Flat
            | Self::RegularFlat
            | Self::ExpandedFlat
            | Self::RunningFlat
            | Self::Abcd => 4,
            Self::Xabcd
            | Self::Cypher
            | Self::Gartley
            | Self::Bat
            | Self::Butterfly
            | Self::Crab
            | Self::Shark => 5,
            Self::Impulse
            | Self::ImpulseExtendedWave1
            | Self::ImpulseExtendedWave3
            | Self::ImpulseExtendedWave5
            | Self::TruncatedImpulse
            | Self::LeadingDiagonalContracting
            | Self::LeadingDiagonalExpanding
            | Self::EndingDiagonalContracting
            | Self::EndingDiagonalExpanding
            | Self::Triangle
            | Self::ContractingTriangle
            | Self::BarrierTriangle
            | Self::ExpandingTriangle
            | Self::RunningTriangle => 6,
            Self::DoubleCombo | Self::DoubleThree | Self::DoubleZigzag => 10,
            Self::TripleCombo | Self::TripleThree | Self::TripleZigzag => 16,
            Self::ThreeDrives => 7,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EwaRuleSettings {
    pub max_candidate_pivots: usize,
    pub ratio_error_scale: f64,
    pub hard_violation_penalty: f64,
    pub compose_corrections_on_cpu: bool,
    pub enabled_patterns: Vec<EwaPatternKind>,
}

impl Default for EwaRuleSettings {
    fn default() -> Self {
        Self {
            max_candidate_pivots: 26,
            ratio_error_scale: 1.0,
            hard_violation_penalty: 0.35,
            compose_corrections_on_cpu: true,
            enabled_patterns: vec![
                EwaPatternKind::Impulse,
                EwaPatternKind::ImpulseExtendedWave1,
                EwaPatternKind::ImpulseExtendedWave3,
                EwaPatternKind::ImpulseExtendedWave5,
                EwaPatternKind::TruncatedImpulse,
                EwaPatternKind::LeadingDiagonalContracting,
                EwaPatternKind::LeadingDiagonalExpanding,
                EwaPatternKind::EndingDiagonalContracting,
                EwaPatternKind::EndingDiagonalExpanding,
                EwaPatternKind::Correction,
                EwaPatternKind::Zigzag,
                EwaPatternKind::RunningZigzag,
                EwaPatternKind::DoubleZigzag,
                EwaPatternKind::TripleZigzag,
                EwaPatternKind::Flat,
                EwaPatternKind::RegularFlat,
                EwaPatternKind::ExpandedFlat,
                EwaPatternKind::RunningFlat,
                EwaPatternKind::Triangle,
                EwaPatternKind::ContractingTriangle,
                EwaPatternKind::BarrierTriangle,
                EwaPatternKind::ExpandingTriangle,
                EwaPatternKind::RunningTriangle,
                EwaPatternKind::DoubleThree,
                EwaPatternKind::TripleThree,
                EwaPatternKind::Xabcd,
                EwaPatternKind::Abcd,
                EwaPatternKind::Cypher,
                EwaPatternKind::Gartley,
                EwaPatternKind::Bat,
                EwaPatternKind::Butterfly,
                EwaPatternKind::Crab,
                EwaPatternKind::Shark,
                EwaPatternKind::ThreeDrives,
            ],
        }
    }
}

impl EwaRuleSettings {
    pub fn is_enabled(&self, pattern: EwaPatternKind) -> bool {
        self.enabled_patterns.contains(&pattern)
    }
}

#[derive(Debug, Clone)]
pub struct EwaRatio {
    pub name: String,
    pub actual: f64,
    pub target: f64,
    pub error: f64,
    pub weight: f64,
}

#[derive(Debug, Clone, Default)]
pub struct EwaTechnicalFeatures {
    pub alternation_score: f64,
    pub channel_score: f64,
    pub throw_over_score: f64,
    pub throw_under_score: f64,
    pub time_proportion_score: f64,
    pub momentum_score: f64,
    pub divergence_score: f64,
    pub pattern_score: f64,
}

impl EwaTechnicalFeatures {
    pub fn aggregate(&self) -> f64 {
        self.pattern_score
    }
}

#[derive(Debug, Clone)]
pub struct EwaCandidate {
    pub source_world: String,
    pub pattern: EwaPatternKind,
    pub pivot_indices: Vec<usize>,
    pub segment_indices: Vec<usize>,
    pub ratios: Vec<EwaRatio>,
    pub children: Vec<EwaChildPattern>,
    pub subdivision_score: f64,
    pub technical_features: EwaTechnicalFeatures,
    pub hard_violations: Vec<String>,
    pub fib_confluence: f64,
    pub fib_matrix_score: f64,
    pub harmonic_confluence: f64,
    pub geometry_score: f64,
    pub prior_probability: f64,
    pub soft_penalty: f64,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct EwaChildPattern {
    pub position: EwaWavePosition,
    pub pattern: EwaPatternKind,
    pub pivot_indices: Vec<usize>,
    pub score: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EwaProofStatus {
    Proven,
    Partial,
    Unresolved,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EwaScenarioStatus {
    Confirmed,
    Provisional,
}

#[derive(Debug, Clone)]
pub struct EwaSubdivisionMatch {
    pub parent_world_id: WorldId,
    pub parent_candidate_id: CandidateId,
    pub parent_world: String,
    pub position: EwaWavePosition,
    pub expected_class: String,
    pub child_world_id: WorldId,
    pub child_candidate_id: CandidateId,
    pub child_world: String,
    pub child_pattern: EwaPatternKind,
    pub span_similarity: f64,
    pub boundary_similarity: f64,
    pub granularity_ratio: f64,
    pub technical_context_score: f64,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct EwaCountNode {
    pub id: CountNodeId,
    pub parent_id: Option<CountNodeId>,
    pub scenario_id: ScenarioId,
    pub world_id: WorldId,
    pub world: String,
    pub candidate_id: CandidateId,
    pub pattern: EwaPatternKind,
    pub position: EwaWavePosition,
    pub degree: usize,
    pub granularity_rank: usize,
    pub bar_start: usize,
    pub bar_end: usize,
    pub proof_status: EwaProofStatus,
    pub subdivision_score: f64,
    pub context_valid: bool,
    pub terminal_evidence: bool,
    pub granularity_bars: f64,
    pub position_context_score: f64,
    pub required_positions: Vec<EwaWavePosition>,
    pub covered_positions: Vec<EwaWavePosition>,
    pub children: Vec<CountNodeId>,
}

impl EwaCandidate {
    pub fn is_rule_valid(&self) -> bool {
        self.hard_violations.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct EwaScenario {
    pub id: ScenarioId,
    pub candidate_id: CandidateId,
    pub rank: usize,
    pub candidate: EwaCandidate,
    pub confidence: f64,
    pub probability: f64,
    pub status: EwaScenarioStatus,
    pub proof_status: EwaProofStatus,
    pub ambiguity_group: usize,
    pub bar_start: usize,
    pub bar_end: usize,
    pub supporting_worlds: usize,
    pub invalidation_price: Option<f64>,
    pub annotation: String,
}

#[derive(Debug, Clone)]
pub struct EwaScenarioGroup {
    pub id: usize,
    pub bar_start: usize,
    pub bar_end: usize,
    pub alternatives: Vec<EwaScenario>,
}

#[derive(Debug, Clone)]
pub struct EwaNestingRelation {
    pub parent_scenario_id: ScenarioId,
    pub parent_world_id: WorldId,
    pub parent_world: String,
    pub child_world_id: WorldId,
    pub child_world: String,
    pub child_candidate_id: CandidateId,
    pub child_pattern: EwaPatternKind,
    pub child_position: EwaWavePosition,
    pub child_score: f64,
    pub parent_bar_start: usize,
    pub parent_bar_end: usize,
    pub child_bar_start: usize,
    pub child_bar_end: usize,
}

#[derive(Debug, Clone)]
pub struct EwaReinterpretationRule {
    pub from: EwaPatternKind,
    pub to: EwaPatternKind,
    pub condition: &'static str,
    pub granularity_effect: &'static str,
    pub required_context: &'static str,
}

#[derive(Debug, Clone)]
pub struct EwaReinterpretation {
    pub source_world: String,
    pub target_world: String,
    pub candidate_index: usize,
    pub from: EwaPatternKind,
    pub to: EwaPatternKind,
    pub reason: String,
    pub granularity_effect: String,
    pub required_context: String,
    pub bar_start: usize,
    pub bar_end: usize,
    pub confidence_hint: f64,
}

#[derive(Debug, Clone)]
pub struct EwaWorldAnalysis {
    pub label: String,
    pub rule_settings: EwaRuleSettings,
    pub pivots: Vec<EwaPivot>,
    pub segments: Vec<EwaSegment>,
    pub fib_relations: Vec<EwaFibRelation>,
    pub candidates: Vec<EwaCandidate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EwaHypothesisSource {
    StrictCandidate,
    NearMatch,
    Baseline,
}

#[derive(Debug, Clone)]
pub struct EwaSwingHypothesis {
    pub pattern: EwaPatternKind,
    pub confidence: f64,
    pub source: EwaHypothesisSource,
    pub candidate_index: Option<usize>,
    pub fib_score: f64,
    pub geometry_score: f64,
    pub technical_score: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EwaSwingFailure {
    pub code: String,
    pub count: usize,
}

#[derive(Debug, Clone)]
pub struct EwaSwingCoverage {
    pub world: String,
    pub segment_index: usize,
    pub start_pivot: usize,
    pub end_pivot: usize,
    pub bar_start: usize,
    pub bar_end: usize,
    pub direction: EwaSegmentDirection,
    pub candidate_count: usize,
    pub strict_candidate_count: usize,
    pub hypotheses: Vec<EwaSwingHypothesis>,
    pub failures: Vec<EwaSwingFailure>,
}

#[derive(Debug, Clone, Default)]
pub struct EwaRefreshTimings {
    pub subdivision_ms: f64,
    pub flatten_ms: f64,
    pub ranking_ms: f64,
    pub proof_ms: f64,
    pub nesting_ms: f64,
    pub count_tree_ms: f64,
    pub reinterpretation_ms: f64,
    pub swing_coverage_ms: f64,
    pub total_ms: f64,
}

#[derive(Debug, Clone)]
pub struct EwaAnalysis {
    pub pivots: Vec<EwaPivot>,
    pub segments: Vec<EwaSegment>,
    pub scenario_groups: Vec<EwaScenarioGroup>,
    pub scenarios: Vec<EwaScenario>,
    pub nesting_relations: Vec<EwaNestingRelation>,
    pub count_nodes: Vec<EwaCountNode>,
    pub subdivision_matches: Vec<EwaSubdivisionMatch>,
    pub reinterpretations: Vec<EwaReinterpretation>,
    pub swing_coverage: Vec<EwaSwingCoverage>,
    pub worlds: Vec<EwaWorldAnalysis>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EwaSemanticSnapshot {
    pub schema_version: u32,
    pub worlds: Vec<EwaSemanticWorld>,
    pub scenarios: Vec<EwaSemanticScenario>,
    pub count_nodes: Vec<EwaSemanticCountNode>,
    pub subdivision_edges: Vec<EwaSemanticSubdivisionEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EwaSemanticWorld {
    pub id: WorldId,
    pub label: String,
    pub pivots: Vec<PivotId>,
    pub segments: Vec<SegmentId>,
    pub candidates: Vec<CandidateId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EwaSemanticScenario {
    pub id: ScenarioId,
    pub candidate_id: CandidateId,
    pub pattern: EwaPatternKind,
    pub status: EwaScenarioStatus,
    pub proof_status: EwaProofStatus,
    pub bar_start: usize,
    pub bar_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EwaSemanticCountNode {
    pub id: CountNodeId,
    pub parent_id: Option<CountNodeId>,
    pub scenario_id: ScenarioId,
    pub candidate_id: CandidateId,
    pub pattern: EwaPatternKind,
    pub position: EwaWavePosition,
    pub proof_status: EwaProofStatus,
    pub children: Vec<CountNodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EwaSemanticSubdivisionEdge {
    pub parent_candidate_id: CandidateId,
    pub position: EwaWavePosition,
    pub child_candidate_id: CandidateId,
}

impl EwaAnalysis {
    pub fn candidate_count(&self) -> usize {
        self.worlds.iter().map(|world| world.candidates.len()).sum()
    }

    pub fn candidates(&self) -> impl Iterator<Item = &EwaCandidate> {
        self.worlds.iter().flat_map(|world| world.candidates.iter())
    }

    pub fn world_id(&self, world_index: usize) -> Option<WorldId> {
        self.worlds.get(world_index).map(EwaWorldAnalysis::id)
    }

    pub fn semantic_snapshot(&self) -> EwaSemanticSnapshot {
        let mut worlds = self
            .worlds
            .iter()
            .map(|world| {
                let mut candidates = (0..world.candidates.len())
                    .filter_map(|index| world.candidate_id(index))
                    .collect::<Vec<_>>();
                candidates.sort_unstable();
                EwaSemanticWorld {
                    id: world.id(),
                    label: world.label.clone(),
                    pivots: (0..world.pivots.len())
                        .filter_map(|index| world.pivot_id(index))
                        .collect(),
                    segments: (0..world.segments.len())
                        .filter_map(|index| world.segment_id(index))
                        .collect(),
                    candidates,
                }
            })
            .collect::<Vec<_>>();
        worlds.sort_by_key(|world| world.id);

        let mut scenarios = self
            .scenarios
            .iter()
            .map(|scenario| {
                EwaSemanticScenario {
                    id: scenario.id,
                    candidate_id: scenario.candidate_id,
                    pattern: scenario.candidate.pattern,
                    status: scenario.status,
                    proof_status: scenario.proof_status,
                    bar_start: scenario.bar_start,
                    bar_end: scenario.bar_end,
                }
            })
            .collect::<Vec<_>>();
        scenarios.sort_by_key(|scenario| scenario.id);

        let mut count_nodes = self
            .count_nodes
            .iter()
            .map(|node| {
                let mut children = node.children.clone();
                children.sort_unstable();
                EwaSemanticCountNode {
                    id: node.id,
                    parent_id: node.parent_id,
                    scenario_id: node.scenario_id,
                    candidate_id: node.candidate_id,
                    pattern: node.pattern,
                    position: node.position,
                    proof_status: node.proof_status,
                    children,
                }
            })
            .collect::<Vec<_>>();
        count_nodes.sort_by_key(|node| node.id);

        let mut subdivision_edges = self
            .subdivision_matches
            .iter()
            .map(|item| EwaSemanticSubdivisionEdge {
                parent_candidate_id: item.parent_candidate_id,
                position: item.position,
                child_candidate_id: item.child_candidate_id,
            })
            .collect::<Vec<_>>();
        subdivision_edges.sort_by_key(|edge| {
            (
                edge.parent_candidate_id,
                wave_position_semantic_order(edge.position),
                edge.child_candidate_id,
            )
        });

        EwaSemanticSnapshot {
            schema_version: 1,
            worlds,
            scenarios,
            count_nodes,
            subdivision_edges,
        }
    }
}

fn wave_position_semantic_order(position: EwaWavePosition) -> u8 {
    match position {
        EwaWavePosition::Root => 0,
        EwaWavePosition::Wave1 => 1,
        EwaWavePosition::Wave2 => 2,
        EwaWavePosition::Wave3 => 3,
        EwaWavePosition::Wave4 => 4,
        EwaWavePosition::Wave5 => 5,
        EwaWavePosition::A => 6,
        EwaWavePosition::B => 7,
        EwaWavePosition::C => 8,
        EwaWavePosition::D => 9,
        EwaWavePosition::E => 10,
        EwaWavePosition::W => 11,
        EwaWavePosition::X => 12,
        EwaWavePosition::Y => 13,
        EwaWavePosition::Z => 14,
    }
}

impl EwaWorldAnalysis {
    pub fn id(&self) -> WorldId {
        WorldId::from_label(&self.label)
    }

    pub fn pivot_id(&self, pivot_index: usize) -> Option<PivotId> {
        let pivot = self.pivots.get(pivot_index)?;
        Some(PivotId::new(self.id(), pivot_index, pivot.kind))
    }

    pub fn segment_id(&self, segment_index: usize) -> Option<SegmentId> {
        let segment = self.segments.get(segment_index)?;
        Some(SegmentId::new(
            self.id(),
            segment.start_pivot,
            segment.end_pivot,
        ))
    }

    pub fn candidate_id(&self, candidate_index: usize) -> Option<CandidateId> {
        let candidate = self.candidates.get(candidate_index)?;
        Some(CandidateId::from_candidate(self.id(), candidate))
    }

    pub fn candidate_index(&self, candidate_id: CandidateId) -> Option<usize> {
        self.candidates.iter().position(|candidate| {
            CandidateId::from_candidate(self.id(), candidate) == candidate_id
        })
    }
}
