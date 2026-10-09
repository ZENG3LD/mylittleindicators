//! Time service utilities for timestamp manipulation

/// Service for time and bar synchronization operations
pub struct TimeService;

impl TimeService {
    /// Align timestamp to the start of period for a given timeframe
    #[inline]
    pub fn align_to_period_start(timestamp: i64, period_seconds: i64) -> i64 {
        (timestamp / period_seconds) * period_seconds
    }

    /// Get start of minute for a given timestamp
    #[inline]
    pub fn get_minute_start(timestamp: i64) -> i64 {
        Self::align_to_period_start(timestamp, 60)
    }

    /// Get start of hour for a given timestamp
    #[inline]
    pub fn get_hour_start(timestamp: i64) -> i64 {
        Self::align_to_period_start(timestamp, 3600)
    }

    /// Get start of day for a given timestamp
    #[inline]
    pub fn get_day_start(timestamp: i64) -> i64 {
        Self::align_to_period_start(timestamp, 86400)
    }

    /// Check if two timestamps belong to the same period
    #[inline]
    pub fn same_period(ts1: i64, ts2: i64, period_seconds: i64) -> bool {
        Self::align_to_period_start(ts1, period_seconds)
            == Self::align_to_period_start(ts2, period_seconds)
    }

    /// Check if two timestamps belong to the same minute
    #[inline]
    pub fn same_minute(ts1: i64, ts2: i64) -> bool {
        Self::same_period(ts1, ts2, 60)
    }

    /// Check if timestamp is start of minute
    #[inline]
    pub fn is_minute_start(timestamp: i64) -> bool {
        timestamp % 60 == 0
    }

    /// Get next minute start after given timestamp
    #[inline]
    pub fn next_minute_start(timestamp: i64) -> i64 {
        Self::get_minute_start(timestamp) + 60
    }

    /// Get previous minute start before given timestamp
    #[inline]
    pub fn prev_minute_start(timestamp: i64) -> i64 {
        Self::get_minute_start(timestamp) - 60
    }

    /// Get hour of day (0-23) in UTC from timestamp
    #[inline]
    pub fn hour_utc(timestamp: i64) -> u32 {
        ((timestamp % 86400) / 3600) as u32
    }
}
