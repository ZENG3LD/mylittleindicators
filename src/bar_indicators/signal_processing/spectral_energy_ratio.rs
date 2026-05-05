// Spectral Energy Ratio: low-band vs high-band power ratio

use crate::bar_indicators::signal_processing::fft::FastFourierTransform;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SpectralEnergyRatio {
    window: usize,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub ratio: f64, // low/(low+high)
    low_cut: f64,
}

impl SpectralEnergyRatio {
    pub fn new(window: usize, low_cut_fraction: f64) -> Self {
        let w = window.clamp(32, 256);
        Self {
            window: w,
            fft: FastFourierTransform::new(w, 1.0),
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            ratio: 0.0,
            low_cut: low_cut_fraction.clamp(0.05, 0.95),
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.fft.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.ratio = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        self.buf[self.idx] = c;
        self.idx = (self.idx + 1) % self.window;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let n = self.window;
            let mut mean = 0.0;
            for i in 0..n {
                mean += self.buf[i];
            }
            mean /= n as f64;
            for i in 0..n {
                let x = self.buf[(self.idx + i) % n] - mean;
                self.fft.update(x);
            }
            let fd = self.fft.frequency_domain();
            let mut low = 0.0;
            let mut high = 0.0;
            let split_freq = self.low_cut * 0.5;
            for i in 0..fd.power_spectrum.len() {
                let f = if i < fd.frequencies.len() {
                    fd.frequencies[i]
                } else {
                    0.0
                };
                let p = fd.power_spectrum[i];
                if f <= split_freq {
                    low += p;
                } else {
                    high += p;
                }
            }
            let total = low + high;
            self.ratio = if total > 0.0 { low / total } else { 0.0 };
        }
        self.ratio
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.ratio)
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_energy_ratio_creation() {
        let ser = SpectralEnergyRatio::new(64, 0.25);
        assert!(!ser.is_ready());
        assert_eq!(ser.value().main(), 0.0);
        assert_eq!(ser.window(), 64);
    }

    #[test]
    fn test_spectral_energy_ratio_warmup() {
        let mut ser = SpectralEnergyRatio::new(64, 0.25);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ser.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ser.is_ready());
    }

    #[test]
    fn test_spectral_energy_ratio_range() {
        let mut ser = SpectralEnergyRatio::new(64, 0.25);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ser.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if ser.is_ready() {
                assert!(value >= 0.0 && value <= 1.0, "Ratio should be in [0, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_spectral_energy_ratio_reset() {
        let mut ser = SpectralEnergyRatio::new(64, 0.25);
        for i in 0..70 {
            ser.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ser.reset();
        assert!(!ser.is_ready());
        assert_eq!(ser.value().main(), 0.0);
    }
}
