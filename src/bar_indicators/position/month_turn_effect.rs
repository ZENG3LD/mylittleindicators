// Start/End-of-Month effect: windowed proximity score to month turn

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct MonthTurnEffect {
    window: u32,
    value: f64,
}

impl MonthTurnEffect {
    pub fn new(window_days: u32) -> Self {
        Self {
            window: window_days.clamp(1, 10),
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
        let days_in_month = Self::days_in_month(year, month);
        let dist_start = (day as i32 - 1).unsigned_abs();
        let dist_end = (days_in_month as i32 - day as i32).unsigned_abs();
        let near = dist_start.min(dist_end);
        self.value = if near <= self.window {
            1.0 - (near as f64 / self.window as f64)
        } else {
            0.0
        };
        self.value
    }

    fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if Self::is_leap(year) {
                    29
                } else {
                    28
                }
            }
            _ => 30,
        }
    }
    fn is_leap(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_month_turn_effect_creation() {
        let mte = MonthTurnEffect::new(3);
        assert!(mte.is_ready());
    }

    #[test]
    fn test_month_turn_effect_start_of_month() {
        let mut mte = MonthTurnEffect::new(3);
        // Day 1 is start of month
        let value = mte.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 2024, 1, 1);
        assert_eq!(value, 1.0, "Day 1 should be at start of month");
    }

    #[test]
    fn test_month_turn_effect_end_of_month() {
        let mut mte = MonthTurnEffect::new(3);
        // Day 31 is end of month for January
        let value = mte.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 2024, 1, 31);
        assert_eq!(value, 1.0, "Day 31 should be at end of month");
    }

    #[test]
    fn test_month_turn_effect_mid_month() {
        let mut mte = MonthTurnEffect::new(3);
        // Day 15 is middle of month
        let value = mte.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 2024, 1, 15);
        assert_eq!(value, 0.0, "Day 15 should be far from turn");
    }

    #[test]
    fn test_month_turn_effect_reset() {
        let mut mte = MonthTurnEffect::new(3);
        mte.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 2024, 1, 1);
        mte.reset();
        assert_eq!(mte.value, 0.0);
    }
}
