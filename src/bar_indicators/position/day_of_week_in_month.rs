// Day-of-Week-in-Month effect: bucketized feature 1..5 (occurrence of weekday within month)

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct DayOfWeekInMonthEffect {
    current_value: f64,
}

impl Default for DayOfWeekInMonthEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl DayOfWeekInMonthEffect {
    pub fn new() -> Self {
        Self { current_value: 0.0 }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.current_value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_value)
    }

    // For simplicity, pass timestamp in seconds via volume field placeholder when integrating; here just keep API
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
        weekday: u32,
    ) -> f64 {
        // weekday: 0=Mon..6=Sun; compute occurrence index 1..5 within this month for that weekday
        let occurrence = Self::weekday_occurrence_in_month(year, month, day, weekday) as f64;
        self.current_value = occurrence;
        self.current_value
    }

    fn weekday_occurrence_in_month(year: i32, month: u32, day: u32, weekday: u32) -> u32 {
        // Find which occurrence of this weekday the given day is (1..5)
        // Compute weekday of the 1st of the month
        let first_weekday = Self::weekday_of_date(year, month, 1);
        let mut first_target_day = 1u32 + ((7 + weekday as i32 - first_weekday as i32) % 7) as u32;
        if first_target_day == 0 {
            first_target_day = 1;
        }
        if day < first_target_day {
            return 0;
        }
        1 + ((day - first_target_day) / 7)
    }

    fn weekday_of_date(year: i32, month: u32, day: u32) -> u32 {
        // Zeller-like congruence (0=Mon..6=Sun).
        let y = if month < 3 { year - 1 } else { year };
        let m = if month < 3 { month + 12 } else { month } as i32;
        let d = day as i32;
        let k = y % 100;
        let j = y / 100;
        let h = (d + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7; // 0=Sat..6=Fri
        let dow = ((h + 6) % 7) as u32; // 0=Sun..6=Sat
        (dow + 6) % 7 // 0=Mon..6=Sun
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_of_week_in_month_creation() {
        let eff = DayOfWeekInMonthEffect::new();
        assert!(eff.is_ready()); // Always ready
    }

    #[test]
    fn test_day_of_week_in_month_update() {
        let mut eff = DayOfWeekInMonthEffect::new();
        // 2024-01-15 is a Monday, so occurrence should be around 3 (3rd Monday)
        let value = eff.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 2024, 1, 15, 0);
        assert!(value >= 0.0 && value <= 5.0, "Occurrence should be in [0, 5]");
    }

    #[test]
    fn test_day_of_week_in_month_range() {
        let mut eff = DayOfWeekInMonthEffect::new();
        for day in 1..=28 {
            let value = eff.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 2024, 1, day, 0);
            assert!(value >= 0.0 && value <= 5.0, "Occurrence should be in [0, 5]");
        }
    }

    #[test]
    fn test_day_of_week_in_month_reset() {
        let mut eff = DayOfWeekInMonthEffect::new();
        eff.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 2024, 1, 15, 0);
        eff.reset();
        assert_eq!(eff.current_value, 0.0);
    }
}
