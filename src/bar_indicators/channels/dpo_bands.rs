// DPO Bands on oscillator scale: upper/lower = +/- k * std(DPO)

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::momentum::dpo::DetrendedPriceOscillator;
use arrayvec::ArrayVec;


#[derive(Clone)]
pub struct DpoBands {
    dpo: DetrendedPriceOscillator,
    window: usize,
    k: f64,
    buf: ArrayVec<f64, 512>,
    idx: usize,
    filled: bool,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl DpoBands {
    pub fn new(period: usize, window: usize, k: f64) -> Self {
        Self {
            dpo: DetrendedPriceOscillator::with_period(period.max(2)),
            window: window.clamp(5, 512),
            k: if k > 0.0 { k } else { 2.0 },
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
        self.dpo.reset();
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.dpo.is_ready()
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
        let d = self.dpo.update_bar(o, h, l, c, v);
        if self.buf.len() < self.window {
            self.buf.push(d);
            if self.buf.len() == self.window {
                self.filled = true;
            }
        } else {
            self.buf[self.idx] = d;
        }
        self.idx = (self.idx + 1) % self.window;
        self.middle = 0.0;
        if self.is_ready() {
            let mean = self.buf.iter().sum::<f64>() / (self.window as f64);
            let var = self
                .buf
                .iter()
                .map(|&x| {
                    let dd = x - mean;
                    dd * dd
                })
                .sum::<f64>()
                / (self.window as f64);
            let sd = var.sqrt();
            self.upper = self.k * sd;
            self.lower = -self.k * sd;
        }
        (self.upper, self.middle, self.lower)
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpo_bands_creation() {
        let db = DpoBands::new(14, 20, 2.0);
        assert!(!db.is_ready());
        assert_eq!(db.window(), 20);
    }

    #[test]
    fn test_dpo_bands_warmup() {
        let mut db = DpoBands::new(14, 20, 2.0);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            db.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(db.is_ready());
    }

    #[test]
    fn test_dpo_bands_symmetric() {
        let mut db = DpoBands::new(14, 20, 2.0);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, middle, lower) = db.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if db.is_ready() {
                assert_eq!(middle, 0.0, "Middle should be 0");
                assert!((upper + lower).abs() < 1e-9, "Bands should be symmetric around 0");
            }
        }
    }

    #[test]
    fn test_dpo_bands_reset() {
        let mut db = DpoBands::new(14, 20, 2.0);
        for i in 0..50 {
            db.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        db.reset();
        assert!(!db.is_ready());
    }
}
