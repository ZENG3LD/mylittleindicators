// Holiday/Weekend proximity effect: simple proximity to weekend (Fri/Mon) as proxy; holidays TBD

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct HolidayWeekendProximityEffect {
    window: u32,
    value: f64,
}

impl HolidayWeekendProximityEffect {
    pub fn new(window_days: u32) -> Self {
        Self {
            window: window_days.clamp(1, 5),
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

    // weekday: 0=Mon..6=Sun
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64, weekday: u32) -> f64 {
        let dist_to_weekend = match weekday {
            0 => 1,
            1 => 2,
            2 => 3,
            3 => 2,
            4 => 1,
            5 => 0,
            6 => 0,
            _ => 3,
        } as u32;
        self.value = if dist_to_weekend <= self.window {
            1.0 - (dist_to_weekend as f64 / self.window as f64)
        } else {
            0.0
        };
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_holiday_weekend_proximity_creation() {
        let hwp = HolidayWeekendProximityEffect::new(3);
        assert!(hwp.is_ready());
    }

    #[test]
    fn test_holiday_weekend_proximity_friday() {
        let mut hwp = HolidayWeekendProximityEffect::new(3);
        // Friday (weekday=4) should be close to weekend
        let value = hwp.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 4);
        assert!(value > 0.5, "Friday should be close to weekend");
    }

    #[test]
    fn test_holiday_weekend_proximity_saturday() {
        let mut hwp = HolidayWeekendProximityEffect::new(3);
        // Saturday (weekday=5) is weekend
        let value = hwp.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 5);
        assert_eq!(value, 1.0, "Saturday should be exactly weekend");
    }

    #[test]
    fn test_holiday_weekend_proximity_range() {
        let mut hwp = HolidayWeekendProximityEffect::new(3);
        for weekday in 0..7 {
            let value = hwp.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, weekday);
            assert!(value >= 0.0 && value <= 1.0, "Value should be in [0, 1]");
        }
    }

    #[test]
    fn test_holiday_weekend_proximity_reset() {
        let mut hwp = HolidayWeekendProximityEffect::new(3);
        hwp.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 4);
        hwp.reset();
        assert_eq!(hwp.value, 0.0);
    }
}
