// Session Effect: simple 4-bucket (Asia/Europe/US/Overnight) mean returns accumulators

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SessionEffect {
    counts: [usize; 4],
    sums: [f64; 4],
    last_close: Option<f64>,
    pub last_bucket: usize,
    pub mean_returns: [f64; 4],
}

impl Default for SessionEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionEffect {
    pub fn new() -> Self {
        Self {
            counts: [0; 4],
            sums: [0.0; 4],
            last_close: None,
            last_bucket: 0,
            mean_returns: [0.0; 4],
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.counts = [0; 4];
        self.sums = [0.0; 4];
        self.last_close = None;
        self.last_bucket = 0;
        self.mean_returns = [0.0; 4];
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.counts.iter().any(|&c| c > 0)
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.mean_returns[self.last_bucket])
    }

    // Caller supplies session bucket 0..3
    pub fn update_with_session(&mut self, close: f64, bucket: u8) {
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            let b = (bucket as usize).min(3);
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
    fn test_session_effect_creation() {
        let se = SessionEffect::new();
        assert!(!se.is_ready());
    }

    #[test]
    fn test_session_effect_update() {
        let mut se = SessionEffect::new();
        se.update_with_session(100.0, 0);
        se.update_with_session(101.0, 0);
        assert!(se.is_ready());
    }

    #[test]
    fn test_session_effect_buckets() {
        let mut se = SessionEffect::new();
        for i in 0..8 {
            let price = 100.0 + i as f64;
            let bucket = (i % 4) as u8;
            se.update_with_session(price, bucket);
        }
        // Check all buckets have been updated
        for mean in &se.mean_returns {
            assert!(mean.is_finite());
        }
    }

    #[test]
    fn test_session_effect_reset() {
        let mut se = SessionEffect::new();
        se.update_with_session(100.0, 0);
        se.update_with_session(101.0, 0);
        se.reset();
        assert!(!se.is_ready());
    }
}
