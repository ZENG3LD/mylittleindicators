//! Elliott Wave Analysis.
//!
//! The declarative half, cloned from the proprietary EWA engine
//! (`mylittlequant/crates/mli/src/ewa`): the pattern catalogue
//! (`primitives`), the position grammar (`grammar`), the ratio bands and
//! geometry predicates (`rules`), the Fibonacci retracement / extension
//! targets (`ratios`), and the harmonic pattern target ratio tables
//! (`harmonic`) — what each `EwaPatternKind` harmonic shape (Gartley, Bat,
//! Butterfly, Crab, Shark, Cypher, Abcd, ThreeDrives) actually means.
//!
//! A lighter, independent COMPUTE half is growing alongside it (`swing` so
//! far; a candidate-enumeration module follows) — pivot/segment extraction
//! over `&[Bar]`-shaped price data, without the proprietary engine's
//! probabilistic apparatus (no ranker, no priors, no probability model, no
//! reinterpretation, no live runtime) — a fresh OSS implementation, not a
//! port; see `docs/mlc/plans/autodetectors-arc-2026-09-23.md` §1/§2 for the
//! architecture decision.

pub mod grammar;
pub mod harmonic;
pub mod primitives;
pub mod ratios;
pub mod rules;
pub mod swing;
pub mod types;

pub use grammar::{
    EwaGrammarClass, EwaGrammarSlot, EwaWavePosition, allowed_in_position, matches_class,
    is_scenario_pattern, slots_for,
};
pub use harmonic::{DEFAULT_HARMONIC_TOLERANCE, HarmonicLeg, ratio_matches, targets_for, widen_band};
pub use primitives::{EwaPrimitiveFamily, EwaPrimitiveSpec, EwaSubdivision, all_ewa_primitives};
pub use swing::{EwaSwingPivot, EwaSwingSegment, PivotKind, build_segments, extract_pivots};
pub use types::{
    EwaFibRelation, EwaFibRelationKind, EwaPatternKind, EwaRatio, EwaSegment, EwaSegmentDirection,
};
