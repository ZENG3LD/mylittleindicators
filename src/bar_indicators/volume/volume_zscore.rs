// Volume Z-Score over rolling window

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct VolumeZscore {
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    sum: f64,
    sumsq: f64,
    z: f64,
}

impl VolumeZscore {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            buf: vec![0.0; window.max(2)],
            idx: 0,
            filled: false,
            sum: 0.0,
            sumsq: 0.0,
            z: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.sum = 0.0;
        self.sumsq = 0.0;
        self.z = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        _close: f64,
        volume: f64,
    ) -> f64 {
        let old = self.buf[self.idx];
        self.buf[self.idx] = volume;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        self.sum += volume - old;
        self.sumsq += volume * volume - old * old;
        let n = if self.filled {
            self.window as f64
        } else {
            self.idx as f64
        };
        if n >= 2.0 {
            let mean = self.sum / n;
            let var = (self.sumsq / n) - mean * mean;
            let std = if var > 0.0 { var.sqrt() } else { 0.0 };
            self.z = if std > 1e-12 {
                (volume - mean) / std
            } else {
                0.0
            };
        }
        self.z
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_zscore_creation() {
        let vz = VolumeZscore::new(20);
        assert!(!vz.is_ready());
        assert_eq!(vz.value().main(), 0.0);
    }

    #[test]
    fn test_volume_zscore_warmup() {
        let mut vz = VolumeZscore::new(20);
        for i in 0..25 {
            let volume = 1000.0 + (i as f64 * 0.1).sin() * 100.0;
            vz.update_bar(100.0, 101.0, 99.0, 100.0, volume);
        }
        assert!(vz.is_ready());
    }

    #[test]
    fn test_volume_zscore_values() {
        let mut vz = VolumeZscore::new(20);
        for i in 0..30 {
            let volume = 1000.0 + i as f64 * 50.0;
            let value = vz.update_bar(100.0, 101.0, 99.0, 100.0, volume);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_volume_zscore_reset() {
        let mut vz = VolumeZscore::new(20);
        for i in 0..30 {
            vz.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0 + i as f64 * 10.0);
        }
        vz.reset();
        assert!(!vz.is_ready());
        assert_eq!(vz.value().main(), 0.0);
    }
}
