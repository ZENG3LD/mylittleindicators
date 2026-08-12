//! Elliott Wave Analysis — declarative core types.
//!
//! Trimmed from the proprietary engine's `types.rs` (782 lines of
//! candidate/scenario/world/engine bookkeeping) down to the six types the
//! declarative half (`primitives`, `grammar`, `rules`, `ratios`) actually
//! reads: the pattern taxonomy, segment direction, the segment shape itself,
//! and the fib-ratio/relation carriers. Everything else in the source file —
//! pivots, candidates, scenarios, count nodes, world analyses, rule settings
//! — belongs to the compute half and stays proprietary.

/// Every Elliott Wave pattern shape the declarative model names.
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

/// Direction a price segment moves between two pivots.
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

/// A single leg between two pivots — the geometric unit the rule/ratio
/// predicates reason about.
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

/// One measured ratio against its nearest Fibonacci target.
#[derive(Debug, Clone)]
pub struct EwaRatio {
    pub name: String,
    pub actual: f64,
    pub target: f64,
    pub error: f64,
    pub weight: f64,
}

/// Whether a fib relation between two segments is a retracement (opposite
/// direction) or an extension (same direction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EwaFibRelationKind {
    Retracement,
    Extension,
}

/// A measured Fibonacci relation between two segments.
#[derive(Debug, Clone)]
pub struct EwaFibRelation {
    pub previous_segment: usize,
    pub current_segment: usize,
    pub kind: EwaFibRelationKind,
    pub ratio: f64,
    pub nearest_target: f64,
    pub error: f64,
}
