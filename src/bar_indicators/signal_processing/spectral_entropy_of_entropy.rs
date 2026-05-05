// Rolling entropy of spectral entropy (volatility of entropy)

use crate::bar_indicators::signal_processing::spectral_entropy::SpectralEntropy;
use crate::bar_indicators::indicator_value::IndicatorValue;

pub struct SpectralEntropyOfEntropy {
    inner: SpectralEntropy,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralEntropyOfEntropy {
    pub fn new(fft_window: usize, window: usize) -> Self {
        let w = window.max(20);
        Self {
            inner: SpectralEntropy::new(fft_window),
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.inner.is_ready()
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let e = self.inner.update_bar(o, h, l, c, v);
        self.buf[self.idx] = e;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut m = 0.0;
            for &x in &self.buf {
                m += x;
            }
            m /= self.window as f64;
            let mut s = 0.0;
            for &x in &self.buf {
                let d = x - m;
                s += d * d;
            }
            self.value = (s / (self.window as f64)).sqrt();
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
    fn test_spectral_eoe_creation() {
        let eoe = SpectralEntropyOfEntropy::new(64, 30);
        assert!(!eoe.is_ready());
        assert_eq!(eoe.value().main(), 0.0);
        assert_eq!(eoe.window(), 30);
    }

    #[test]
    fn test_spectral_eoe_warmup() {
        let mut eoe = SpectralEntropyOfEntropy::new(64, 30);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            eoe.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(eoe.is_ready());
    }

    #[test]
    fn test_spectral_eoe_finite() {
        let mut eoe = SpectralEntropyOfEntropy::new(64, 30);
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = eoe.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "EoE should be finite");
        }
    }

    #[test]
    fn test_spectral_eoe_reset() {
        let mut eoe = SpectralEntropyOfEntropy::new(64, 30);
        for i in 0..100 {
            eoe.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        eoe.reset();
        assert!(!eoe.is_ready());
        assert_eq!(eoe.value().main(), 0.0);
    }
}
