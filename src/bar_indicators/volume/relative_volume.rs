// Relative Volume (RVOL) and percentile variant

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct RelativeVolume {
    window: usize,
    vol_buffer: Vec<f64>,
    idx: usize,
    filled: bool,
    rvol: f64,
    rvol_percentile: f64,
}

impl RelativeVolume {
    pub fn new(window: usize) -> Self {
        Self {
            window,
            vol_buffer: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            rvol: 0.0,
            rvol_percentile: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.vol_buffer.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.rvol = 0.0;
        self.rvol_percentile = 0.0;
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        _close: f64,
        volume: f64,
    ) -> (f64, f64) {
        // Update buffer with current volume
        self.vol_buffer[self.idx] = volume.max(0.0);
        self.idx = (self.idx + 1) % self.window.max(1);
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }

        let len = if self.filled { self.window } else { self.idx };
        if len == 0 {
            self.rvol = 0.0;
            self.rvol_percentile = 0.0;
            return (self.rvol, self.rvol_percentile);
        }

        // Mean volume
        let mut sum = 0.0;
        for i in 0..len {
            sum += self.vol_buffer[i];
        }
        let mean = if sum > 0.0 { sum / (len as f64) } else { 0.0 };
        self.rvol = if mean > 0.0 {
            self.vol_buffer[(self.idx + self.window - 1) % self.window] / mean
        } else {
            0.0
        };

        // Percentile rank of current vol
        let curr = self.vol_buffer[(self.idx + self.window - 1) % self.window];
        let mut count = 0usize;
        for i in 0..len {
            if self.vol_buffer[i] <= curr {
                count += 1;
            }
        }
        self.rvol_percentile = (count as f64) / (len as f64);
        (self.rvol, self.rvol_percentile)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.rvol, self.rvol_percentile)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_volume_creation() {
        let rvol = RelativeVolume::new(20);
        assert!(!rvol.is_ready());
        assert_eq!(rvol.value(), IndicatorValue::Double(0.0, 0.0));
    }

    #[test]
    fn test_relative_volume_warmup() {
        let mut rvol = RelativeVolume::new(20);
        for i in 0..25 {
            let volume = 1000.0 + (i as f64 * 50.0);
            rvol.update_bar(100.0, 101.0, 99.0, 100.0, volume);
        }
        assert!(rvol.is_ready());
    }

    #[test]
    fn test_relative_volume_values() {
        let mut rvol = RelativeVolume::new(10);
        for i in 0..15 {
            let volume = 1000.0 + (i as f64 * 0.2).sin() * 500.0;
            let (rv, percentile) = rvol.update_bar(100.0, 101.0, 99.0, 100.0, volume);
            assert!(rv >= 0.0, "RVOL should be non-negative");
            assert!(percentile >= 0.0 && percentile <= 1.0, "Percentile in [0, 1]");
        }
    }

    #[test]
    fn test_relative_volume_reset() {
        let mut rvol = RelativeVolume::new(10);
        for i in 0..15 {
            rvol.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0 + i as f64 * 100.0);
        }
        rvol.reset();
        assert!(!rvol.is_ready());
        assert_eq!(rvol.value(), IndicatorValue::Double(0.0, 0.0));
    }
}
