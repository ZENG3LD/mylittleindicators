
use super::types::EwaPatternKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EwaPrimitiveFamily {
    Motive,
    Corrective,
    Harmonic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EwaSubdivision {
    Five,
    Three,
    Either,
}

#[derive(Debug, Clone)]
pub struct EwaPrimitiveSpec {
    pub kind: EwaPatternKind,
    pub family: EwaPrimitiveFamily,
    pub notation: &'static str,
    pub pivots: usize,
    pub subdivisions: &'static [EwaSubdivision],
    pub rules: &'static [&'static str],
    pub guidelines: &'static [&'static str],
}

pub fn all_ewa_primitives() -> &'static [EwaPrimitiveSpec] {
    EWA_PRIMITIVES
}

const F53535: &[EwaSubdivision] = &[
    EwaSubdivision::Five,
    EwaSubdivision::Three,
    EwaSubdivision::Five,
    EwaSubdivision::Three,
    EwaSubdivision::Five,
];
const T33333: &[EwaSubdivision] = &[
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
];
const C535: &[EwaSubdivision] = &[
    EwaSubdivision::Five,
    EwaSubdivision::Three,
    EwaSubdivision::Five,
];
const C335: &[EwaSubdivision] = &[
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Five,
];
const COMBO7: &[EwaSubdivision] = &[
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
];
const COMBO11: &[EwaSubdivision] = &[
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
    EwaSubdivision::Three,
];

