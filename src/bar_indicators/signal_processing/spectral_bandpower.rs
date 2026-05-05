// Spectral Bandpower: power within predefined frequency bands over rolling window

use crate::bar_indicators::signal_processing::fft::FastFourierTransform;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone, Copy)]
enum BandKind {
    Low,
    Mid,
    High,
}

/// Generic spectral bandpower calculator with 3 bands: [0, l), [l, h), [h, 0.5]
/// Frequency expressed as fraction of sampling rate (Nyquist = 0.5)
struct SpectralBandpowerGeneric {
    window: usize,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    low_cut: f64,
    high_cut: f64,
    band: BandKind,
    value: f64, // primary output (band power share)
}

impl SpectralBandpowerGeneric {
    fn new(window: usize, low_cut_fraction: f64, high_cut_fraction: f64, band: BandKind) -> Self {
        let w = window.clamp(32, 512);
        let l = low_cut_fraction.clamp(0.05, 0.45);
        let h = high_cut_fraction.clamp(l + 0.01, 0.49);
        Self {
            window: w,
            fft: FastFourierTransform::new(w, 1.0),
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            low_cut: l,
            high_cut: h,
            band,
            value: 0.0,
        }
    }

    #[inline]
    fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.buf.fill(0.0);
        self.value = 0.0;
        self.fft.reset();
    }

    #[inline]
    fn is_ready(&self) -> bool {
        self.filled && self.fft.is_ready()
    }

    fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let n = self.window;
        self.buf[self.idx] = c;
        self.idx = (self.idx + 1) % n;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }

        if self.filled {
            // Demean and feed into FFT in ring order
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
            let mut low = 0.0f64;
            let mut mid = 0.0f64;
            let mut high = 0.0f64;
            for i in 0..fd.power_spectrum.len() {
                let f = if i < fd.frequencies.len() {
                    fd.frequencies[i]
                } else {
                    0.0
                };
                let p = fd.power_spectrum[i];
                if f <= self.low_cut {
                    low += p;
                } else if f <= self.high_cut {
                    mid += p;
                } else {
                    high += p;
                }
            }
            let total = (low + mid + high).max(1e-18);
            self.value = match self.band {
                BandKind::Low => low / total,
                BandKind::Mid => mid / total,
                BandKind::High => high / total,
            };
        }
        self.value
    }
}

pub struct SpectralBandpowerLow {
    inner: SpectralBandpowerGeneric,
}
pub struct SpectralBandpowerMid {
    inner: SpectralBandpowerGeneric,
}
pub struct SpectralBandpowerHigh {
    inner: SpectralBandpowerGeneric,
}

impl SpectralBandpowerLow {
    pub fn new(window: usize, low_cut_fraction: f64, high_cut_fraction: f64) -> Self {
        Self {
            inner: SpectralBandpowerGeneric::new(
                window,
                low_cut_fraction,
                high_cut_fraction,
                BandKind::Low,
            ),
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }
    #[inline]
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        self.inner.update_bar(o, h, l, c, v)
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.inner.value
    }
}

impl SpectralBandpowerMid {
    pub fn new(window: usize, low_cut_fraction: f64, high_cut_fraction: f64) -> Self {
        Self {
            inner: SpectralBandpowerGeneric::new(
                window,
                low_cut_fraction,
                high_cut_fraction,
                BandKind::Mid,
            ),
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }
    #[inline]
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        self.inner.update_bar(o, h, l, c, v)
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.inner.value
    }
}

impl SpectralBandpowerHigh {
    pub fn new(window: usize, low_cut_fraction: f64, high_cut_fraction: f64) -> Self {
        Self {
            inner: SpectralBandpowerGeneric::new(
                window,
                low_cut_fraction,
                high_cut_fraction,
                BandKind::High,
            ),
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }
    #[inline]
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        self.inner.update_bar(o, h, l, c, v)
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.inner.value
    }
}

/// Base SpectralBandpower that returns all 3 bands (low, mid, high) at once
#[derive(Clone)]
pub struct SpectralBandpower {
    window: usize,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    low_cut: f64,
    high_cut: f64,
    pub low: f64,
    pub mid: f64,
    pub high: f64,
}

impl SpectralBandpower {
    pub fn new(window: usize, low_cut_fraction: f64, high_cut_fraction: f64) -> Self {
        let w = window.clamp(32, 512);
        let l = low_cut_fraction.clamp(0.05, 0.45);
        let h = high_cut_fraction.clamp(l + 0.01, 0.49);
        Self {
            window: w,
            fft: FastFourierTransform::new(w, 1.0),
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            low_cut: l,
            high_cut: h,
            low: 0.0,
            mid: 0.0,
            high: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.buf.fill(0.0);
        self.low = 0.0;
        self.mid = 0.0;
        self.high = 0.0;
        self.fft.reset();
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.fft.is_ready()
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> (f64, f64, f64) {
        let n = self.window;
        self.buf[self.idx] = c;
        self.idx = (self.idx + 1) % n;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }

        if self.filled {
            // Demean and feed into FFT in ring order
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
            let mut low_sum = 0.0f64;
            let mut mid_sum = 0.0f64;
            let mut high_sum = 0.0f64;
            for i in 0..fd.power_spectrum.len() {
                let f = if i < fd.frequencies.len() {
                    fd.frequencies[i]
                } else {
                    0.0
                };
                let p = fd.power_spectrum[i];
                if f <= self.low_cut {
                    low_sum += p;
                } else if f <= self.high_cut {
                    mid_sum += p;
                } else {
                    high_sum += p;
                }
            }
            let total = (low_sum + mid_sum + high_sum).max(1e-18);
            self.low = low_sum / total;
            self.mid = mid_sum / total;
            self.high = high_sum / total;
        }
        (self.low, self.mid, self.high)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.low, self.mid, self.high)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_bandpower_creation() {
        let sb = SpectralBandpower::new(64, 0.1, 0.3);
        assert!(!sb.is_ready());
        assert_eq!(sb.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_spectral_bandpower_warmup() {
        let mut sb = SpectralBandpower::new(64, 0.1, 0.3);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sb.is_ready());
    }

    #[test]
    fn test_spectral_bandpower_sum_to_one() {
        let mut sb = SpectralBandpower::new(64, 0.1, 0.3);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (l, m, h) = sb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if sb.is_ready() {
                let sum = l + m + h;
                assert!((sum - 1.0).abs() < 0.01, "Bands should sum to ~1.0, got {}", sum);
            }
        }
    }

    #[test]
    fn test_spectral_bandpower_reset() {
        let mut sb = SpectralBandpower::new(64, 0.1, 0.3);
        for i in 0..70 {
            sb.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        sb.reset();
        assert!(!sb.is_ready());
        assert_eq!(sb.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_spectral_bandpower_low() {
        let mut sbl = SpectralBandpowerLow::new(64, 0.1, 0.3);
        assert!(!sbl.is_ready());
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sbl.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sbl.is_ready());
        assert!(sbl.value() >= 0.0 && sbl.value() <= 1.0);
    }
}
