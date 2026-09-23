//! Harmonic pattern target ratios — the declarative table that says what
//! `Xabcd`'s siblings in [`EwaPatternKind`] actually MEAN.
//!
//! A harmonic pattern is defined by its table of Fibonacci target ratios: for
//! each named leg of the XABCD shape, which prior leg it is measured
//! against, and the band the ratio must land in for that leg to qualify.
//! [`EwaPatternKind`] (from [`super::types`]) already names the shapes;
//! [`RatioBand`] (from [`super::rules`]) already carries "band" as a
//! concept — this module supplies only the table, no new vocabulary.
//!
//! The tables below are Scott Carney's published ratio definitions
//! ("Harmonic Trading", Carney) for `Gartley`, `Bat`, `AltBat`, `Butterfly`,
//! `Crab`, `DeepCrab`, `Shark`, `Cypher` and the generic `Abcd`/`ThreeDrives`
//! shapes — not this module's own judgement. `FiveZero` (the "5-0" pattern)
//! is Suri Duddella's, not Carney's own, but is grouped here per the same
//! published-ratio-table posture since it is commonly scanned alongside the
//! Carney set. `Xabcd` is the unclassified five-point container and
//! deliberately carries no table (see [`targets_for`]).
//!
//! Every table entry obeys one structural invariant: the measured leg starts
//! exactly where its reference leg ends (`leg.0 == reference.1`). That is
//! what makes `reference` a *real* leg of the pattern rather than an
//! arbitrary pair of points, and it is what the consistency test below
//! checks for every kind.

use super::rules::RatioBand;
use super::types::EwaPatternKind;

/// Default tolerance (an absolute delta applied to both sides of a ratio) for
/// harmonic targets published as a single "exact" number — e.g. Gartley's
/// `AB = 0.618 XA`. A ratio measured off real price data is essentially never
/// bit-exact to a published Fibonacci constant, so every "exact" target in
/// this module is expressed as a narrow [`RatioBand`] centered on the
/// published value (see [`exact`]) rather than as an equality test on a
/// float. A consumer that wants a looser or tighter tolerance for one
/// drawing calls [`widen_band`] instead of re-deriving the arithmetic.
pub const DEFAULT_HARMONIC_TOLERANCE: f64 = 0.005;

/// One leg of a harmonic pattern's target ratio table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HarmonicLeg {
    /// The measured leg, by pivot names — its own two endpoints, e.g.
    /// `("A", "B")` for the AB leg.
    pub leg: (&'static str, &'static str),
    /// The leg `leg` is measured against, by pivot names. By construction
    /// `leg.0 == reference.1`: the measured leg begins exactly where the
    /// reference leg it is compared against ends.
    pub reference: (&'static str, &'static str),
    /// The band `length(leg) / length(reference)` must fall in for this leg
    /// to satisfy the pattern.
    pub band: RatioBand,
}

/// Widens (or, with a negative `delta`, narrows) `band` on both sides by
/// `delta`. Lets a consumer relax or tighten [`DEFAULT_HARMONIC_TOLERANCE`]
/// for one drawing without hand-rolling the min/max arithmetic that
/// [`RatioBand`] already owns.
pub fn widen_band(band: RatioBand, delta: f64) -> RatioBand {
    RatioBand::new(band.min - delta, band.max + delta)
}

/// Whether `ratio` (a measured `length(leg) / length(reference)`) satisfies
/// `leg`'s band. A thin wrapper over [`RatioBand::contains`] — this module
/// does not re-derive band membership, it only supplies the tables.
pub fn ratio_matches(ratio: f64, leg: &HarmonicLeg) -> bool {
    leg.band.contains(ratio)
}

/// Builds an "exact" target as a band of width `2 * DEFAULT_HARMONIC_TOLERANCE`
/// centered on `value`, per the doc on [`DEFAULT_HARMONIC_TOLERANCE`].
const fn exact(value: f64) -> RatioBand {
    RatioBand::new(value - DEFAULT_HARMONIC_TOLERANCE, value + DEFAULT_HARMONIC_TOLERANCE)
}

/// Gartley (Carney): `AB = 0.618 XA` exact; `BC = 0.382-0.886 AB`;
/// `CD = 1.13-1.618 BC`; `AD = 0.786 XA` exact.
const GARTLEY: &[HarmonicLeg] = &[
    HarmonicLeg { leg: ("A", "B"), reference: ("X", "A"), band: exact(0.618) },
    HarmonicLeg { leg: ("B", "C"), reference: ("A", "B"), band: RatioBand::new(0.382, 0.886) },
    HarmonicLeg { leg: ("C", "D"), reference: ("B", "C"), band: RatioBand::new(1.13, 1.618) },
    HarmonicLeg { leg: ("A", "D"), reference: ("X", "A"), band: exact(0.786) },
];

