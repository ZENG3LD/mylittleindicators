//! Lightweight calendar utilities to derive date parts and simple flags from UNIX timestamp (UTC)

pub struct CalendarService;

impl CalendarService {
    #[inline]
    pub fn weekday_from_timestamp(unix_secs: i64) -> u32 {
        // 0=Mon..6=Sun
        // Unix epoch 1970-01-01 was Thursday (weekday=3 by Mon=0 convention)
        let days = unix_secs.div_euclid(86_400);
        let weekday = (days + 3).rem_euclid(7);
        ((weekday + 6) % 7) as u32 // convert 0=Thu.. to 0=Mon..
    }

    #[inline]
    pub fn ymd_from_timestamp(unix_secs: i64) -> (i32, u32, u32) {
        // Simple civil-from-days algorithm (Howard Hinnant)
        let z = unix_secs.div_euclid(86_400) + 719468; // days since 0000-03-01
        let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
        let doe = z - era * 146097; // [0, 146096]
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
        let y = (yoe as i32) + era as i32 * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
        let mp = (5 * doy + 2) / 153; // [0, 11]
        let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
        let m = mp + if mp < 10 { 3 } else { -9 }; // [1, 12]
        let year = y + (m <= 2) as i32;
        (year, m as u32, d as u32)
    }

    #[inline]
    pub fn day_of_year(unix_secs: i64) -> u32 {
        let (y, m, d) = Self::ymd_from_timestamp(unix_secs);
        Self::doy(y, m, d)
    }

    #[inline]
    pub fn week_of_quarter(unix_secs: i64) -> u32 {
        let (y, m, d) = Self::ymd_from_timestamp(unix_secs);
        let q_start_month = ((m - 1) / 3) * 3 + 1;
        let (_y1, m1) = (y, q_start_month);
        let first_doy_quarter = Self::doy(y, m1, 1);
        let doy = Self::doy(y, m, d);
        ((doy - first_doy_quarter) / 7) + 1
    }

    #[inline]
    pub fn is_start_of_month(_unix_secs: i64, window_days: u32) -> bool {
        let (_y, _m, d) = Self::ymd_from_timestamp(_unix_secs);
        d <= window_days.max(1)
    }

    #[inline]
    pub fn is_end_of_month(unix_secs: i64, window_days: u32) -> bool {
        let (y, m, d) = Self::ymd_from_timestamp(unix_secs);
        let dim = Self::days_in_month(y, m);
        (dim - d + 1) <= window_days.max(1)
    }

    #[inline]
    pub fn is_start_of_week(unix_secs: i64, window_days: u32) -> bool {
        let w = Self::weekday_from_timestamp(unix_secs);
        w <= window_days.min(3)
    }

    #[inline]
    pub fn is_end_of_week(unix_secs: i64, window_days: u32) -> bool {
        let w = Self::weekday_from_timestamp(unix_secs);
        (6 - w) <= window_days.min(3)
    }

    #[inline]
    pub fn days_in_month(year: i32, month: u32) -> u32 {
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

    #[inline]
    pub fn is_leap(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    #[inline]
    fn doy(year: i32, month: u32, day: u32) -> u32 {
        const MD: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let mut s = 0;
        for &days in MD[..(month - 1) as usize].iter() {
            s += days;
        }
        s + day
            + if month > 2 && Self::is_leap(year) {
                1
            } else {
                0
            }
    }
}
