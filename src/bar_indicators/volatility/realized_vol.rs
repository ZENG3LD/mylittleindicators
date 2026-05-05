// Realized Volatility over rolling window using log returns

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct RealizedVol {
    window: usize,
    // ring buffer of squared log returns
    r2_buffer: Vec<f64>,
    idx: usize,
    filled: bool,
    prev_close: f64,
    sum_r2: f64,
    value: f64,
    annualize_factor: f64,
}

impl RealizedVol {
    pub fn new(window: usize, annualize_factor: f64) -> Self {
        Self {
            window: window.max(1),
            r2_buffer: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            prev_close: 0.0,
            sum_r2: 0.0,
            value: 0.0,
            annualize_factor,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.r2_buffer.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.prev_close = 0.0;
        self.sum_r2 = 0.0;
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
        if self.prev_close <= 0.0 {
            self.prev_close = close.max(1e-12);
            return self.value;
        }
        let r = (close / self.prev_close).ln();
        self.prev_close = close.max(1e-12);
        let r2 = r * r;

        // update ring buffer
        let old = self.r2_buffer[self.idx];
        self.r2_buffer[self.idx] = r2;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        // update sum
        self.sum_r2 += r2 - old;
        let denom = if self.filled {
            self.window as f64
        } else {
            self.idx as f64
        };
        if denom > 0.0 {
            let vol = (self.sum_r2 / denom).sqrt();
            self.value = if self.annualize_factor > 0.0 {
                vol * self.annualize_factor
            } else {
                vol
            };
        } else {
            self.value = 0.0;
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_realized_vol_creation() {
        let rv = RealizedVol::new(20, 252.0);
        assert!(!rv.is_ready());
        assert_eq!(rv.value().main(), 0.0);
    }

    #[test]
    fn test_realized_vol_warmup() {
        let mut rv = RealizedVol::new(20, 252.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rv.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rv.is_ready());
    }

    #[test]
    fn test_realized_vol_positive() {
        let mut rv = RealizedVol::new(20, 252.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            let value = rv.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_realized_vol_reset() {
        let mut rv = RealizedVol::new(20, 252.0);
        for i in 0..25 {
            rv.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        rv.reset();
        assert!(!rv.is_ready());
        assert_eq!(rv.value().main(), 0.0);
    }
}
