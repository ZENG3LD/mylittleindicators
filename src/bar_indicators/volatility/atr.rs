// High-performance Average True Range (ATR)
// (c) 2024

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::utils::true_range::true_range;

#[derive(Debug, Clone)]
pub struct Atr {
    ma: MovingAverageProvider,
    ma_type: MovingAverageType,
    prev_close: Option<f64>,
    value: f64,
}

impl Atr {
    /// 🚀 ATR с поддержкой ВСЕХ 19 типов скользящих средних!
    pub fn new(period: usize, ma_type: MovingAverageType) -> Self {
        Self {
            ma: MovingAverageProvider::new(ma_type, period), // 🚀 Поддерживаем ВСЕ типы MA!
            ma_type,
            prev_close: None,
            value: 0.0,
        }
    }

    /// Создать ATR с традиционным Wilder's smoothing (RMA)
    pub fn new_wilder(period: usize) -> Self {
        Self::new(period, MovingAverageType::RMA)
    }

    /// Создать ATR с Simple Moving Average
    pub fn new_sma(period: usize) -> Self {
        Self::new(period, MovingAverageType::SMA)
    }

    /// Создать ATR с Exponential Moving Average
    pub fn new_ema(period: usize) -> Self {
        Self::new(period, MovingAverageType::EMA)
    }

    /// Обновить ATR новым баром (open, high, low, close, volume)
    /// Всегда использует классический True Range (Wilder): max(high-low, |high-prev_close|, |low-prev_close|)
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        let tr = if let Some(prev_close) = self.prev_close {
            true_range(high, low, prev_close)
        } else {
            high - low
        };

        // 🚀 Используем выбранный тип MA для сглаживания True Range
        let _update = self.ma.update_bar(0.0, 0.0, 0.0, tr, 0.0);
        self.value = self.ma.value().main();
        self.prev_close = Some(close);
        self.value
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    
    pub fn period(&self) -> usize {
        self.ma.period()
    }

    /// 🚀 Получить тип MA используемый для сглаживания
    pub fn ma_type(&self) -> MovingAverageType {
        self.ma_type
    }
    
    pub fn is_ready(&self) -> bool {
        self.ma.is_ready()
    }
    
    pub fn reset(&mut self) {
        self.ma.reset();
        self.prev_close = None;
        self.value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atr_creation() {
        let atr = Atr::new_wilder(14);
        assert!(!atr.is_ready());
        assert_eq!(atr.value().main(), 0.0);
    }

    #[test]
    fn test_atr_warmup() {
        let mut atr = Atr::new_wilder(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            atr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(atr.is_ready());
    }

    #[test]
    fn test_atr_positive() {
        let mut atr = Atr::new_wilder(14);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            let value = atr.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value >= 0.0, "ATR should be non-negative");
        }
    }

    #[test]
    fn test_atr_with_ema() {
        let mut atr = Atr::new_ema(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            atr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(atr.is_ready());
        assert!(atr.value().main() > 0.0);
    }

    #[test]
    fn test_atr_reset() {
        let mut atr = Atr::new_wilder(14);
        for i in 0..20 {
            atr.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        atr.reset();
        assert!(!atr.is_ready());
        assert_eq!(atr.value().main(), 0.0);
    }
}






















