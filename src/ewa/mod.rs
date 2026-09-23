//! Elliott Wave Analysis.
//!
//! The declarative half, cloned from the proprietary EWA engine
//! (`mylittlequant/crates/mli/src/ewa`): the pattern catalogue
//! (`primitives`), the position grammar (`grammar`), the ratio bands and
//! geometry predicates (`rules`), the Fibonacci retracement / extension
//! targets (`ratios`), and the harmonic pattern target ratio tables
//! (`harmonic`) — what each `EwaPatternKind` harmonic shape (Gartley, Bat,
//! AltBat, Butterfly, Crab, DeepCrab, Shark, Cypher, Abcd, ThreeDrives,
//! FiveZero) actually means.
//!
//! A lighter, independent COMPUTE half lives alongside it (`swing`, `scan`)
//! — pivot/segment extraction and single-degree candidate enumeration over
//! `&[Bar]`-shaped price data, validated through the SAME `grammar`/`rules`
//! predicates. This mirrors the proprietary engine's algorithm SHAPE
//! (window-by-segment-count enumeration) without its probabilistic
//! apparatus (no ranker, no priors, no probability model, no
//! reinterpretation, no live runtime) — a fresh OSS implementation, not a
//! port; see `docs/mlc/plans/autodetectors-arc-2026-09-23.md` §1/§2 for the
//! architecture decision.
//!
//! `fib_swing` selects which of `swing`'s own confirmed segments a
//! Fibonacci autodetector should draw retracement/extension levels on
//! inside a bar range — largest swing, last N swings, direction-filtered —
//! a thin selection layer over the same pivot/segment output, no new
//! detection math.

pub mod fib_swing;
pub mod grammar;
pub mod harmonic;
pub mod primitives;
pub mod ratios;
pub mod rules;
pub mod scan;
pub mod swing;
pub mod types;

pub use fib_swing::{FibSwingDirection, FibSwingMode, FibSwingPair, select_fib_swings};
pub use grammar::{
    EwaGrammarClass, EwaGrammarSlot, EwaWavePosition, allowed_in_position, matches_class,
    is_scenario_pattern, slots_for,
};
pub use harmonic::{DEFAULT_HARMONIC_TOLERANCE, HarmonicLeg, ratio_matches, targets_for, widen_band};
pub use primitives::{EwaPrimitiveFamily, EwaPrimitiveSpec, EwaSubdivision, all_ewa_primitives};
pub use scan::{EwaWaveHit, MAX_PIVOTS_PER_SCAN, scan_waves};
pub use swing::{EwaSwingPivot, EwaSwingSegment, PivotKind, build_segments, extract_pivots};
pub use types::{
    EwaFibRelation, EwaFibRelationKind, EwaPatternKind, EwaRatio, EwaSegment, EwaSegmentDirection,
};