/// Bat (Carney): `AB = 0.382-0.50 XA`; `BC = 0.382-0.886 AB`;
/// `CD = 1.618-2.618 BC`; `AD = 0.886 XA` exact.
const BAT: &[HarmonicLeg] = &[
    HarmonicLeg { leg: ("A", "B"), reference: ("X", "A"), band: RatioBand::new(0.382, 0.50) },
    HarmonicLeg { leg: ("B", "C"), reference: ("A", "B"), band: RatioBand::new(0.382, 0.886) },
    HarmonicLeg { leg: ("C", "D"), reference: ("B", "C"), band: RatioBand::new(1.618, 2.618) },
    HarmonicLeg { leg: ("A", "D"), reference: ("X", "A"), band: exact(0.886) },
];

/// Butterfly (Carney): `AB = 0.786 XA` exact; `BC = 0.382-0.886 AB`;
/// `CD = 1.618-2.24 BC`; `AD = 1.27-1.618 XA`.
const BUTTERFLY: &[HarmonicLeg] = &[
    HarmonicLeg { leg: ("A", "B"), reference: ("X", "A"), band: exact(0.786) },
    HarmonicLeg { leg: ("B", "C"), reference: ("A", "B"), band: RatioBand::new(0.382, 0.886) },
    HarmonicLeg { leg: ("C", "D"), reference: ("B", "C"), band: RatioBand::new(1.618, 2.24) },
    HarmonicLeg { leg: ("A", "D"), reference: ("X", "A"), band: RatioBand::new(1.27, 1.618) },
];

/// Crab (Carney): `AB = 0.382-0.618 XA`; `BC = 0.382-0.886 AB`;
/// `CD = 2.618-3.618 BC`; `AD = 1.618 XA` exact.
const CRAB: &[HarmonicLeg] = &[
    HarmonicLeg { leg: ("A", "B"), reference: ("X", "A"), band: RatioBand::new(0.382, 0.618) },
    HarmonicLeg { leg: ("B", "C"), reference: ("A", "B"), band: RatioBand::new(0.382, 0.886) },
    HarmonicLeg { leg: ("C", "D"), reference: ("B", "C"), band: RatioBand::new(2.618, 3.618) },
    HarmonicLeg { leg: ("A", "D"), reference: ("X", "A"), band: exact(1.618) },
];

/// Shark (Carney): `AB = 0.382-0.618 XA`; `AC = 1.13-1.618 XA`;
/// `CD = 0.886-1.13 XC`.
const SHARK: &[HarmonicLeg] = &[
    HarmonicLeg { leg: ("A", "B"), reference: ("X", "A"), band: RatioBand::new(0.382, 0.618) },
    HarmonicLeg { leg: ("A", "C"), reference: ("X", "A"), band: RatioBand::new(1.13, 1.618) },
    HarmonicLeg { leg: ("C", "D"), reference: ("X", "C"), band: RatioBand::new(0.886, 1.13) },
];

/// Alternate Bat (Carney): `AB = 0.382 XA` exact; `BC = 0.382-0.886 AB`;
/// `CD = 2.0-3.618 BC`; `AD = 1.13 XA` exact.
const ALT_BAT: &[HarmonicLeg] = &[
    HarmonicLeg { leg: ("A", "B"), reference: ("X", "A"), band: exact(0.382) },
    HarmonicLeg { leg: ("B", "C"), reference: ("A", "B"), band: RatioBand::new(0.382, 0.886) },
    HarmonicLeg { leg: ("C", "D"), reference: ("B", "C"), band: RatioBand::new(2.0, 3.618) },
    HarmonicLeg { leg: ("A", "D"), reference: ("X", "A"), band: exact(1.13) },
];

/// Deep Crab (Carney): `AB = 0.886 XA` exact; `BC = 0.382-0.886 AB`;
/// `CD = 2.24-3.618 BC`; `AD = 1.618 XA` exact.
const DEEP_CRAB: &[HarmonicLeg] = &[
    HarmonicLeg { leg: ("A", "B"), reference: ("X", "A"), band: exact(0.886) },
    HarmonicLeg { leg: ("B", "C"), reference: ("A", "B"), band: RatioBand::new(0.382, 0.886) },
    HarmonicLeg { leg: ("C", "D"), reference: ("B", "C"), band: RatioBand::new(2.24, 3.618) },
    HarmonicLeg { leg: ("A", "D"), reference: ("X", "A"), band: exact(1.618) },
];

