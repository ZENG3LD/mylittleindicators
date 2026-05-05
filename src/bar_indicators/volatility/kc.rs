// High-performance Keltner Channel (KC)
// (c) 2024

use super::atr::Atr;
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct Kc {
    period: usize,
    k_multiplier: f64,
    sma_buf: ArrayVec<f64, 512>,
    sma_sum: f64,
    sma_filled: bool, // true, если буфер заполнен полностью
    atr: Atr,
    pub upper: f64,
    middle: f64,
    pub lower: f64,
}

impl Kc {
    pub fn new(period: usize, k_multiplier: f64) -> Self {
        Self {
            period,
            k_multiplier,
            sma_buf: ArrayVec::new(),
            sma_sum: 0.0,
            sma_filled: false,
            atr: Atr::new(period, crate::bar_indicators::average::moving_average::MovingAverageType::RMA),
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }
    /// Обновить KC новым баром (используются high, low, close)
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> (f64, f64, f64) {
        let typical = (high + low + close) / 3.0;
        // Если буфер заполнен, удаляем старое значение
        if self.sma_buf.len() == self.period {
            let old = self.sma_buf.remove(0);
            self.sma_sum -= old;
        }
        self.sma_buf.push(typical);
        self.sma_sum += typical;
        if self.sma_buf.len() == self.period {
            self.sma_filled = true;
        }
        if self.sma_buf.len() < self.period {
            self.upper = 0.0;
            self.middle = 0.0;
            self.lower = 0.0;
            return (self.upper, self.middle, self.lower);
        }
        self.middle = self.sma_sum / self.period as f64;
        let atr = self.atr.update_bar(_open, high, low, close, _volume);
        self.upper = self.middle + self.k_multiplier * atr;
        self.lower = self.middle - self.k_multiplier * atr;
        (self.upper, self.middle, self.lower)
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 { upper: self.upper, middle: self.middle, lower: self.lower }
    }
    pub fn is_ready(&self) -> bool {
        self.sma_filled && self.atr.is_ready()
    }
    pub fn reset(&mut self) {
        self.sma_buf.clear();
        self.sma_sum = 0.0;
        self.sma_filled = false;
        self.atr.reset();
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kc_creation() {
        let kc = Kc::new(20, 2.0);
        assert!(!kc.is_ready());
        assert_eq!(kc.value(), IndicatorValue::Channel3 { upper: 0.0, middle: 0.0, lower: 0.0 });
    }

    #[test]
    fn test_kc_warmup() {
        let mut kc = Kc::new(20, 2.0);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(kc.is_ready());
    }

    #[test]
    fn test_kc_band_ordering() {
        let mut kc = Kc::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, middle, lower) = kc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if kc.is_ready() {
                assert!(upper >= middle, "Upper should be >= middle");
                assert!(middle >= lower, "Middle should be >= lower");
            }
        }
    }

    #[test]
    fn test_kc_reset() {
        let mut kc = Kc::new(20, 2.0);
        for i in 0..25 {
            kc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        kc.reset();
        assert!(!kc.is_ready());
        assert_eq!(kc.value(), IndicatorValue::Channel3 { upper: 0.0, middle: 0.0, lower: 0.0 });
    }
} 






















