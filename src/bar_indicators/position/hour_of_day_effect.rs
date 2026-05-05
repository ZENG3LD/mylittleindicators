// Hour-of-day effect (0..23) using TimeService

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::types::TimeService;

#[derive(Clone)]
pub struct HourOfDayEffect {
    pub value: f64,
}

impl Default for HourOfDayEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl HourOfDayEffect {
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
        let h = TimeService::hour_utc(unix_secs) as f64;
        self.value = h;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hour_of_day_effect_creation() {
        let hod = HourOfDayEffect::new();
        assert!(hod.is_ready());
        assert_eq!(hod.value, 0.0);
    }

    #[test]
    fn test_hour_of_day_effect_update() {
        let mut hod = HourOfDayEffect::new();
        // 1700000000 is ~2023-11-14 22:13:20 UTC
        let value = hod.update_with_timestamp(1700000000);
        assert!(value >= 0.0 && value < 24.0, "Hour should be in [0, 24)");
    }

    #[test]
    fn test_hour_of_day_effect_range() {
        let mut hod = HourOfDayEffect::new();
        for hour in 0..24 {
            let ts = 1700000000 + hour * 3600;
            let value = hod.update_with_timestamp(ts);
            assert!(value >= 0.0 && value < 24.0, "Hour should be in [0, 24)");
        }
    }

    #[test]
    fn test_hour_of_day_effect_reset() {
        let mut hod = HourOfDayEffect::new();
        hod.update_with_timestamp(1700000000);
        hod.reset();
        assert_eq!(hod.value, 0.0);
    }
}
