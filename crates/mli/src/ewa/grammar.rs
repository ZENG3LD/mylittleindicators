
use super::types::EwaPatternKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EwaWavePosition {
    Root,
    Wave1,
    Wave2,
    Wave3,
    Wave4,
    Wave5,
    A,
    B,
    C,
    D,
    E,
    W,
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EwaGrammarClass {
    Motive,
    MotiveOrCorrective,
    Corrective,
    SimpleCorrection,
    Zigzag,
    Triangle,
}

#[derive(Debug, Clone, Copy)]
pub struct EwaGrammarSlot {
    pub position: EwaWavePosition,
    pub class: EwaGrammarClass,
}

pub const IMPULSE_SLOTS: &[EwaGrammarSlot] = &[
    EwaGrammarSlot { position: EwaWavePosition::Wave1, class: EwaGrammarClass::Motive },
    EwaGrammarSlot { position: EwaWavePosition::Wave2, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::Wave3, class: EwaGrammarClass::Motive },
    EwaGrammarSlot { position: EwaWavePosition::Wave4, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::Wave5, class: EwaGrammarClass::Motive },
];

pub const LEADING_DIAGONAL_SLOTS: &[EwaGrammarSlot] = &[
    EwaGrammarSlot { position: EwaWavePosition::Wave1, class: EwaGrammarClass::MotiveOrCorrective },
    EwaGrammarSlot { position: EwaWavePosition::Wave2, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::Wave3, class: EwaGrammarClass::MotiveOrCorrective },
    EwaGrammarSlot { position: EwaWavePosition::Wave4, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::Wave5, class: EwaGrammarClass::MotiveOrCorrective },
];

pub const ENDING_DIAGONAL_SLOTS: &[EwaGrammarSlot] = &[
    EwaGrammarSlot { position: EwaWavePosition::Wave1, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::Wave2, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::Wave3, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::Wave4, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::Wave5, class: EwaGrammarClass::Corrective },
];

pub const ZIGZAG_SLOTS: &[EwaGrammarSlot] = &[
    EwaGrammarSlot { position: EwaWavePosition::A, class: EwaGrammarClass::Motive },
    EwaGrammarSlot { position: EwaWavePosition::B, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::C, class: EwaGrammarClass::Motive },
];

pub const FLAT_SLOTS: &[EwaGrammarSlot] = &[
    EwaGrammarSlot { position: EwaWavePosition::A, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::B, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::C, class: EwaGrammarClass::Motive },
];

pub const TRIANGLE_SLOTS: &[EwaGrammarSlot] = &[
    EwaGrammarSlot { position: EwaWavePosition::A, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::B, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::C, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::D, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::E, class: EwaGrammarClass::Corrective },
];

pub const DOUBLE_CORRECTION_SLOTS: &[EwaGrammarSlot] = &[
    EwaGrammarSlot { position: EwaWavePosition::W, class: EwaGrammarClass::SimpleCorrection },
    EwaGrammarSlot { position: EwaWavePosition::X, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::Y, class: EwaGrammarClass::SimpleCorrection },
];

pub const DOUBLE_ZIGZAG_SLOTS: &[EwaGrammarSlot] = &[
    EwaGrammarSlot { position: EwaWavePosition::W, class: EwaGrammarClass::Zigzag },
    EwaGrammarSlot { position: EwaWavePosition::X, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::Y, class: EwaGrammarClass::Zigzag },
];

pub const TRIPLE_CORRECTION_SLOTS: &[EwaGrammarSlot] = &[
    EwaGrammarSlot { position: EwaWavePosition::W, class: EwaGrammarClass::SimpleCorrection },
    EwaGrammarSlot { position: EwaWavePosition::X, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::Y, class: EwaGrammarClass::SimpleCorrection },
    EwaGrammarSlot { position: EwaWavePosition::X, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::Z, class: EwaGrammarClass::SimpleCorrection },
];

pub const TRIPLE_ZIGZAG_SLOTS: &[EwaGrammarSlot] = &[
    EwaGrammarSlot { position: EwaWavePosition::W, class: EwaGrammarClass::Zigzag },
    EwaGrammarSlot { position: EwaWavePosition::X, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::Y, class: EwaGrammarClass::Zigzag },
    EwaGrammarSlot { position: EwaWavePosition::X, class: EwaGrammarClass::Corrective },
    EwaGrammarSlot { position: EwaWavePosition::Z, class: EwaGrammarClass::Zigzag },
];

pub fn slots_for(pattern: EwaPatternKind) -> &'static [EwaGrammarSlot] {
    match pattern {
        EwaPatternKind::Impulse
        | EwaPatternKind::ImpulseExtendedWave1
        | EwaPatternKind::ImpulseExtendedWave3
        | EwaPatternKind::ImpulseExtendedWave5
        | EwaPatternKind::TruncatedImpulse => IMPULSE_SLOTS,
        EwaPatternKind::LeadingDiagonalContracting
        | EwaPatternKind::LeadingDiagonalExpanding => LEADING_DIAGONAL_SLOTS,
        EwaPatternKind::EndingDiagonalContracting
        | EwaPatternKind::EndingDiagonalExpanding => ENDING_DIAGONAL_SLOTS,
        EwaPatternKind::Zigzag | EwaPatternKind::RunningZigzag => ZIGZAG_SLOTS,
        EwaPatternKind::Flat
        | EwaPatternKind::RegularFlat
        | EwaPatternKind::ExpandedFlat
        | EwaPatternKind::RunningFlat
        | EwaPatternKind::Correction => FLAT_SLOTS,
        EwaPatternKind::Triangle
        | EwaPatternKind::ContractingTriangle
        | EwaPatternKind::BarrierTriangle
        | EwaPatternKind::ExpandingTriangle
        | EwaPatternKind::RunningTriangle => TRIANGLE_SLOTS,
        EwaPatternKind::DoubleZigzag => DOUBLE_ZIGZAG_SLOTS,
        EwaPatternKind::DoubleThree | EwaPatternKind::DoubleCombo => DOUBLE_CORRECTION_SLOTS,
        EwaPatternKind::TripleZigzag => TRIPLE_ZIGZAG_SLOTS,
        EwaPatternKind::TripleThree | EwaPatternKind::TripleCombo => TRIPLE_CORRECTION_SLOTS,
        _ => &[],
    }
}

