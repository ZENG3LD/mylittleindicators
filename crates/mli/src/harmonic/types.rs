#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HarmonicKind {
    Abcd,
    Xabcd,
    Cypher,
    Gartley,
    Bat,
    Butterfly,
    Crab,
    Shark,
    ThreeDrives,
    AltBat,
    DeepCrab,
    FiveZero,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HarmonicHit {
    pub kind: HarmonicKind,
    pub pivot_indices: Vec<usize>,
    pub score: f64,
}
