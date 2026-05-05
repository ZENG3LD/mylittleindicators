// MACD Histogram Z-Score over rolling window

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::momentum::macd::Macd;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct MacdHistZscore {
    macd: Macd,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    sum: f64,
    sumsq: f64,
    value: f64,
}

impl MacdHistZscore {
    pub fn new(
        fast: usize,
        slow: usize,
        signal: usize,
        ma_type: MovingAverageType,
        window: usize,
    ) -> Self {
        let mut macd = Macd::with_signal(fast, slow, signal, ma_type);
        // ensure clean
        macd.reset();
        Self {
            macd,
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
        self.macd.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.sum = 0.0;
        self.sumsq = 0.0;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.macd.is_ready()
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let _macd_v = self.macd.update_bar(open, high, low, close, volume);
        let hist = self.macd.value_histogram();
        let old = self.buf[self.idx];
        self.buf[self.idx] = hist;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        self.sum += hist - old;
        self.sumsq += hist * hist - old * old;
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
                (hist - mean) / std
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

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macd_hist_zscore_creation() {
        let mhz = MacdHistZscore::new(12, 26, 9, MovingAverageType::EMA, 20);
        assert!(!mhz.is_ready());
        assert_eq!(mhz.value().main(), 0.0);
        assert_eq!(mhz.window(), 20);
    }

    #[test]
    fn test_macd_hist_zscore_basic() {
        let mut mhz = MacdHistZscore::new(12, 26, 9, MovingAverageType::EMA, 20);
        for i in 1..=60 {
            let price = 100.0 + i as f64 * 2.0;
            mhz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(mhz.is_ready());
        assert!(mhz.value().main().is_finite());
    }

    #[test]
    fn test_macd_hist_zscore_reset() {
        let mut mhz = MacdHistZscore::new(12, 26, 9, MovingAverageType::EMA, 20);
        for i in 1..=60 {
            let price = 100.0 + i as f64;
            mhz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(mhz.is_ready());
        mhz.reset();
        assert!(!mhz.is_ready());
        assert_eq!(mhz.value().main(), 0.0);
    }

    #[test]
    fn test_macd_hist_zscore_finite_values() {
        let mut mhz = MacdHistZscore::new(12, 26, 9, MovingAverageType::EMA, 20);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = mhz.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "MACD Hist Zscore should always be finite");
        }
    }
}