pub fn matches_class(pattern: EwaPatternKind, class: EwaGrammarClass) -> bool {
    match class {
        EwaGrammarClass::Motive => is_motive(pattern),
        EwaGrammarClass::MotiveOrCorrective => is_motive(pattern) || is_corrective(pattern),
        EwaGrammarClass::Corrective => is_corrective(pattern),
        EwaGrammarClass::SimpleCorrection => is_simple_correction(pattern),
        EwaGrammarClass::Zigzag => is_zigzag(pattern),
        EwaGrammarClass::Triangle => is_triangle(pattern),
    }
}

pub fn is_motive(pattern: EwaPatternKind) -> bool {
    matches!(
        pattern,
        EwaPatternKind::Impulse
            | EwaPatternKind::ImpulseExtendedWave1
            | EwaPatternKind::ImpulseExtendedWave3
            | EwaPatternKind::ImpulseExtendedWave5
            | EwaPatternKind::TruncatedImpulse
            | EwaPatternKind::LeadingDiagonalContracting
            | EwaPatternKind::LeadingDiagonalExpanding
            | EwaPatternKind::EndingDiagonalContracting
            | EwaPatternKind::EndingDiagonalExpanding
    )
}

pub fn is_zigzag(pattern: EwaPatternKind) -> bool {
    matches!(pattern, EwaPatternKind::Zigzag | EwaPatternKind::RunningZigzag)
}

