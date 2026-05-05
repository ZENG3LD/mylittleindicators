// Start/End of Quarter flags using calendar service

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::types::CalendarService;

#[derive(Clone)]
pub struct StartEndOfQuarterFlags {
    window_days: u32,
    pub start_flag: f64,
    pub end_flag: f64,
}

impl StartEndOfQuarterFlags {
    pub fn new(window_days: u32) -> Self {
        Self {
            window_days: window_days.clamp(1, 7),
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

    pub fn update_with_timestamp(&mut self, unix_secs: i64) -> (f64, f64) {
        let (y, m, d) = CalendarService::ymd_from_timestamp(unix_secs);
        let q_start_month = ((m - 1) / 3) * 3 + 1;
        let q_end_month = q_start_month + 2;
        let start = m == q_start_month && d <= self.window_days;
        let end =
            m == q_end_month && (CalendarService::days_in_month(y, m) - d + 1) <= self.window_days;
        self.start_flag = if start { 1.0 } else { 0.0 };
        self.end_flag = if end { 1.0 } else { 0.0 };
        (self.start_flag, self.end_flag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_end_of_quarter_flags_creation() {
        let flags = StartEndOfQuarterFlags::new(5);
        assert!(flags.is_ready());
    }

    #[test]
    fn test_start_end_of_quarter_flags_update() {
        let mut flags = StartEndOfQuarterFlags::new(5);
        // 1704067200 is 2024-01-01 00:00:00 UTC
        let (start, end) = flags.update_with_timestamp(1704067200);
        assert_eq!(start, 1.0, "Jan 1 should be start of quarter");
        assert_eq!(end, 0.0);
    }

    #[test]
    fn test_start_end_of_quarter_flags_range() {
        let mut flags = StartEndOfQuarterFlags::new(5);
        // Test several timestamps throughout Q1
        for day in 0..90 {
            let ts = 1704067200 + day * 86400;
            let (start, end) = flags.update_with_timestamp(ts);
            assert!(start == 0.0 || start == 1.0);
            assert!(end == 0.0 || end == 1.0);
        }
    }

    #[test]
    fn test_start_end_of_quarter_flags_reset() {
        let mut flags = StartEndOfQuarterFlags::new(5);
        flags.update_with_timestamp(1704067200);
        flags.reset();
        assert_eq!(flags.start_flag, 0.0);
        assert_eq!(flags.end_flag, 0.0);
    }
}
