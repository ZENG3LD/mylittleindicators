// Range Compression Burst: detect transitions from compressed ranges (NR) to expansion

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct RangeCompressionBurst {
    window: usize,
    ranges: Vec<f64>,
    idx: usize,
    filled: bool,
    pub compressed: bool,
    pub burst: bool,
}

impl RangeCompressionBurst {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            ranges: vec![0.0; window.max(2)],
            idx: 0,
            filled: false,
            compressed: false,
            burst: false,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ranges.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.compressed = false;
        self.burst = false;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        high: f64,
        low: f64,
        _close: f64,
        _volume: f64,
    ) -> (bool, bool) {
        let r = (high - low).max(0.0);
        self.ranges[self.idx] = r;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if !self.filled {
            return (self.compressed, self.burst);
        }

        let len = self.window;
        let mut min_r: f64 = f64::MAX;
        let mut max_r: f64 = 0.0;
        for i in 0..len {
            let v = self.ranges[i];
            min_r = min_r.min(v);
            max_r = max_r.max(v);
        }
        let current = self.ranges[(self.idx + len - 1) % len];
        let prev = self.ranges[(self.idx + len - 2) % len];
        let thr = min_r + 0.1 * (max_r - min_r).max(1e-12);
        let was_compressed = self.compressed;
        self.compressed = current <= thr;
        self.burst = was_compressed && current > prev && current > thr;
        (self.compressed, self.burst)
    }

    pub fn value(&self) -> IndicatorValue {
        // burst=1.0 (breakout), compressed=0.5, else=0.0
        let v = if self.burst { 1.0 } else if self.compressed { 0.5 } else { 0.0 };
        IndicatorValue::Single(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_compression_burst_creation() {
        let rcb = RangeCompressionBurst::new(20);
        assert!(!rcb.is_ready());
    }

    #[test]
    fn test_range_compression_burst_warmup() {
        let mut rcb = RangeCompressionBurst::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rcb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rcb.is_ready());
    }

    #[test]
    fn test_range_compression_burst_values() {
        let mut rcb = RangeCompressionBurst::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            let (compressed, burst) = rcb.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            // Just check no panics, values can be true or false
            let _ = compressed;
            let _ = burst;
        }
    }

    #[test]
    fn test_range_compression_burst_reset() {
        let mut rcb = RangeCompressionBurst::new(20);
        for i in 0..25 {
            rcb.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        rcb.reset();
        assert!(!rcb.is_ready());
    }
}
