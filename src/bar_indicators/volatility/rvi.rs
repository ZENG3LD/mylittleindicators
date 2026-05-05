// High-performance Relative Volatility Index (RVI)
// (c) 2024

use arrayvec::ArrayVec;
use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct Rvi {
    period: usize,
    ma_type: MovingAverageType,
    scalar: f64,
    closes: ArrayVec<f64, 512>,
    idx: usize,
    filled: bool,
    prev_close: f64,
    value: f64,
    pos_rma: MovingAverageProvider,
    neg_rma: MovingAverageProvider,
    ma_close: MovingAverageProvider,
}


impl Rvi {
    /// Create RVI with default MA type (RMA)
    pub fn new(period: usize) -> Self {
        Self::new_with_ma_type(period, MovingAverageType::RMA)
    }

    /// Create RVI with specified MA type
    pub fn new_with_ma_type(period: usize, ma_type: MovingAverageType) -> Self {
        assert!(period <= 512, "RVI period must be <= 512");
        Self {
            period,
            ma_type,
            scalar: 100.0,
            closes: ArrayVec::from([0.0; 512]),
            idx: 0,
            filled: false,
            prev_close: 0.0,
            value: 0.0,
            pos_rma: MovingAverageProvider::new(ma_type, period),
            neg_rma: MovingAverageProvider::new(ma_type, period),
            ma_close: MovingAverageProvider::new(ma_type, period),
        }
    }

    /// Set the MA type and reset the indicator
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }
    /// Обновить RVI новым баром (используется close)
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> f64 {
        // cyclically update closes
        self.closes[self.idx % 512] = close;
        // обновить ma_close (Wilder)
        self.ma_close.update_bar(0.0, 0.0, 0.0, close, 0.0);
        // calculate std по rolling window (circular buffer)
        let n = self.period.min(self.idx + 1);
        let mean = self.ma_close.value().main();
        // Собрать последние n close в правильном порядке
        let mut window = [0.0; 512];
        for (i, slot) in window[..n].iter_mut().enumerate() {
            let idx = if self.idx + 512 >= i { (self.idx + 512 - i) % 512 } else { 0 };
            *slot = self.closes[idx];
        }
        let mut std = (window[..n].iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
        // корректировка на n-1 (несмещённая оценка)
        if n > 1 {
            std *= (n as f64).sqrt() / ((n - 1) as f64).sqrt();
        }
        // сглаживаем через RMA (Wilder)
        if self.idx > 0 {
            if close > self.prev_close {
                self.pos_rma.update_bar(0.0, 0.0, 0.0, std, 0.0);
                self.neg_rma.update_bar(0.0, 0.0, 0.0, 0.0, 0.0);
            } else if close < self.prev_close {
                self.pos_rma.update_bar(0.0, 0.0, 0.0, 0.0, 0.0);
                self.neg_rma.update_bar(0.0, 0.0, 0.0, std, 0.0);
            } else {
                self.pos_rma.update_bar(0.0, 0.0, 0.0, 0.0, 0.0);
                self.neg_rma.update_bar(0.0, 0.0, 0.0, 0.0, 0.0);
            }
        }
        self.prev_close = close;
        self.idx += 1;
        if self.idx >= self.period {
            self.filled = true;
        }
        if !self.filled {
            self.value = 0.0;
            return self.value;
        }
        let pos = self.pos_rma.value().main();
        let neg = self.neg_rma.value().main();
        let denom = pos + neg;
        self.value = if denom.abs() < 1e-12 { 0.0 } else { self.scalar * pos / denom };
        self.value
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn reset(&mut self) {
        self.closes.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.prev_close = 0.0;
        self.value = 0.0;
        self.pos_rma = MovingAverageProvider::new(self.ma_type, self.period);
        self.neg_rma = MovingAverageProvider::new(self.ma_type, self.period);
        self.ma_close = MovingAverageProvider::new(self.ma_type, self.period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rvi_creation() {
        let rvi = Rvi::new(14);
        assert!(!rvi.is_ready());
        assert_eq!(rvi.value().main(), 0.0);
    }

    #[test]
    fn test_rvi_warmup() {
        let mut rvi = Rvi::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rvi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rvi.is_ready());
    }

    #[test]
    fn test_rvi_range() {
        let mut rvi = Rvi::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = rvi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 100.0, "RVI should be in [0, 100]");
        }
    }

    #[test]
    fn test_rvi_with_ema() {
        let mut rvi = Rvi::new_with_ma_type(14, MovingAverageType::EMA);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.15).sin() * 7.0;
            rvi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rvi.is_ready());
    }

    #[test]
    fn test_rvi_reset() {
        let mut rvi = Rvi::new(14);
        for i in 0..20 {
            rvi.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        rvi.reset();
        assert!(!rvi.is_ready());
        assert_eq!(rvi.value().main(), 0.0);
    }
}






















