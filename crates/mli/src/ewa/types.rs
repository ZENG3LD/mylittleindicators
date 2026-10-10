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
            | Self::RunningFlat => 4,
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
        }
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
