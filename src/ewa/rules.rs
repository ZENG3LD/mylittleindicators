use super::types::EwaSegmentDirection;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RatioBand {
    pub min: f64,
    pub max: f64,
}

impl RatioBand {
    pub const fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    pub fn contains(self, value: f64) -> bool {
        value.is_finite() && value >= self.min && value <= self.max
    }
}

pub const ZIGZAG_B_A: RatioBand = RatioBand::new(0.0, 0.899_999);
pub const FLAT_B_A: RatioBand = RatioBand::new(0.9, f64::INFINITY);
pub const REGULAR_FLAT_B_A: RatioBand = RatioBand::new(0.9, 1.05);
pub const EXPANDED_FLAT_B_A: RatioBand = RatioBand::new(1.000_001, f64::INFINITY);

pub fn ratio(numerator: f64, denominator: f64) -> Option<f64> {
    if !numerator.is_finite() || !denominator.is_finite() || denominator.abs() <= f64::EPSILON {
        return None;
    }
    Some(numerator.abs() / denominator.abs())
}

pub fn beyond(origin: f64, reference: f64, test: f64) -> bool {
    if reference >= origin {
        test > reference
    } else {
        test < reference
    }
}

pub fn beyond_origin(origin: f64, reference: f64, test: f64) -> bool {
    if reference >= origin {
        test < origin
    } else {
        test > origin
    }
}

pub fn not_beyond_origin(origin: f64, reference: f64, test: f64) -> bool {
    !beyond_origin(origin, reference, test)
}

pub fn progresses(direction: EwaSegmentDirection, previous: f64, next: f64) -> bool {
    match direction {
        EwaSegmentDirection::Up => next > previous,
        EwaSegmentDirection::Down => next < previous,
    }
}

pub fn retracement_stays_above_origin(
    direction: EwaSegmentDirection,
    origin: f64,
    retracement_end: f64,
) -> bool {
    match direction {
        EwaSegmentDirection::Up => retracement_end > origin,
        EwaSegmentDirection::Down => retracement_end < origin,
    }
}
