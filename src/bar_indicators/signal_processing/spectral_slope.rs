// Spectral Slope: linear regression slope of log power vs log frequency

use crate::bar_indicators::signal_processing::fft::FastFourierTransform;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SpectralSlope {
    window: usize,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub slope: f64,
}

impl SpectralSlope {
    pub fn new(window: usize) -> Self {
        let w = window.clamp(32, 512);
        Self {
            window: w,
            fft: FastFourierTransform::new(w, 1.0),
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            slope: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.buf.fill(0.0);
        self.slope = 0.0;
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
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            // demean and feed FFT
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
            // linear regression of ln(P) ~ a + b ln(f) over f>0
            let mut sx = 0.0;
            let mut sy = 0.0;
            let mut sxx = 0.0;
            let mut sxy = 0.0;
            let mut cnt = 0.0;
            for i in 0..fd.power_spectrum.len() {
                if i >= fd.frequencies.len() {
                    break;
                }
                let f = fd.frequencies[i];
                if f <= 0.0 {
                    continue;
                }
                let p = fd.power_spectrum[i].max(1e-18);
                let x = f.ln();
                let y = p.ln();
                sx += x;
                sy += y;
                sxx += x * x;
                sxy += x * y;
                cnt += 1.0;
            }
            if cnt >= 2.0 {
                let denom = sxx * cnt - sx * sx;
                self.slope = if denom.abs() > 1e-12 {
                    (sxy * cnt - sx * sy) / denom
                } else {
                    0.0
                };
            }
        }
        self.slope
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.slope)
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_slope_creation() {
        let ss = SpectralSlope::new(64);
        assert!(!ss.is_ready());
        assert_eq!(ss.value().main(), 0.0);
        assert_eq!(ss.window(), 64);
    }

    #[test]
    fn test_spectral_slope_warmup() {
        let mut ss = SpectralSlope::new(64);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ss.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ss.is_ready());
    }

    #[test]
    fn test_spectral_slope_finite() {
        let mut ss = SpectralSlope::new(64);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ss.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Slope should be finite");
        }
    }

    #[test]
    fn test_spectral_slope_reset() {
        let mut ss = SpectralSlope::new(64);
        for i in 0..70 {
            ss.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ss.reset();
        assert!(!ss.is_ready());
        assert_eq!(ss.value().main(), 0.0);
    }
}
