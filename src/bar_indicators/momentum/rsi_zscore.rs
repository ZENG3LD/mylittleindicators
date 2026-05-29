// RSI Z-Score over rolling window

use crate::bar_indicators::momentum::rsi::Rsi;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct RsiZscore {
    rsi: Rsi,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    sum: f64,
    sumsq: f64,
    z: f64,
}

impl RsiZscore {
    pub fn new(rsi_period: usize, window: usize) -> Self {
        let w = window.max(2);
        Self {
            rsi: Rsi::new(rsi_period),
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            sum: 0.0,
            sumsq: 0.0,
            z: 0.0,
        }
    }

    /// Alias exposing the RSI period parameter explicitly.
    ///
    /// # Arguments
    /// * `rsi_period` - RSI lookback period
    /// * `window`     - Rolling z-score window (minimum 2)
    #[inline]
    pub fn with_rsi_period(rsi_period: usize, window: usize) -> Self {
        Self::new(rsi_period, window)
    }

    #[inline]
    pub fn reset(&mut self) {
        self.rsi.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.sum = 0.0;
        self.sumsq = 0.0;
        self.z = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.rsi.is_ready()
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let v = self.rsi.update_bar(open, high, low, close, volume);
        let old = self.buf[self.idx];
        self.buf[self.idx] = v;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        self.sum += v - old;
        self.sumsq += v * v - old * old;
        let n = if self.filled {
            self.window as f64
        } else {
            self.idx as f64
        };
        if n >= 2.0 {
            let mean = self.sum / n;
            let var = (self.sumsq / n) - mean * mean;
            let std = if var > 0.0 { var.sqrt() } else { 0.0 };
            self.z = if std > 1e-12 { (v - mean) / std } else { 0.0 };
        } else {
            self.z = 0.0;
        }
        self.z
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.z)
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsi_zscore_creation() {
        let rz = RsiZscore::new(14, 20);
        assert!(!rz.is_ready());
        assert_eq!(rz.value().main(), 0.0);
        assert_eq!(rz.window(), 20);
    }

    #[test]
    fn test_rsi_zscore_with_rsi_period() {
        let mut rz = RsiZscore::with_rsi_period(9, 15);
        assert_eq!(rz.window(), 15);
        for i in 1..=40 {
            let p = 100.0 + i as f64 * 0.5;
            let v = rz.update_bar(p, p + 1.0, p - 1.0, p, 1000.0);
            assert!(v.is_finite());
        }
        assert!(rz.is_ready());
    }

    #[test]
    fn test_rsi_zscore_basic() {
        let mut rz = RsiZscore::new(14, 20);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            rz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rz.is_ready());
        assert!(rz.value().main().is_finite());
    }

    #[test]
    fn test_rsi_zscore_reset() {
        let mut rz = RsiZscore::new(14, 20);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            rz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rz.is_ready());
        rz.reset();
        assert!(!rz.is_ready());
        assert_eq!(rz.value().main(), 0.0);
    }

    #[test]
    fn test_rsi_zscore_finite_values() {
        let mut rz = RsiZscore::new(14, 20);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = rz.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "RSI Zscore should always be finite");
        }
    }
}
