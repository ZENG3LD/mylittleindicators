// Price Median Absolute Deviation Z-Score over rolling window

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct PriceMadZscore {
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl PriceMadZscore {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(3),
            buf: vec![0.0; window.max(3)],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> f64 {
        self.buf[self.idx] = close;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if !self.filled {
            return self.value;
        }

        let len = if self.filled { self.window } else { self.idx };
        let mut tmp = self.buf[..len].to_vec();
        tmp.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = if len % 2 == 1 {
            tmp[len / 2]
        } else {
            0.5 * (tmp[len / 2 - 1] + tmp[len / 2])
        };
        let mut dev: Vec<f64> = tmp.iter().map(|v| (v - median).abs()).collect();
        dev.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mad = if len % 2 == 1 {
            dev[len / 2]
        } else {
            0.5 * (dev[len / 2 - 1] + dev[len / 2])
        };
        let denom = (mad * 1.4826).max(1e-12);
        self.value = (close - median) / denom;
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
    fn test_price_mad_zscore_creation() {
        let pmz = PriceMadZscore::new(20);
        assert!(!pmz.is_ready());
        assert_eq!(pmz.value().main(), 0.0);
        assert_eq!(pmz.window(), 20);
    }

    #[test]
    fn test_price_mad_zscore_warmup() {
        let mut pmz = PriceMadZscore::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pmz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pmz.is_ready());
    }

    #[test]
    fn test_price_mad_zscore_finite() {
        let mut pmz = PriceMadZscore::new(20);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pmz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Z-score should be finite");
        }
    }

    #[test]
    fn test_price_mad_zscore_reset() {
        let mut pmz = PriceMadZscore::new(20);
        for i in 0..30 {
            pmz.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        pmz.reset();
        assert!(!pmz.is_ready());
        assert_eq!(pmz.value().main(), 0.0);
    }
}
