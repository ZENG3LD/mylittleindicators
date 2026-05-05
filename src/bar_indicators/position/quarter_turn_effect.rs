// Start/End-of-Quarter effect: proximity to quarter boundaries

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct QuarterTurnEffect {
    window: u32,
    value: f64,
}

impl QuarterTurnEffect {
    pub fn new(window_days: u32) -> Self {
        Self {
            window: window_days.clamp(1, 15),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(
        &mut self,
        _o: f64,
        _h: f64,
        _l: f64,
        _c: f64,
        _v: f64,
        year: i32,
        month: u32,
        day: u32,
    ) -> f64 {
        let (q_start_m, q_start_d) = Self::quarter_start(month);
        let (q_end_m, q_end_d) = Self::quarter_end(year, month);
        let dist_start = Self::day_distance(year, month, day, year, q_start_m, q_start_d) as u32;
        let dist_end = Self::day_distance(year, month, day, year, q_end_m, q_end_d) as u32;
        let near = dist_start.min(dist_end);
        self.value = if near <= self.window {
            1.0 - (near as f64 / self.window as f64)
        } else {
            0.0
        };
        self.value
    }

    fn quarter_start(month: u32) -> (u32, u32) {
        match ((month - 1) / 3) + 1 {
            1 => (1, 1),
            2 => (4, 1),
            3 => (7, 1),
            _ => (10, 1),
        }
    }
    fn quarter_end(_year: i32, month: u32) -> (u32, u32) {
        match ((month - 1) / 3) + 1 {
            1 => (3, 31),
            2 => (6, 30),
            3 => (9, 30),
            _ => (12, 31),
        }
    }
    fn day_distance(_y1: i32, _m1: u32, _d1: u32, _y2: i32, _m2: u32, _d2: u32) -> i32 {
        (_d1 as i32 - _d2 as i32).abs() /* approximation by day only */
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quarter_turn_effect_creation() {
        let qte = QuarterTurnEffect::new(5);
        assert!(qte.is_ready());
    }

    #[test]
    fn test_quarter_turn_effect_start_of_quarter() {
        let mut qte = QuarterTurnEffect::new(5);
        // Day 1 of month 1 is start of Q1
        let value = qte.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 2024, 1, 1);
        assert_eq!(value, 1.0, "Day 1 Jan should be at start of quarter");
    }

    #[test]
    fn test_quarter_turn_effect_range() {
        let mut qte = QuarterTurnEffect::new(5);
        for day in 1..=31 {
            let value = qte.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 2024, 1, day);
            assert!(value >= 0.0 && value <= 1.0, "Value should be in [0, 1]");
        }
    }

    #[test]
    fn test_quarter_turn_effect_reset() {
        let mut qte = QuarterTurnEffect::new(5);
        qte.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 2024, 1, 1);
        qte.reset();
        assert_eq!(qte.value, 0.0);
    }
}
