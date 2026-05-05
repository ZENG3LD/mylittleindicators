//! atr_channels.rs: High-Performance ATR Channels
//! Улучшенная реализация ATR каналов

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::average::moving_average::MovingAverageProvider;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::volatility::atr::Atr;
use serde::{Serialize, Deserialize};

/// Режимы расчета ATR Channels
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub enum AtrChannelMode {
    /// Close - использует Close цену для средней линии
    #[default]
    Close,
    /// Typical - использует Typical Price (HLC/3) для средней линии
    Typical,
    /// OHLC - использует (Open + High + Low + Close) / 4 для средней линии
    OHLC,
    /// HL - использует (High + Low) / 2 для средней линии
    HL,
}


/// High-Performance ATR Channels
#[derive(Debug, Clone)]
pub struct AtrChannels {
    period: usize,
    multiplier: f64,
    mode: AtrChannelMode,
    ma_type: MovingAverageType,
    atr_ma_type: MovingAverageType,
    
    // MovingAverage для средней линии
    ma: MovingAverageProvider,
    
    // ATR для расчета каналов
    atr: Atr,
    
    // Текущие значения канала
    upper: f64,
    middle: f64,
    lower: f64,
}

impl AtrChannels {
    /// Создать ATR Channels с указанными параметрами
    pub fn new(
        period: usize, 
        multiplier: f64, 
        mode: AtrChannelMode, 
        ma_type: MovingAverageType, 
        atr_ma_type: MovingAverageType
    ) -> Self {
        assert!(period > 0, "Period must be positive");
        assert!(multiplier > 0.0, "Multiplier must be positive");
        
        Self {
            period,
            multiplier,
            mode,
            ma_type,
            atr_ma_type,
            ma: MovingAverageProvider::new(ma_type, period),
            atr: Atr::new(period, atr_ma_type),
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }
    
    /// Создать классические ATR Channels (Close, SMA, Wilder ATR)
    pub fn new_classic(period: usize, multiplier: f64) -> Self {
        Self::new(period, multiplier, AtrChannelMode::Close, MovingAverageType::SMA, MovingAverageType::RMA)
    }
    
    /// Обновить каналы новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64, f64) {
        // Выбираем цену для средней линии в зависимости от режима
        let _price = match self.mode {
            AtrChannelMode::Close => close,
            AtrChannelMode::Typical => (high + low + close) / 3.0,
            AtrChannelMode::OHLC => (open + high + low + close) / 4.0,
            AtrChannelMode::HL => (high + low) / 2.0,
        };
        
        // Обновляем среднюю линию
        self.middle = self.ma.update_bar(open, high, low, close, volume);
        
        // Обновляем ATR
        let atr_value = self.atr.update_bar(open, high, low, close, volume);
        
        // Рассчитываем границы каналов
        if self.is_ready() {
            self.upper = self.middle + self.multiplier * atr_value;
            self.lower = self.middle - self.multiplier * atr_value;
        } else {
            self.upper = 0.0;
            self.lower = 0.0;
        }
        
        (self.upper, self.middle, self.lower)
    }
    
    /// Получить текущие значения каналов как типизированный IndicatorValue
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.upper,
            middle: self.middle,
            lower: self.lower,
        }
    }

    /// Получить текущие значения каналов как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper, self.middle, self.lower)
    }
    
    /// Получить верхний канал
    pub fn upper(&self) -> f64 {
        self.upper
    }
    
    /// Получить среднюю линию
    pub fn middle(&self) -> f64 {
        self.middle
    }
    
    /// Получить нижний канал
    pub fn lower(&self) -> f64 {
        self.lower
    }
    
    /// Получить ширину канала
    pub fn channel_width(&self) -> f64 {
        if self.is_ready() {
            self.upper - self.lower
        } else {
            0.0
        }
    }
    
    /// Получить позицию цены в канале (0.0 = нижняя граница, 1.0 = верхняя граница)
    pub fn position_in_channel(&self, price: f64) -> f64 {
        if !self.is_ready() || self.upper == self.lower {
            0.5 // По центру если канал не готов или нулевой ширины
        } else {
            ((price - self.lower) / (self.upper - self.lower)).clamp(0.0, 1.0)
        }
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.ma.is_ready() && self.atr.is_ready()
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.ma.reset();
        self.atr.reset();
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Получить множитель
    pub fn multiplier(&self) -> f64 {
        self.multiplier
    }
    
    /// Получить режим расчета
    pub fn mode(&self) -> AtrChannelMode {
        self.mode
    }
    
    /// Получить тип MA для средней линии
    pub fn ma_type(&self) -> MovingAverageType {
        self.ma_type
    }
    
    /// Получить тип MA для ATR
    pub fn atr_ma_type(&self) -> MovingAverageType {
        self.atr_ma_type
    }
}

impl Default for AtrChannels {
    fn default() -> Self {
        Self::new_classic(14, 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atr_channels_creation() {
        let ac = AtrChannels::new_classic(14, 2.0);
        assert!(!ac.is_ready());
        assert_eq!(ac.period(), 14);
        assert_eq!(ac.multiplier(), 2.0);
    }

    #[test]
    fn test_atr_channels_warmup() {
        let mut ac = AtrChannels::new_classic(14, 2.0);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ac.is_ready());
    }

    #[test]
    fn test_atr_channels_values() {
        let mut ac = AtrChannels::new_classic(14, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, middle, lower) = ac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if ac.is_ready() {
                assert!(upper > middle, "Upper should be > middle");
                assert!(middle > lower, "Middle should be > lower");
            }
        }
    }

    #[test]
    fn test_atr_channels_reset() {
        let mut ac = AtrChannels::new_classic(14, 2.0);
        for i in 0..20 {
            ac.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ac.reset();
        assert!(!ac.is_ready());
    }
} 






















