// TRIMA Bands: TRIMA +/- k * std

use crate::bar_indicators::average::trima::Trima;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;
use arrayvec::ArrayVec;

#[derive(Debug, Clone)]
pub struct TrimaBands {
    trima: Trima,
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

impl TrimaBands {
    pub fn new(period: usize, k: f64) -> Self {
        Self {
            trima: Trima::new(period.max(2)),
            window: period.clamp(2, 512),
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

    pub fn with_source(period: usize, k: f64, source: OhlcvField) -> Self {
        Self {
            trima: Trima::new(period.max(2)),
            window: period.clamp(2, 512),
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
        self.trima.reset();
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.trima.is_ready()
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
        self.middle = self.trima.update_bar(0.0, 0.0, 0.0, price, 0.0);
        if self.is_ready() {
            let mean = self.buf.iter().sum::<f64>() / (self.window as f64);
            let var = self
                .buf
                .iter()
                .map(|&x| {
                    let d = x - mean;
                    d * d
                })
                .sum::<f64>()
                / (self.window as f64);
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
    fn test_trima_bands_creation() {
        let tb = TrimaBands::new(20, 2.0);
        assert!(!tb.is_ready());
        assert_eq!(tb.upper, 0.0);
        assert_eq!(tb.lower, 0.0);
    }

    #[test]
    fn test_trima_bands_warmup() {
        let mut tb = TrimaBands::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            tb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(tb.is_ready());
    }

    #[test]
    fn test_trima_bands_values() {
        let mut tb = TrimaBands::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            tb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(tb.upper >= tb.middle);
        assert!(tb.middle >= tb.lower);
    }

    #[test]
    fn test_trima_bands_reset() {
        let mut tb = TrimaBands::new(20, 2.0);
        for i in 0..25 {
            tb.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        tb.reset();
        assert!(!tb.is_ready());
        assert_eq!(tb.upper, 0.0);
        assert_eq!(tb.lower, 0.0);
    }
}
