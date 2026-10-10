pub mod swing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FibRelationKind {
    Retracement,
    Extension,
}

#[derive(Debug, Clone)]
pub struct FibRelation {
    pub previous_segment: usize,
    pub current_segment: usize,
    pub kind: FibRelationKind,
    pub ratio: f64,
    pub nearest_target: f64,
    pub error: f64,
}
