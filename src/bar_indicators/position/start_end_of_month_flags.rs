// Start/End of Month binary flags within N-day window

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct StartEndOfMonthFlags {
    window_days: u32,
    pub start_flag: f64,
    pub end_flag: f64,
}

impl StartEndOfMonthFlags {
    pub fn new(window_days: u32) -> Self {
        Self {
            window_days: window_days.clamp(1, 5),
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
    ) -> (f64, f64) {
        let dim = Self::days_in_month(year, month);
        let dist_start = (day as i32 - 1).unsigned_abs();
        let dist_end = (dim as i32 - day as i32).unsigned_abs();
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
    fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            _ => {
                if Self::is_leap(year) {
                    29
                } else {
                    28
                }
            }
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
    fn test_start_end_of_month_flags_creation() {
        let flags = StartEndOfMonthFlags::new(3);
        assert!(flags.is_ready());
    }

    #[test]
    fn test_start_end_of_month_start_flag() {
        let mut flags = StartEndOfMonthFlags::new(3);
        let (start, end) = flags.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 2024, 1, 1);
        assert_eq!(start, 1.0, "Day 1 should be start of month");
        assert_eq!(end, 0.0);
    }

    #[test]
    fn test_start_end_of_month_end_flag() {
        let mut flags = StartEndOfMonthFlags::new(3);
        let (start, end) = flags.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 2024, 1, 31);
        assert_eq!(start, 0.0);
        assert_eq!(end, 1.0, "Day 31 should be end of month");
    }

    #[test]
    fn test_start_end_of_month_mid_month() {
        let mut flags = StartEndOfMonthFlags::new(3);
        let (start, end) = flags.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 2024, 1, 15);
        assert_eq!(start, 0.0);
        assert_eq!(end, 0.0);
    }

    #[test]
    fn test_start_end_of_month_reset() {
        let mut flags = StartEndOfMonthFlags::new(3);
        flags.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0, 2024, 1, 1);
        flags.reset();
        assert_eq!(flags.start_flag, 0.0);
        assert_eq!(flags.end_flag, 0.0);
    }
}