/// Cypher (Carney): `AB = 0.382-0.618 XA`; `AC = 1.272-1.414 XA`;
/// `CD = 0.786 XC` exact.
const CYPHER: &[HarmonicLeg] = &[
    HarmonicLeg { leg: ("A", "B"), reference: ("X", "A"), band: RatioBand::new(0.382, 0.618) },
    HarmonicLeg { leg: ("A", "C"), reference: ("X", "A"), band: RatioBand::new(1.272, 1.414) },
    HarmonicLeg { leg: ("C", "D"), reference: ("X", "C"), band: exact(0.786) },
];

/// ABCD (Carney, the four-point generic): `BC = 0.618-0.786 AB`;
/// `CD = 1.272-1.618 BC`.
const ABCD: &[HarmonicLeg] = &[
    HarmonicLeg { leg: ("B", "C"), reference: ("A", "B"), band: RatioBand::new(0.618, 0.786) },
    HarmonicLeg { leg: ("C", "D"), reference: ("B", "C"), band: RatioBand::new(1.272, 1.618) },
];

/// Three Drives (Carney): pivots `X, 1, A, 2, B, 3` — start, three drive legs
/// (`X-1`, `A-2`, `B-3`) and two retracement legs (`1-A`, `2-B`) between
/// them. Each retracement is `0.618-0.786` of the drive it follows; each
/// drive after the first is `1.272-1.618` of the retracement it follows.
const THREE_DRIVES: &[HarmonicLeg] = &[
    HarmonicLeg { leg: ("1", "A"), reference: ("X", "1"), band: RatioBand::new(0.618, 0.786) },
    HarmonicLeg { leg: ("A", "2"), reference: ("1", "A"), band: RatioBand::new(1.272, 1.618) },
    HarmonicLeg { leg: ("2", "B"), reference: ("A", "2"), band: RatioBand::new(0.618, 0.786) },
    HarmonicLeg { leg: ("B", "3"), reference: ("2", "B"), band: RatioBand::new(1.272, 1.618) },
];

/// 5-0 (Duddella, grouped here per the module doc's note above): 6 points
/// `0-X-A-B-C-D`. `AB = 1.13-1.618 XA`; `BC = 1.618-2.24 AB`;
/// `CD = 0.5 BC` exact — the "reciprocal AB=CD" 50% retracement that names
/// the pattern. The leading `0` point anchors the 6-pivot window (matching
/// the shape's own published point count) but carries no ratio constraint
/// of its own in this table.
const FIVE_ZERO: &[HarmonicLeg] = &[
    HarmonicLeg { leg: ("A", "B"), reference: ("X", "A"), band: RatioBand::new(1.13, 1.618) },
    HarmonicLeg { leg: ("B", "C"), reference: ("A", "B"), band: RatioBand::new(1.618, 2.24) },
    HarmonicLeg { leg: ("C", "D"), reference: ("B", "C"), band: exact(0.5) },
];

