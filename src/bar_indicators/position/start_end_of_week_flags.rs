// Start/End of week flags within N-day window (Mon..Sun)

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct StartEndOfWeekFlags {
    window_days: u32,
    pub start_flag: f64,
    pub end_flag: f64,
}

impl StartEndOfWeekFlags {
    pub fn new(window_days: u32) -> Self {
        Self {
            window_days: window_days.clamp(1, 3),
            start_flag: 0.0,
            end_flag: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.start_flag = 0.0;
        self.end_flag = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.start_flag, self.end_flag)
    }

    // weekday: 0=Mon..6=Sun
    pub fn update_bar(
        &mut self,
        _o: f64,
        _h: f64,
        _l: f64,
        _c: f64,
        _v: f64,
        weekday: u32,
    ) -> (f64, f64) {
        let dist_start = weekday.min(6);
        let dist_end = 6 - weekday.min(6);
        self.start_flag = if dist_start <= self.window_days {
            1.0
        } else {
            0.0
        };
        self.end_flag = if dist_end <= self.window_days {
            1.0
        } else {
            0.0
        };
        (self.start_flag, self.end_flag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_end_of_week_flags_creation() {
        let flags = StartEndOfWeekFlags::new(1);
        assert!(flags.is_ready());
    }

    #[test]
    fn test_start_end_of_week_monday() {
        let mut flags = StartEndOfWeekFlags::new(1);
        // Monday is weekday 0
        let (start, end) = flags.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 0);
        assert_eq!(start, 1.0, "Monday should be start of week");
        assert_eq!(end, 0.0);
    }

    #[test]
    fn test_start_end_of_week_sunday() {
        let mut flags = StartEndOfWeekFlags::new(1);
        // Sunday is weekday 6
        let (start, end) = flags.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 6);
        assert_eq!(start, 0.0);
        assert_eq!(end, 1.0, "Sunday should be end of week");
    }

    #[test]
    fn test_start_end_of_week_mid_week() {
        let mut flags = StartEndOfWeekFlags::new(1);
        // Wednesday is weekday 2
        let (start, end) = flags.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 2);
        assert_eq!(start, 0.0);
        assert_eq!(end, 0.0);
    }

    #[test]
    fn test_start_end_of_week_reset() {
        let mut flags = StartEndOfWeekFlags::new(1);
        flags.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 0);
        flags.reset();
        assert_eq!(flags.start_flag, 0.0);
        assert_eq!(flags.end_flag, 0.0);
    }
}
