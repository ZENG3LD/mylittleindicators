// Week-in-Month effect: 1..5 bucket of week index within month (using calendar service)

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::types::CalendarService;

#[derive(Clone)]
pub struct WeekInMonthEffect {
    pub value: f64,
}

impl Default for WeekInMonthEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl WeekInMonthEffect {
    pub fn new() -> Self {
        Self { value: 0.0 }
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

    pub fn update_with_timestamp(&mut self, unix_secs: i64) -> f64 {
        let (_y, _m, d) = CalendarService::ymd_from_timestamp(unix_secs);
        let w = ((d - 1) / 7) + 1;
        self.value = w as f64;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_week_in_month_effect_creation() {
        let wim = WeekInMonthEffect::new();
        assert!(wim.is_ready()); // Always ready
        assert_eq!(wim.value, 0.0);
    }

    #[test]
    fn test_week_in_month_effect_range() {
        let mut wim = WeekInMonthEffect::new();
        // Jan 1, 2024 = day 1 = week 1
        let value = wim.update_with_timestamp(1704067200);
        assert!(value >= 1.0 && value <= 5.0, "Week should be in [1, 5]");
    }

    #[test]
    fn test_week_in_month_effect_values() {
        let mut wim = WeekInMonthEffect::new();
        // Test different days of the month
        let ts_base = 1704067200_i64; // Jan 1, 2024
        for day in 0..28 {
            let value = wim.update_with_timestamp(ts_base + day * 86400);
            let expected_week = ((day as u32) / 7) + 1;
            assert_eq!(value as u32, expected_week, "Day {} should be week {}", day + 1, expected_week);
        }
    }

    #[test]
    fn test_week_in_month_effect_reset() {
        let mut wim = WeekInMonthEffect::new();
        wim.update_with_timestamp(1704067200);
        wim.reset();
        assert_eq!(wim.value, 0.0);
    }
}
