// Spectral Entropy using normalized FFT power on rolling window of closes

use crate::bar_indicators::signal_processing::fft::FastFourierTransform;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SpectralEntropy {
    window: usize,
    fft: FastFourierTransform,
    closes: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl SpectralEntropy {
    pub fn new(window: usize) -> Self {
        let w = window.clamp(16, 256);
        // sampling_rate: 1.0 bar^-1
        Self {
            window: w,
            fft: FastFourierTransform::new(w, 1.0),
            closes: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.closes.fill(0.0);
        self.value = 0.0;
        self.fft.reset();
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.fft.is_ready()
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> f64 {
        let n = self.window;
        self.closes[self.idx] = close;
        self.idx = (self.idx + 1) % n;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }
        // feed normalized demeaned close to FFT
        if self.filled {
            let mut mean = 0.0;
            for i in 0..n {
                mean += self.closes[i];
            }
            mean /= n as f64;
            for i in 0..n {
                self.fft.update(self.closes[(self.idx + i) % n] - mean);
            }
            let fd = self.fft.frequency_domain();
            let mut ps_sum = 0.0;
            for i in 0..fd.power_spectrum.len() {
                ps_sum += fd.power_spectrum[i];
            }
            if ps_sum > 1e-12 {
                // Избегаем деления на очень малые числа
                let mut h = 0.0;
                let n = fd.power_spectrum.len() as f64;
                let epsilon = 1e-15; // Минимальное значение для избежания ln(0)

                for i in 0..fd.power_spectrum.len() {
                    let p = (fd.power_spectrum[i] / ps_sum).max(epsilon);
                    if p > epsilon && p.is_finite() {
                        h -= p * p.ln();
                    }
                }

                // Проверяем на NaN и нормализуем
                if h.is_finite() && h >= 0.0 && n > 1.0 {
                    self.value = (h / n.ln()).clamp(0.0, 1.0);
                } else {
                    self.value = 0.5; // Средняя энтропия при проблемах
                }
            } else {
                self.value = 0.0;
            }
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
    fn test_spectral_entropy_creation() {
        let se = SpectralEntropy::new(64);
        assert!(!se.is_ready());
        assert_eq!(se.value().main(), 0.0);
        assert_eq!(se.window(), 64);
    }

    #[test]
    fn test_spectral_entropy_warmup() {
        let mut se = SpectralEntropy::new(64);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            se.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(se.is_ready());
    }

    #[test]
    fn test_spectral_entropy_range() {
        let mut se = SpectralEntropy::new(64);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = se.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if se.is_ready() {
                assert!(value >= 0.0 && value <= 1.0, "Entropy should be in [0, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_spectral_entropy_reset() {
        let mut se = SpectralEntropy::new(64);
        for i in 0..70 {
            se.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        se.reset();
        assert!(!se.is_ready());
        assert_eq!(se.value().main(), 0.0);
    }
}