pub fn is_flat(pattern: EwaPatternKind) -> bool {
    matches!(
        pattern,
        EwaPatternKind::Flat
            | EwaPatternKind::RegularFlat
            | EwaPatternKind::ExpandedFlat
            | EwaPatternKind::RunningFlat
    )
}

pub fn is_triangle(pattern: EwaPatternKind) -> bool {
    matches!(
        pattern,
        EwaPatternKind::Triangle
            | EwaPatternKind::ContractingTriangle
            | EwaPatternKind::BarrierTriangle
            | EwaPatternKind::ExpandingTriangle
            | EwaPatternKind::RunningTriangle
    )
}

pub fn is_simple_correction(pattern: EwaPatternKind) -> bool {
    is_zigzag(pattern) || is_flat(pattern) || is_triangle(pattern)
}

pub fn is_specific_simple_correction(pattern: EwaPatternKind) -> bool {
    is_zigzag(pattern)
        || matches!(
            pattern,
            EwaPatternKind::RegularFlat
                | EwaPatternKind::ExpandedFlat
                | EwaPatternKind::RunningFlat
                | EwaPatternKind::ContractingTriangle
                | EwaPatternKind::BarrierTriangle
                | EwaPatternKind::ExpandingTriangle
                | EwaPatternKind::RunningTriangle
        )
}

pub fn is_corrective(pattern: EwaPatternKind) -> bool {
    is_simple_correction(pattern)
        || matches!(
            pattern,
            EwaPatternKind::Correction
                | EwaPatternKind::DoubleZigzag
                | EwaPatternKind::TripleZigzag
                | EwaPatternKind::DoubleThree
                | EwaPatternKind::TripleThree
        )
}

pub fn is_scenario_pattern(pattern: EwaPatternKind) -> bool {
    is_motive(pattern)
        || is_specific_simple_correction(pattern)
        || matches!(
            pattern,
            EwaPatternKind::DoubleZigzag
                | EwaPatternKind::TripleZigzag
                | EwaPatternKind::DoubleThree
                | EwaPatternKind::TripleThree
        )
}

pub fn allowed_in_position(pattern: EwaPatternKind, position: EwaWavePosition) -> bool {
    if position == EwaWavePosition::Root {
        return true;
    }
    if matches!(
        pattern,
        EwaPatternKind::LeadingDiagonalContracting | EwaPatternKind::LeadingDiagonalExpanding
    ) {
        return matches!(position, EwaWavePosition::Wave1 | EwaWavePosition::A);
    }
    if matches!(
        pattern,
        EwaPatternKind::EndingDiagonalContracting | EwaPatternKind::EndingDiagonalExpanding
    ) {
        return matches!(position, EwaWavePosition::Wave5 | EwaWavePosition::C);
    }
    if is_triangle(pattern) {
        return matches!(
            position,
            EwaWavePosition::Wave4
                | EwaWavePosition::B
                | EwaWavePosition::X
                | EwaWavePosition::Y
                | EwaWavePosition::Z
        );
    }
    if is_motive(pattern) {
        return matches!(
            position,
            EwaWavePosition::Wave1
                | EwaWavePosition::Wave3
                | EwaWavePosition::Wave5
                | EwaWavePosition::A
                | EwaWavePosition::C
        );
    }
    is_corrective(pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonals_and_triangles_have_strict_positions() {
        assert!(allowed_in_position(
            EwaPatternKind::LeadingDiagonalContracting,
            EwaWavePosition::Wave1
        ));
        assert!(!allowed_in_position(
            EwaPatternKind::LeadingDiagonalContracting,
            EwaWavePosition::Wave5
        ));
        assert!(allowed_in_position(
            EwaPatternKind::ContractingTriangle,
            EwaWavePosition::Wave4
        ));
        assert!(!allowed_in_position(
            EwaPatternKind::ContractingTriangle,
            EwaWavePosition::Wave2
        ));
    }
}