/// The target ratio table for `kind`, or an empty slice if `kind` has no
/// fixed harmonic table.
///
/// `EwaPatternKind::Xabcd` always returns an empty slice by design: it names
/// the generic, unclassified five-point shape — the container an analyst
/// classifies into one of the other harmonic kinds — not a pattern with its
/// own targets. An empty slice therefore means "unclassified", not "no
/// constraints"; callers must not treat it as "anything matches". Every
/// non-harmonic `EwaPatternKind` (the Elliott motive/corrective shapes) also
/// returns an empty slice, for the same reason: this table only speaks for
/// the harmonic family.
pub fn targets_for(kind: EwaPatternKind) -> &'static [HarmonicLeg] {
    match kind {
        EwaPatternKind::Gartley => GARTLEY,
        EwaPatternKind::Bat => BAT,
        EwaPatternKind::AltBat => ALT_BAT,
        EwaPatternKind::Butterfly => BUTTERFLY,
        EwaPatternKind::Crab => CRAB,
        EwaPatternKind::DeepCrab => DEEP_CRAB,
        EwaPatternKind::Shark => SHARK,
        EwaPatternKind::Cypher => CYPHER,
        EwaPatternKind::Abcd => ABCD,
        EwaPatternKind::ThreeDrives => THREE_DRIVES,
        EwaPatternKind::FiveZero => FIVE_ZERO,
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `EwaPatternKind` this module claims to have a table for.
    const HARMONIC_KINDS: &[EwaPatternKind] = &[
        EwaPatternKind::Gartley,
        EwaPatternKind::Bat,
        EwaPatternKind::AltBat,
        EwaPatternKind::Butterfly,
        EwaPatternKind::Crab,
        EwaPatternKind::DeepCrab,
        EwaPatternKind::Shark,
        EwaPatternKind::Cypher,
        EwaPatternKind::Abcd,
        EwaPatternKind::ThreeDrives,
        EwaPatternKind::FiveZero,
    ];

    #[test]
    fn xabcd_is_unclassified_and_carries_no_table() {
        assert!(targets_for(EwaPatternKind::Xabcd).is_empty());
    }

    #[test]
    fn non_harmonic_kinds_carry_no_table() {
        assert!(targets_for(EwaPatternKind::Impulse).is_empty());
        assert!(targets_for(EwaPatternKind::Zigzag).is_empty());
        assert!(targets_for(EwaPatternKind::ContractingTriangle).is_empty());
    }

    #[test]
    fn every_table_is_internally_consistent() {
        for &kind in HARMONIC_KINDS {
            let table = targets_for(kind);
            assert!(!table.is_empty(), "{kind:?} should carry a non-empty table");

            for entry in table {
                assert_ne!(
                    entry.leg.0, entry.leg.1,
                    "{kind:?}: a leg needs two distinct endpoints, got {:?}",
                    entry.leg
                );
                assert_ne!(
                    entry.reference.0, entry.reference.1,
                    "{kind:?}: a reference needs two distinct endpoints, got {:?}",
                    entry.reference
                );
                assert_ne!(
                    entry.leg, entry.reference,
                    "{kind:?}: leg {:?} must not be measured against itself",
                    entry.leg
                );
                // The invariant every table above is built on: the measured
                // leg starts exactly where its reference leg ends, which is
                // what makes `reference` a real leg of the pattern rather
                // than an arbitrary pivot pair.
                assert_eq!(
                    entry.leg.0, entry.reference.1,
                    "{kind:?}: leg {:?} does not start where reference {:?} ends",
                    entry.leg, entry.reference
                );
            }
        }
    }

    #[test]
    fn exact_targets_accept_their_nominal_ratio() {
        let cases: &[(EwaPatternKind, (&str, &str), f64)] = &[
            (EwaPatternKind::Gartley, ("A", "B"), 0.618),
            (EwaPatternKind::Gartley, ("A", "D"), 0.786),
            (EwaPatternKind::Bat, ("A", "D"), 0.886),
            (EwaPatternKind::AltBat, ("A", "B"), 0.382),
            (EwaPatternKind::AltBat, ("A", "D"), 1.13),
            (EwaPatternKind::Butterfly, ("A", "B"), 0.786),
            (EwaPatternKind::Crab, ("A", "D"), 1.618),
            (EwaPatternKind::DeepCrab, ("A", "B"), 0.886),
            (EwaPatternKind::DeepCrab, ("A", "D"), 1.618),
            (EwaPatternKind::Cypher, ("C", "D"), 0.786),
            (EwaPatternKind::FiveZero, ("C", "D"), 0.5),
        ];

        for &(kind, leg, nominal) in cases {
            let entry = targets_for(kind)
                .iter()
                .find(|entry| entry.leg == leg)
                .unwrap_or_else(|| panic!("{kind:?} has no {leg:?} leg in its table"));
            assert!(
                ratio_matches(nominal, entry),
                "{kind:?} leg {leg:?} should accept its own nominal ratio {nominal}"
            );
        }
    }

    #[test]
    fn exact_targets_reject_a_ratio_clearly_outside_tolerance() {
        let entry = targets_for(EwaPatternKind::Gartley)
            .iter()
            .find(|entry| entry.leg == ("A", "B"))
            .unwrap_or_else(|| panic!("Gartley has no (A, B) leg in its table"));
        // Ten times the default tolerance away from 0.618 — well outside any
        // sane widening, so this is unambiguously a rejection, not a
        // boundary case.
        let clearly_outside = 0.618 + 10.0 * DEFAULT_HARMONIC_TOLERANCE;
        assert!(!ratio_matches(clearly_outside, entry));
    }

    #[test]
    fn band_edges_follow_ratio_band_contains_documented_behaviour() {
        // RatioBand::contains is documented (rules.rs) as an inclusive
        // range: `value >= min && value <= max`. Assert that documented
        // behaviour at the edges rather than assuming it here.
        let entry = targets_for(EwaPatternKind::Abcd)
            .iter()
            .find(|entry| entry.leg == ("B", "C"))
            .unwrap_or_else(|| panic!("Abcd has no (B, C) leg in its table"));

        assert!(ratio_matches(entry.band.min, entry));
        assert!(ratio_matches(entry.band.max, entry));
        assert!(!ratio_matches(entry.band.min - f64::EPSILON.max(1e-9), entry));
        assert!(!ratio_matches(entry.band.max + f64::EPSILON.max(1e-9), entry));
    }

    #[test]
    fn widen_band_loosens_both_sides() {
        let base = RatioBand::new(0.5, 0.6);
        let widened = widen_band(base, 0.1);
        assert!((widened.min - 0.4).abs() < 1e-12);
        assert!((widened.max - 0.7).abs() < 1e-12);
    }
}
