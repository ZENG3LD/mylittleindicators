// Normalized Range and NR flags (NR4/NR7) plus percentile of range

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct NrRange {
    window: usize,
    ranges: Vec<f64>,
    idx: usize,
    filled: bool,
    // outputs
    pub range: f64,
    pub percentile: f64, // [0..1]
    pub is_nr4: bool,
    pub is_nr7: bool,
}

impl NrRange {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            ranges: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            range: 0.0,
            percentile: 0.0,
            is_nr4: false,
            is_nr7: false,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ranges.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.range = 0.0;
        self.percentile = 0.0;
        self.is_nr4 = false;
        self.is_nr7 = false;
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
    ) -> (f64, f64, bool, bool) {
        let current_range = (high - low).max(0.0);
        self.range = current_range;
        self.ranges[self.idx] = current_range;

        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        // Percentile of current range among window
        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut count_le = 0usize;
            for i in 0..len {
                if self.ranges[i] <= current_range {
                    count_le += 1;
                }
            }
            self.percentile = count_le as f64 / len as f64;
        } else {
            self.percentile = 0.0;
        }

        // NR4 / NR7 flags
        self.is_nr4 = false;
        self.is_nr7 = false;
        if len >= 4 {
            let start = len - 4;
            let mut min_r = f64::INFINITY;
            for i in start..len {
                if self.ranges[i] < min_r {
                    min_r = self.ranges[i];
                }
            }
            self.is_nr4 = current_range <= min_r + 1e-12;
        }
        if len >= 7 {
            let start = len - 7;
            let mut min_r = f64::INFINITY;
            for i in start..len {
                if self.ranges[i] < min_r {
                    min_r = self.ranges[i];
                }
            }
            self.is_nr7 = current_range <= min_r + 1e-12;
        }

        (self.range, self.percentile, self.is_nr4, self.is_nr7)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nr_range_creation() {
        let nr = NrRange::new(20);
        assert!(!nr.is_ready());
        assert_eq!(nr.value().main(), 0.0);
    }

    #[test]
    fn test_nr_range_warmup() {
        let mut nr = NrRange::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            nr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(nr.is_ready());
    }

    #[test]
    fn test_nr_range_percentile() {
        let mut nr = NrRange::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            let (_, pct, _, _) = nr.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(pct >= 0.0 && pct <= 1.0);
        }
    }

    #[test]
    fn test_nr_range_reset() {
        let mut nr = NrRange::new(20);
        for i in 0..25 {
            nr.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        nr.reset();
        assert!(!nr.is_ready());
        assert_eq!(nr.value().main(), 0.0);
    }
}
