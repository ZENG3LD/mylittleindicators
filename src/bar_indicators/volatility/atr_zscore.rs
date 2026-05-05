// ATR Z-Score over rolling window

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct AtrZscore {
    atr: Atr,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    sum: f64,
    sumsq: f64,
    value: f64,
}

impl AtrZscore {
    pub fn new(atr_period: usize, atr_ma_type: MovingAverageType, window: usize) -> Self {
        Self {
            atr: Atr::new(atr_period, atr_ma_type),
            window: window.max(2),
            buf: vec![0.0; window.max(2)],
            idx: 0,
            filled: false,
            sum: 0.0,
            sumsq: 0.0,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.atr.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.sum = 0.0;
        self.sumsq = 0.0;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.atr.is_ready()
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let atr_v = self.atr.update_bar(open, high, low, close, volume);

        // rolling mean/std via ring buffer
        let old = self.buf[self.idx];
        self.buf[self.idx] = atr_v;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        self.sum += atr_v - old;
        self.sumsq += atr_v * atr_v - old * old;

        let n = if self.filled {
            self.window as f64
        } else {
            self.idx as f64
        };
        if n >= 2.0 {
            let mean = self.sum / n;
            let var = (self.sumsq / n) - mean * mean;
            let std = if var > 0.0 { var.sqrt() } else { 0.0 };
            self.value = if std > 1e-12 {
                (atr_v - mean) / std
            } else {
                0.0
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
    fn test_atr_zscore_creation() {
        let az = AtrZscore::new(14, MovingAverageType::RMA, 50);
        assert!(!az.is_ready());
        assert_eq!(az.value().main(), 0.0);
    }

    #[test]
    fn test_atr_zscore_warmup() {
        let mut az = AtrZscore::new(14, MovingAverageType::RMA, 50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            az.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(az.is_ready());
    }

    #[test]
    fn test_atr_zscore_values() {
        let mut az = AtrZscore::new(14, MovingAverageType::RMA, 50);
        for i in 0..60 {
            let price = 100.0 + i as f64;
            let value = az.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_atr_zscore_reset() {
        let mut az = AtrZscore::new(14, MovingAverageType::RMA, 50);
        for i in 0..60 {
            az.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        az.reset();
        assert!(!az.is_ready());
        assert_eq!(az.value().main(), 0.0);
    }
}