const EWA_PRIMITIVES: &[EwaPrimitiveSpec] = &[
    EwaPrimitiveSpec {
        kind: EwaPatternKind::Impulse,
        family: EwaPrimitiveFamily::Motive,
        notation: "1-2-3-4-5",
        pivots: 6,
        subdivisions: F53535,
        rules: &["wave3_not_shortest", "wave2_not_beyond_wave1_origin", "wave4_no_wave1_overlap"],
        guidelines: &["wave3_often_extended", "wave2_wave4_alternation", "channeling"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::ImpulseExtendedWave1,
        family: EwaPrimitiveFamily::Motive,
        notation: "1x-2-3-4-5",
        pivots: 6,
        subdivisions: F53535,
        rules: &["impulse_core", "wave1_extension_dominant"],
        guidelines: &["rare_vs_wave3_extension"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::ImpulseExtendedWave3,
        family: EwaPrimitiveFamily::Motive,
        notation: "1-2-3x-4-5",
        pivots: 6,
        subdivisions: F53535,
        rules: &["impulse_core", "wave3_extension_dominant"],
        guidelines: &["most_common_extension"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::ImpulseExtendedWave5,
        family: EwaPrimitiveFamily::Motive,
        notation: "1-2-3-4-5x",
        pivots: 6,
        subdivisions: F53535,
        rules: &["impulse_core", "wave5_extension_dominant"],
        guidelines: &["often_commodity_like"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::TruncatedImpulse,
        family: EwaPrimitiveFamily::Motive,
        notation: "1-2-3-4-5(truncated)",
        pivots: 6,
        subdivisions: F53535,
        rules: &["impulse_core", "wave5_fails_to_exceed_wave3"],
        guidelines: &["signals_powerful_reversal"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::LeadingDiagonalContracting,
        family: EwaPrimitiveFamily::Motive,
        notation: "LD 1-2-3-4-5",
        pivots: 6,
        subdivisions: T33333,
        rules: &["diagonal_core", "wave4_overlaps_wave1", "contracting_boundaries"],
        guidelines: &["wave1_or_wave_a_position"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::LeadingDiagonalExpanding,
        family: EwaPrimitiveFamily::Motive,
        notation: "LD expanding 1-2-3-4-5",
        pivots: 6,
        subdivisions: T33333,
        rules: &["diagonal_core", "wave4_overlaps_wave1", "expanding_boundaries"],
        guidelines: &["rare"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::EndingDiagonalContracting,
        family: EwaPrimitiveFamily::Motive,
        notation: "ED 1-2-3-4-5",
        pivots: 6,
        subdivisions: T33333,
        rules: &["diagonal_core", "wave4_overlaps_wave1", "contracting_boundaries"],
        guidelines: &["wave5_or_wave_c_position", "sharp_reversal_after_completion"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::EndingDiagonalExpanding,
        family: EwaPrimitiveFamily::Motive,
        notation: "ED expanding 1-2-3-4-5",
        pivots: 6,
        subdivisions: T33333,
        rules: &["diagonal_core", "wave4_overlaps_wave1", "expanding_boundaries"],
        guidelines: &["very_rare"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::Zigzag,
        family: EwaPrimitiveFamily::Corrective,
        notation: "A-B-C",
        pivots: 4,
        subdivisions: C535,
        rules: &["b_does_not_retrace_a_origin", "c_exceeds_a_end"],
        guidelines: &["sharp_correction", "a_and_c_often_related_by_equality_or_1_618"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::RunningZigzag,
        family: EwaPrimitiveFamily::Corrective,
        notation: "A-B-C running",
        pivots: 4,
        subdivisions: C535,
        rules: &["zigzag_core", "c_fails_to_exceed_a_end"],
        guidelines: &["strong_larger_trend"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::DoubleZigzag,
        family: EwaPrimitiveFamily::Corrective,
        notation: "W-X-Y",
        pivots: 10,
        subdivisions: COMBO7,
        rules: &["w_and_y_are_zigzags", "x_is_corrective"],
        guidelines: &["deepens_price_correction"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::TripleZigzag,
        family: EwaPrimitiveFamily::Corrective,
        notation: "W-X-Y-X-Z",
        pivots: 16,
        subdivisions: COMBO11,
        rules: &["w_y_z_are_zigzags", "x_waves_are_corrective"],
        guidelines: &["rare", "at_most_three_zigzags"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::RegularFlat,
        family: EwaPrimitiveFamily::Corrective,
        notation: "A-B-C regular flat",
        pivots: 4,
        subdivisions: C335,
        rules: &["b_terminates_near_a_origin", "c_slightly_exceeds_a_end"],
        guidelines: &["sideways_correction"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::ExpandedFlat,
        family: EwaPrimitiveFamily::Corrective,
        notation: "A-B-C expanded flat",
        pivots: 4,
        subdivisions: C335,
        rules: &["b_exceeds_a_origin", "c_exceeds_a_end"],
        guidelines: &["common_flat_variant"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::RunningFlat,
        family: EwaPrimitiveFamily::Corrective,
        notation: "A-B-C running flat",
        pivots: 4,
        subdivisions: C335,
        rules: &["b_exceeds_a_origin", "c_fails_to_exceed_a_end"],
        guidelines: &["rare", "strong_larger_trend"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::ContractingTriangle,
        family: EwaPrimitiveFamily::Corrective,
        notation: "A-B-C-D-E contracting",
        pivots: 6,
        subdivisions: T33333,
        rules: &["five_overlapping_threes", "contracting_boundaries"],
        guidelines: &["wave4_b_or_final_x_position"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::BarrierTriangle,
        family: EwaPrimitiveFamily::Corrective,
        notation: "A-B-C-D-E barrier",
        pivots: 6,
        subdivisions: T33333,
        rules: &["five_overlapping_threes", "one_boundary_near_horizontal"],
        guidelines: &["thrust_follows_triangle"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::ExpandingTriangle,
        family: EwaPrimitiveFamily::Corrective,
        notation: "A-B-C-D-E expanding",
        pivots: 6,
        subdivisions: T33333,
        rules: &["five_overlapping_threes", "expanding_boundaries"],
        guidelines: &["rare"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::RunningTriangle,
        family: EwaPrimitiveFamily::Corrective,
        notation: "A-B-C-D-E running",
        pivots: 6,
        subdivisions: T33333,
        rules: &["triangle_core", "wave_b_exceeds_a_origin"],
        guidelines: &["common_running_variation"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::DoubleThree,
        family: EwaPrimitiveFamily::Corrective,
        notation: "W-X-Y",
        pivots: 10,
        subdivisions: COMBO7,
        rules: &["w_and_y_are_corrective", "x_is_corrective", "at_most_one_zigzag", "triangle_only_final_component"],
        guidelines: &["extends_time_sideways"],
    },
    EwaPrimitiveSpec {
        kind: EwaPatternKind::TripleThree,
        family: EwaPrimitiveFamily::Corrective,
        notation: "W-X-Y-X-Z",
        pivots: 16,
        subdivisions: COMBO11,
        rules: &["w_y_z_are_corrective", "x_waves_are_corrective"],
        guidelines: &["theoretical_or_extremely_rare"],
    },
];
