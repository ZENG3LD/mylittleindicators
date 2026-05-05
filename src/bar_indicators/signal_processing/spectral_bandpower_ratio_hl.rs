// Ratio of spectral bandpower: high / low

use crate::bar_indicators::signal_processing::fft::FastFourierTransform;
use crate::bar_indicators::indicator_value::IndicatorValue;

pub struct SpectralBandpowerRatioHL {
    window: usize,
    low_cut: f64,
    high_cut: f64,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralBandpowerRatioHL {
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
            value: 1.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.fft.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 1.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let sr = self.fft.sampling_rate();
        let fd = self.fft.update(c);
        let freqs = &fd.frequencies;
        let power = &fd.power_spectrum;
        if freqs.is_empty() || power.is_empty() {
            return self.value;
        }
        let mut low = 0.0;
        let mut high = 0.0;
        let nyq = 0.5;
        for (i, &f) in freqs.iter().enumerate() {
            let frac = f / (sr.max(1e-9));
            let p = *power.get(i).unwrap_or(&0.0);
            if frac < self.low_cut * nyq {
                low += p;
            } else if frac >= self.high_cut * nyq {
                high += p;
            }
        }
        self.value = if low > 0.0 { high / low } else { self.value };
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
    fn test_spectral_bandpower_ratio_creation() {
        let sbr = SpectralBandpowerRatioHL::new(64, 0.2, 0.6);
        assert!(!sbr.is_ready());
        assert_eq!(sbr.value().main(), 1.0);
        assert_eq!(sbr.window(), 64);
    }

    #[test]
    fn test_spectral_bandpower_ratio_finite() {
        let mut sbr = SpectralBandpowerRatioHL::new(64, 0.2, 0.6);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sbr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Ratio should be finite, got {}", value);
        }
    }

    #[test]
    fn test_spectral_bandpower_ratio_reset() {
        let mut sbr = SpectralBandpowerRatioHL::new(64, 0.2, 0.6);
        for i in 0..70 {
            sbr.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        sbr.reset();
        assert!(!sbr.is_ready());
        assert_eq!(sbr.value().main(), 1.0);
    }
}
