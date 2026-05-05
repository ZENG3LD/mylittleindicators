// Projection Bands - project slope and add std-based bands

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;
use arrayvec::ArrayVec;

#[derive(Debug, Clone)]
pub struct ProjectionBands {
    window: usize,
    k: f64,
    source: OhlcvField,
    buf: ArrayVec<f64, 512>,
    idx: usize,
    filled: bool,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl ProjectionBands {
    pub fn new(window: usize, k: f64) -> Self {
        Self {
            window: window.clamp(2, 512),
            k: if k > 0.0 { k } else { 2.0 },
            source: OhlcvField::Close,
            buf: ArrayVec::new(),
            idx: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    pub fn with_source(window: usize, k: f64, source: OhlcvField) -> Self {
        Self {
            window: window.clamp(2, 512),
            k: if k > 0.0 { k } else { 2.0 },
            source,
            buf: ArrayVec::new(),
            idx: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.upper,
            middle: self.middle,
            lower: self.lower,
        }
    }

    #[inline]
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper, self.middle, self.lower)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> (f64, f64, f64) {
        let price = self.source.extract(o, h, l, c, v);
        if self.buf.len() < self.window {
            self.buf.push(price);
            if self.buf.len() == self.window {
                self.filled = true;
            }
        } else {
            self.buf[self.idx] = price;
        }
        self.idx = (self.idx + 1) % self.window;
        if self.is_ready() {
            let n = self.window as f64;
            let mean = self.buf.iter().sum::<f64>() / n;
            let mut sxx = 0.0;
            let mut sxy = 0.0;
            let mut sx = 0.0;
            for (i, &y) in self.buf.iter().enumerate() {
                let x = i as f64;
                sx += x;
                sxx += x * x;
                sxy += x * (y - mean);
            }
            let den = (n * sxx - sx * sx).max(1e-9);
            let slope = sxy / den;
            let a = mean; // center-based
            self.middle = a + slope * (n - 1.0);
            let var = self
                .buf
                .iter()
                .map(|&p| (p - mean) * (p - mean))
                .sum::<f64>()
                / n.max(1.0);
            let sd = var.sqrt();
            self.upper = self.middle + self.k * sd;
            self.lower = self.middle - self.k * sd;
        }
        (self.upper, self.middle, self.lower)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projection_bands_creation() {
        let pb = ProjectionBands::new(20, 2.0);
        assert!(!pb.is_ready());
        assert_eq!(pb.upper, 0.0);
        assert_eq!(pb.lower, 0.0);
    }

    #[test]
    fn test_projection_bands_warmup() {
        let mut pb = ProjectionBands::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pb.is_ready());
    }

    #[test]
    fn test_projection_bands_values() {
        let mut pb = ProjectionBands::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            pb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pb.upper >= pb.middle);
        assert!(pb.middle >= pb.lower);
    }

    #[test]
    fn test_projection_bands_reset() {
        let mut pb = ProjectionBands::new(20, 2.0);
        for i in 0..25 {
            pb.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        pb.reset();
        assert!(!pb.is_ready());
        assert_eq!(pb.upper, 0.0);
        assert_eq!(pb.lower, 0.0);
    }
}
