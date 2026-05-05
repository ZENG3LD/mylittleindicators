// Darvas Box - simple box breakout structure

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct DarvasBox {
    lookback: usize,
    high_box: f64,
    low_box: f64,
    ready: bool,
}

impl DarvasBox {
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback: lookback.max(2),
            high_box: 0.0,
            low_box: 0.0,
            ready: false,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.high_box = 0.0;
        self.low_box = 0.0;
        self.ready = false;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.high_box, self.low_box)
    }
    #[inline]
    pub fn value_tuple(&self) -> (f64, f64) {
        (self.high_box, self.low_box)
    }
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, _c: f64, _v: f64) -> (f64, f64) {
        if !self.ready {
            self.high_box = h;
            self.low_box = l;
            self.ready = true;
        } else {
            self.high_box = self.high_box.max(h);
            self.low_box = self.low_box.min(l);
        }
        (self.high_box, self.low_box)
    }

    pub fn lookback(&self) -> usize {
        self.lookback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_darvas_box_creation() {
        let db = DarvasBox::new(20);
        assert!(!db.is_ready());
        assert_eq!(db.lookback(), 20);
    }

    #[test]
    fn test_darvas_box_warmup() {
        let mut db = DarvasBox::new(20);
        let (high, low) = db.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0);
        assert!(db.is_ready());
        assert_eq!(high, 101.0);
        assert_eq!(low, 99.0);
    }

    #[test]
    fn test_darvas_box_expansion() {
        let mut db = DarvasBox::new(20);
        db.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0);
        let (high, low) = db.update_bar(100.0, 105.0, 95.0, 100.0, 1000.0);
        assert_eq!(high, 105.0);
        assert_eq!(low, 95.0);
    }

    #[test]
    fn test_darvas_box_reset() {
        let mut db = DarvasBox::new(20);
        db.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0);
        db.reset();
        assert!(!db.is_ready());
    }
}
