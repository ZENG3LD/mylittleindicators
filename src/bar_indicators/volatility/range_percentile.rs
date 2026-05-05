// Range Percentile over rolling window using (High-Low)

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct RangePercentile {
    window: usize,
    buffer: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
    percentile: f64,
}

impl RangePercentile {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            buffer: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            value: 0.0,
            percentile: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
        self.percentile = 0.0;
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
    ) -> (f64, f64) {
        self.value = (high - low).max(0.0);
        self.buffer[self.idx] = self.value;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut count_le = 0usize;
            for i in 0..len {
                if self.buffer[i] <= self.value {
                    count_le += 1;
                }
            }
            self.percentile = count_le as f64 / len as f64;
        } else {
            self.percentile = 0.0;
        }
        (self.value, self.percentile)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    #[inline]
    pub fn percentile(&self) -> f64 {
        self.percentile
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_percentile_creation() {
        let rp = RangePercentile::new(20);
        assert!(!rp.is_ready());
        assert_eq!(rp.value().main(), 0.0);
    }

    #[test]
    fn test_range_percentile_warmup() {
        let mut rp = RangePercentile::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rp.is_ready());
    }

    #[test]
    fn test_range_percentile_range() {
        let mut rp = RangePercentile::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            let (_, pct) = rp.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(pct >= 0.0 && pct <= 1.0);
        }
    }

    #[test]
    fn test_range_percentile_reset() {
        let mut rp = RangePercentile::new(20);
        for i in 0..25 {
            rp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        rp.reset();
        assert!(!rp.is_ready());
        assert_eq!(rp.value().main(), 0.0);
    }
}
