// Ratio of spectral power: high band / mid band

use crate::bar_indicators::signal_processing::fft::FastFourierTransform;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SpectralHighMidPowerRatio {
    window: usize,
    low_cut: f64,
    high_cut: f64,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralHighMidPowerRatio {
    pub fn new(window: usize, low_cut_fraction: f64, high_cut_fraction: f64) -> Self {
        let w = window.clamp(32, 1024);
        let lc = low_cut_fraction.clamp(1e-6, 0.49);
        let hc = high_cut_fraction.clamp(lc + 1e-6, 0.499);
        Self {
            window: w,
            low_cut: lc,
            high_cut: hc,
            fft: FastFourierTransform::new(w, 1.0),
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
        self.fft.reset();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.fft.is_ready()
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let n = self.window;
        self.buf[self.idx] = c;
        self.idx = (self.idx + 1) % n;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
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
            let mut mid = 0.0;
            let mut high = 0.0;
            for i in 0..fd.power_spectrum.len() {
                if i >= fd.frequencies.len() {
                    break;
                }
                let f = fd.frequencies[i];
                let p = fd.power_spectrum[i].max(1e-18);
                if f <= self.low_cut {
                    continue;
                } else if f <= self.high_cut {
                    mid += p;
                } else {
                    high += p;
                }
            }
            self.value = if mid > 0.0 { high / mid } else { 0.0 };
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_hm_ratio_creation() {
        let shm = SpectralHighMidPowerRatio::new(64, 0.15, 0.35);
        assert!(!shm.is_ready());
        assert_eq!(shm.value().main(), 0.0);
        assert_eq!(shm.window(), 64);
    }

    #[test]
    fn test_spectral_hm_ratio_warmup() {
        let mut shm = SpectralHighMidPowerRatio::new(64, 0.15, 0.35);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            shm.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(shm.is_ready());
    }

    #[test]
    fn test_spectral_hm_ratio_finite() {
        let mut shm = SpectralHighMidPowerRatio::new(64, 0.15, 0.35);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = shm.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Ratio should be finite");
        }
    }

    #[test]
    fn test_spectral_hm_ratio_reset() {
        let mut shm = SpectralHighMidPowerRatio::new(64, 0.15, 0.35);
        for i in 0..70 {
            shm.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        shm.reset();
        assert!(!shm.is_ready());
        assert_eq!(shm.value().main(), 0.0);
    }
}
