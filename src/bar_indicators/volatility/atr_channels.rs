// High-performance ATR Channels (ATR Bands)
// (c) 2024

use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// ATR Channels: Middle = MA(Close), Upper = Middle + k*ATR, Lower = Middle - k*ATR
#[derive(Clone)]
pub struct AtrChannels {
    ma: MovingAverageProvider,
    atr: Atr,
    k: f64,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl AtrChannels {
    /// Создать ATR Channels
    /// period - период для MA
    /// ma_type - тип MA (SMA, EMA, RMA)
    /// atr_period - период ATR
    /// atr_type - тип MA для ATR (SMA, EMA, RMA)
    /// k - множитель ATR
    pub fn new(period: usize, ma_type: MovingAverageType, atr_period: usize, atr_type: MovingAverageType, k: f64) -> Self {
        Self {
            ma: MovingAverageProvider::new(ma_type, period),
            atr: Atr::new(atr_period, atr_type),
            k,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }
    /// Обновить канал новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) {
        let ma_val = self.ma.update_bar(open, high, low, close, volume);
        let atr_val = self.atr.update_bar(open, high, low, close, volume);
        self.middle = ma_val;
        self.upper = ma_val + self.k * atr_val;
        self.lower = ma_val - self.k * atr_val;
    }
    pub fn upper(&self) -> f64 { self.upper }
    pub fn middle(&self) -> f64 { self.middle }
    pub fn lower(&self) -> f64 { self.lower }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.upper,
            middle: self.middle,
            lower: self.lower,
        }
    }
    pub fn is_ready(&self) -> bool { self.ma.is_ready() && self.atr.is_ready() }
    pub fn reset(&mut self) {
        self.ma.reset();
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
    fn test_atr_channels_creation() {
        let ac = AtrChannels::new(20, MovingAverageType::SMA, 14, MovingAverageType::RMA, 2.0);
        assert!(!ac.is_ready());
        assert_eq!(ac.upper(), 0.0);
        assert_eq!(ac.middle(), 0.0);
        assert_eq!(ac.lower(), 0.0);
    }

    #[test]
    fn test_atr_channels_warmup() {
        let mut ac = AtrChannels::new(20, MovingAverageType::SMA, 14, MovingAverageType::RMA, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ac.is_ready());
    }

    #[test]
    fn test_atr_channels_band_ordering() {
        let mut ac = AtrChannels::new(20, MovingAverageType::SMA, 14, MovingAverageType::RMA, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if ac.is_ready() {
                assert!(ac.upper() >= ac.middle(), "Upper should be >= middle");
                assert!(ac.middle() >= ac.lower(), "Middle should be >= lower");
            }
        }
    }

    #[test]
    fn test_atr_channels_reset() {
        let mut ac = AtrChannels::new(20, MovingAverageType::SMA, 14, MovingAverageType::RMA, 2.0);
        for i in 0..25 {
            ac.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ac.reset();
        assert!(!ac.is_ready());
        assert_eq!(ac.upper(), 0.0);
        assert_eq!(ac.middle(), 0.0);
        assert_eq!(ac.lower(), 0.0);
    }
}






















