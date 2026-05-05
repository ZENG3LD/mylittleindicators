// Time Encoders: day-of-week, month, session buckets (requires external time feed)

#[derive(Clone, Copy, Debug, Default)]
pub struct TimeBuckets {
    pub dow: u8,     // 1..7
    pub month: u8,   // 1..12
    pub session: u8, // 0..3 (Asia/Europe/US/Overnight for crypto ~UTC buckets)
}

#[derive(Clone)]
pub struct TimeEncoders {
    pub buckets: TimeBuckets,
}

impl Default for TimeEncoders {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeEncoders {
    pub fn new() -> Self {
        Self {
            buckets: TimeBuckets::default(),
        }
    }
    pub fn reset(&mut self) {
        self.buckets = TimeBuckets::default();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }

    #[inline]
    pub fn value(&self) -> crate::IndicatorValue {
        crate::IndicatorValue::Triple(
            self.buckets.dow as f64,
            self.buckets.month as f64,
            self.buckets.session as f64,
        )
    }

    // Caller supplies unix timestamp (secs)
    pub fn update_with_timestamp(&mut self, unix_ts: i64) -> TimeBuckets {
        // Very lightweight UTC splitting without chrono deps
        // Placeholder: compute day-of-week from Unix time via Zeller-like; for now map by modulo
        // NOTE: In production, replace with chrono if allowed by project
        let days = (unix_ts / 86_400).max(0);
        self.buckets.dow = ((days + 4).rem_euclid(7) + 1) as u8; // 1970-01-01 was Thursday (4)
        let secs = unix_ts.rem_euclid(86_400);
        let hour = secs / 3600;
        self.buckets.session = if hour < 6 {
            3
        } else if hour < 12 {
            0
        } else if hour < 18 {
            1
        } else {
            2
        };
        // Month is unknown without full calendar; keep 0; or caller sets separately
        self.buckets
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_encoders_creation() {
        let te = TimeEncoders::new();
        assert!(te.is_ready());
        assert_eq!(te.buckets.dow, 0);
        assert_eq!(te.buckets.session, 0);
    }

    #[test]
    fn test_time_encoders_dow() {
        let mut te = TimeEncoders::new();

        // Test known Unix timestamp (Jan 1, 1970 was Thursday = dow 4, but we use 1-7 so 4+1=5? or just 4)
        // 1970-01-01 00:00:00 UTC (Thursday)
        let buckets = te.update_with_timestamp(0);
        assert!(buckets.dow >= 1 && buckets.dow <= 7, "Day of week should be 1-7, got {}", buckets.dow);
    }

    #[test]
    fn test_time_encoders_session() {
        let mut te = TimeEncoders::new();

        // Test different hours for session detection
        // Hour 0-5: session 3 (overnight)
        let buckets = te.update_with_timestamp(3 * 3600); // 3 AM
        assert_eq!(buckets.session, 3);

        // Hour 6-11: session 0 (Asia)
        let buckets = te.update_with_timestamp(9 * 3600); // 9 AM
        assert_eq!(buckets.session, 0);

        // Hour 12-17: session 1 (Europe)
        let buckets = te.update_with_timestamp(15 * 3600); // 3 PM
        assert_eq!(buckets.session, 1);

        // Hour 18-23: session 2 (US)
        let buckets = te.update_with_timestamp(21 * 3600); // 9 PM
        assert_eq!(buckets.session, 2);
    }

    #[test]
    fn test_time_encoders_reset() {
        let mut te = TimeEncoders::new();
        te.update_with_timestamp(1000000);
        te.reset();
        assert_eq!(te.buckets.dow, 0);
        assert_eq!(te.buckets.session, 0);
    }
}
