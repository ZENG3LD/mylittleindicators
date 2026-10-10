use crate::ewa::types::EwaPatternKind;

use super::types::HarmonicKind;

#[derive(Debug, Clone, Copy)]
pub struct HarmonicReinterpretation {
    pub from: HarmonicKind,
    pub to: EwaPatternKind,
    pub condition: &'static str,
    pub granularity_effect: &'static str,
    pub required_context: &'static str,
}

pub fn reinterpretation_matrix() -> &'static [HarmonicReinterpretation] {
    REINTERPRETATION_MATRIX
}

const REINTERPRETATION_MATRIX: &[HarmonicReinterpretation] = &[
    HarmonicReinterpretation {
        from: HarmonicKind::Abcd,
        to: EwaPatternKind::Zigzag,
        condition: "ABCD harmonic overlay maps to A-B-C zigzag when CD is the C leg projection",
        granularity_effect: "coarser pass may see ABCD symmetry; EWA pass labels A-B-C",
        required_context: "requires corrective context, not motive continuation",
    },
    HarmonicReinterpretation {
        from: HarmonicKind::Xabcd,
        to: EwaPatternKind::Flat,
        condition: "XABCD completion near the origin can be a flat subtype rather than standalone harmonic pattern",
        granularity_effect: "endpoint relation to A origin and A endpoint decides flat subtype",
        required_context: "harmonics are PRZ evidence, not replacement for EWA position rules",
    },
    HarmonicReinterpretation {
        from: HarmonicKind::Gartley,
        to: EwaPatternKind::Zigzag,
        condition: "Gartley ratios can describe a zigzag correction with harmonic PRZ confluence",
        granularity_effect: "EWA count should own the label; harmonic pattern boosts ratio evidence",
        required_context: "use as confluence when A-B-C rules are also valid",
    },
    HarmonicReinterpretation {
        from: HarmonicKind::Bat,
        to: EwaPatternKind::Flat,
        condition: "Bat completion can coincide with flat C termination zone",
        granularity_effect: "nearby endpoints can flip harmonic overlay while EWA structure stays corrective",
        required_context: "prefer EWA label; keep harmonic as probability boost",
    },
    HarmonicReinterpretation {
        from: HarmonicKind::Crab,
        to: EwaPatternKind::ExpandedFlat,
        condition: "deep crab completion can coincide with expanded flat C extension",
        granularity_effect: "C extension magnitude and false-break B decide expanded flat reading",
        required_context: "requires B beyond origin and C beyond A endpoint",
    },
];
