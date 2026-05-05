// Weekday Effect: one-hot buckets for weekday (1..7) with rolling mean returns per bucket

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct WeekdayEffect {
    counts: [usize; 7],
    sums: [f64; 7],
    last_close: Option<f64>,
    pub last_bucket: usize,
    pub mean_returns: [f64; 7],
}

impl Default for WeekdayEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl WeekdayEffect {
    pub fn new() -> Self {
        Self {
            counts: [0; 7],
            sums: [0.0; 7],
            last_close: None,
            last_bucket: 0,
            mean_returns: [0.0; 7],
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.counts = [0; 7];
        self.sums = [0.0; 7];
        self.last_close = None;
        self.last_bucket = 0;
        self.mean_returns = [0.0; 7];
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.counts.iter().any(|&c| c > 0)
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.mean_returns[self.last_bucket])
    }

    // Caller must pass weekday in 1..=7 (UTC). For crypto, derive upstream from timestamp.
    pub fn update_with_weekday(&mut self, close: f64, weekday: u8) {
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            let b = ((weekday as usize).saturating_sub(1)).min(6);
            self.counts[b] += 1;
            self.sums[b] += r;
            self.mean_returns[b] = self.sums[b] / self.counts[b] as f64;
            self.last_bucket = b;
        }
        self.last_close = Some(close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weekday_effect_creation() {
        let we = WeekdayEffect::new();
        assert!(!we.is_ready());
        assert_eq!(we.last_bucket, 0);
    }

    #[test]
    fn test_weekday_effect_update() {
        let mut we = WeekdayEffect::new();
        we.update_with_weekday(100.0, 1); // Monday, first call sets last_close
        we.update_with_weekday(101.0, 2); // Tuesday, this computes return
        assert!(we.is_ready());
        assert_eq!(we.last_bucket, 1); // Tuesday = bucket 1
    }

    #[test]
    fn test_weekday_effect_mean_returns() {
        let mut we = WeekdayEffect::new();
        // Simulate multiple days
        for i in 0..14 {
            let price = 100.0 + i as f64;
            let weekday = (i % 7 + 1) as u8;
            we.update_with_weekday(price, weekday);
        }
        // Check mean returns are finite
        for mean in &we.mean_returns {
            assert!(mean.is_finite());
        }
    }

    #[test]
    fn test_weekday_effect_reset() {
        let mut we = WeekdayEffect::new();
        we.update_with_weekday(100.0, 1);
        we.update_with_weekday(101.0, 2);
        we.reset();
        assert!(!we.is_ready());
        assert_eq!(we.last_bucket, 0);
    }
}
