//! Elliott Wave Analysis — declarative half.
//!
//! This is the declarative half of the Elliott Wave model, cloned from the
//! proprietary EWA engine (`mylittlequant/crates/mli/src/ewa`): the pattern
//! catalogue (`primitives`), the position grammar (`grammar`), the ratio
//! bands and geometry predicates (`rules`), the Fibonacci retracement /
//! extension targets (`ratios`), and the harmonic pattern target ratio
//! tables (`harmonic`) — what each `EwaPatternKind` harmonic shape (Gartley,
//! Bat, Butterfly, Crab, Shark, Cypher, Abcd, ThreeDrives) actually means.
//! The proprietary fork keeps everything compute-side — the pivot/segment
//! scanner, the candidate ranker, the probability model, priors,
//! reinterpretation, and the live runtime.
//!
//! A consumer of this module gets the Elliott vocabulary and the rules a
//! pattern must satisfy — not automatic wave-count enumeration. Building
//! candidates from price data, scoring them, and picking a count are out of
//! scope here by design.

pub mod grammar;
pub mod harmonic;
pub mod primitives;
pub mod ratios;
pub mod rules;
pub mod types;

pub use grammar::{
    EwaGrammarClass, EwaGrammarSlot, EwaWavePosition, allowed_in_position, matches_class,
    is_scenario_pattern, slots_for,
};
pub use harmonic::{DEFAULT_HARMONIC_TOLERANCE, HarmonicLeg, ratio_matches, targets_for, widen_band};
pub use primitives::{EwaPrimitiveFamily, EwaPrimitiveSpec, EwaSubdivision, all_ewa_primitives};
pub use types::{
    EwaFibRelation, EwaFibRelationKind, EwaPatternKind, EwaRatio, EwaSegment, EwaSegmentDirection,
};
