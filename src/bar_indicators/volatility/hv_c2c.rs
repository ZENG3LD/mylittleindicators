// Historical Volatility (Close-to-Close) - annualized std dev of log returns

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct HistoricalVolatilityC2C {
    window: usize,
    rets: ArrayVec<f64, 1024>,
    idx: usize,
    count: usize,
    prev_close: f64,
    initialized: bool,
    value: f64,
}

impl HistoricalVolatilityC2C {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.clamp(5, 1024),
            rets: ArrayVec::new(),
            idx: 0,
            count: 0,
            prev_close: 0.0,
            initialized: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.rets.clear();
        self.idx = 0;
        self.count = 0;
        self.prev_close = 0.0;
        self.initialized = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.window
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        if !self.initialized {
            self.prev_close = c;
            self.initialized = true;
            return self.value;
        }
        let ret = (c / self.prev_close).ln();
        self.prev_close = c;
        if self.count < self.window {
            self.rets.push(ret);
            self.count += 1;
        } else {
            self.rets[self.idx] = ret;
        }
        self.idx = (self.idx + 1) % self.window;

        if self.is_ready() {
            // compute mean
            let n = self.window as f64;
            let mut sum = 0.0;
            for &r in &self.rets[0..self.window] {
                sum += r;
            }
            let mean = sum / n;
            let mut var = 0.0;
            for &r in &self.rets[0..self.window] {
                let d = r - mean;
                var += d * d;
            }
            var /= n.max(1.0);
            let daily_vol = var.sqrt();
            self.value = daily_vol * (252.0_f64).sqrt();
        } else {
            self.value = 0.0;
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hv_c2c_creation() {
        let hv = HistoricalVolatilityC2C::new(20);
        assert!(!hv.is_ready());
        assert_eq!(hv.value().main(), 0.0);
    }

    #[test]
    fn test_hv_c2c_warmup() {
        let mut hv = HistoricalVolatilityC2C::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            hv.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(hv.is_ready());
    }

    #[test]
    fn test_hv_c2c_positive() {
        let mut hv = HistoricalVolatilityC2C::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            let value = hv.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_hv_c2c_reset() {
        let mut hv = HistoricalVolatilityC2C::new(20);
        for i in 0..25 {
            hv.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        hv.reset();
        assert!(!hv.is_ready());
        assert_eq!(hv.value().main(), 0.0);
    }
}
