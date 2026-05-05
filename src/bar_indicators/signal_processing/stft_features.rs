// STFT band energy ratio (placeholder using rolling var over subwindows)

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct StftBandEnergyRatio {
    window: usize,
    split: usize,
    buf: ArrayVec<f64, 1024>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl StftBandEnergyRatio {
    pub fn new(window: usize, split: usize) -> Self {
        Self {
            window: window.clamp(4, 1024),
            split: split.clamp(1, 8),
            buf: ArrayVec::new(),
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        if self.buf.len() < self.window {
            self.buf.push(c);
            if self.buf.len() == self.window {
                self.filled = true;
            }
        } else {
            self.buf[self.idx] = c;
        }
        self.idx = (self.idx + 1) % self.window;
        if self.is_ready() {
            let part = self.window / self.split.max(1);
            let mut low_var = 0.0;
            let mut high_var = 0.0;
            for i in 0..part {
                let x = self.buf[i] - c;
                low_var += x * x;
            }
            for i in (self.window - part)..self.window {
                let x = self.buf[i] - c;
                high_var += x * x;
            }
            // Add epsilon to avoid division by zero
            // Returns ratio of high-frequency to low-frequency energy
            let eps = 1e-10;
            self.value = (high_var + eps) / (low_var + eps);
        }
        self.value
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stft_band_energy_creation() {
        let stft = StftBandEnergyRatio::new(64, 4);
        assert!(!stft.is_ready());
        assert_eq!(stft.value().main(), 0.0);
        assert_eq!(stft.window(), 64);
    }

    #[test]
    fn test_stft_band_energy_warmup() {
        let mut stft = StftBandEnergyRatio::new(64, 4);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            stft.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(stft.is_ready());
    }

    #[test]
    fn test_stft_band_energy_finite() {
        let mut stft = StftBandEnergyRatio::new(64, 4);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = stft.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "STFT value should be finite");
        }
    }

    #[test]
    fn test_stft_band_energy_reset() {
        let mut stft = StftBandEnergyRatio::new(64, 4);
        for i in 0..70 {
            stft.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        stft.reset();
        assert!(!stft.is_ready());
        assert_eq!(stft.value().main(), 0.0);
    }
}
