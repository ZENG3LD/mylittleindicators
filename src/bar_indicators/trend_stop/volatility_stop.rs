//! Volatility Stop - индикатор адаптивных динамических уровней на основе волатильности
//! 
//! Вычисляет адаптивные уровни на основе различных мер волатильности:
//! - Standard Deviation - стандартное отклонение цен
//! - Average True Range (ATR) - средний истинный диапазон
//! - Range - простой диапазон High-Low
//! 
//! Уровни адаптируются к текущей волатильности рынка и могут использовать
//! разные типы сглаживания для более стабильной работы.
//! 
//! Индикатор НЕ содержит логику стопов - только возвращает адаптивные уровни.
//! Логика остановки позиций реализуется в стратегиях на основе этих уровней.

use arrayvec::ArrayVec;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Тип волатильности для расчета уровней
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolatilityType {
    /// Стандартное отклонение цен закрытия
    StandardDeviation,
    /// Average True Range
    ATR,
    /// Простой диапазон High-Low
    Range,
}

/// Volatility Stop индикатор - адаптивные уровни на основе волатильности
#[derive(Debug, Clone)]
pub struct VolatilityStop {
    period: usize,
    multiplier: f64,
    volatility_type: VolatilityType,
    ma_type: MovingAverageType,
    
    // Индикаторы
    atr: Option<Atr>,
    price_ma: MovingAverageProvider,
    
    // Буферы для расчетов
    closes: ArrayVec<f64, 512>,
    ranges: ArrayVec<f64, 512>,
    
    // Текущие значения
    current_price: f64,
    current_volatility: f64,
    long_stop: f64,
    short_stop: f64,
    
    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl VolatilityStop {
    /// Создать Volatility Stop с параметрами по умолчанию
    /// (period=20, multiplier=2.0, StandardDeviation, SMA)
    pub fn new() -> Self {
        Self::with_params(20, 2.0, VolatilityType::StandardDeviation, MovingAverageType::SMA)
    }
    
    /// Создать Volatility Stop с настраиваемыми параметрами
    pub fn with_params(
        period: usize,
        multiplier: f64,
        volatility_type: VolatilityType,
        ma_type: MovingAverageType,
    ) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        assert!(multiplier > 0.0, "Multiplier must be greater than 0");
        
        let atr = if volatility_type == VolatilityType::ATR {
            Some(Atr::new(period, MovingAverageType::RMA))
        } else {
            None
        };
        
        Self {
            period,
            multiplier,
            volatility_type,
            ma_type,
            atr,
            price_ma: MovingAverageProvider::new(ma_type, period),
            closes: ArrayVec::new(),
            ranges: ArrayVec::new(),
            current_price: 0.0,
            current_volatility: 0.0,
            long_stop: 0.0,
            short_stop: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Создать для оптимизации с широким диапазоном параметров
    pub fn for_optimization(
        period: usize,
        multiplier: f64,
        volatility_type: VolatilityType,
        ma_type: MovingAverageType,
    ) -> Self {
        Self::with_params(period, multiplier, volatility_type, ma_type)
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64) {
        self.bars_count += 1;
        
        // Обновляем среднюю цену (используем типичную цену HLC/3)
        let typical_price = (high + low + close) / 3.0;
        self.current_price = self.price_ma.update_bar(typical_price, typical_price, typical_price, typical_price, volume);
        
        // Вычисляем волатильность в зависимости от типа
        self.current_volatility = match self.volatility_type {
            VolatilityType::ATR => {
                if let Some(ref mut atr) = self.atr {
                    atr.update_bar(open, high, low, close, volume)
                } else {
                    0.0
                }
            },
            VolatilityType::StandardDeviation => {
                // Добавляем цену в буфер
                if self.closes.len() >= self.period {
                    self.closes.remove(0);
                }
                self.closes.push(close);
                
                if self.closes.len() >= 2 {
                    self.calculate_standard_deviation()
                } else {
                    0.0
                }
            },
            VolatilityType::Range => {
                // Добавляем диапазон в буфер
                let range = high - low;
                if self.ranges.len() >= self.period {
                    self.ranges.remove(0);
                }
                self.ranges.push(range);
                
                if !self.ranges.is_empty() {
                    self.ranges.iter().sum::<f64>() / self.ranges.len() as f64
                } else {
                    0.0
                }
            },
        };
        
        // Вычисляем уровни на основе средней цены и волатильности
        let volatility_offset = self.current_volatility * self.multiplier;
        self.long_stop = self.current_price - volatility_offset;
        self.short_stop = self.current_price + volatility_offset;
        
        // Готов когда есть достаточно данных
        self.is_ready = self.bars_count >= self.period && self.current_volatility > 0.0;
        
        (self.long_stop, self.short_stop)
    }
    
    /// Вычислить стандартное отклонение
    fn calculate_standard_deviation(&self) -> f64 {
        if self.closes.len() < 2 {
            return 0.0;
        }
        
        let mean = self.closes.iter().sum::<f64>() / self.closes.len() as f64;
        let variance = self.closes.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / self.closes.len() as f64;
        
        variance.sqrt()
    }
    
    /// Получить уровень для лонг позиций (стоп снизу)
    pub fn long_stop(&self) -> f64 {
        self.long_stop
    }
    
    /// Получить уровень для шорт позиций (стоп сверху)
    pub fn short_stop(&self) -> f64 {
        self.short_stop
    }
    
    /// Получить основной уровень (лонг по умолчанию)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.long_stop)
    }
    
    /// Получить оба уровня как кортеж (long, short)
    pub fn levels(&self) -> (f64, f64) {
        (self.long_stop, self.short_stop)
    }
    
    /// Получить текущую среднюю цену
    pub fn price(&self) -> f64 {
        self.current_price
    }
    
    /// Получить текущую волатильность
    pub fn volatility(&self) -> f64 {
        self.current_volatility
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        if let Some(ref mut atr) = self.atr {
            atr.reset();
        }
        self.price_ma.reset();
        self.closes.clear();
        self.ranges.clear();
        self.current_price = 0.0;
        self.current_volatility = 0.0;
        self.long_stop = 0.0;
        self.short_stop = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Получить параметры индикатора
    pub fn params(&self) -> (usize, f64, VolatilityType, MovingAverageType) {
        (self.period, self.multiplier, self.volatility_type, self.ma_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volatility_stop_creation() {
        let ind = VolatilityStop::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_volatility_stop_with_params() {
        let ind = VolatilityStop::with_params(20, 2.5, VolatilityType::StandardDeviation, MovingAverageType::SMA);
        assert!(!ind.is_ready());
        let (period, mult, _, _) = ind.params();
        assert_eq!(period, 20);
        assert_eq!(mult, 2.5);
    }

    #[test]
    fn test_volatility_stop_warmup() {
        let mut ind = VolatilityStop::with_params(10, 2.0, VolatilityType::StandardDeviation, MovingAverageType::SMA);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_volatility_stop_values_finite() {
        let mut ind = VolatilityStop::with_params(10, 2.0, VolatilityType::StandardDeviation, MovingAverageType::SMA);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (long, short) = ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(long.is_finite());
            assert!(short.is_finite());
        }
    }

    #[test]
    fn test_volatility_stop_atr_type() {
        let mut ind = VolatilityStop::with_params(10, 2.0, VolatilityType::ATR, MovingAverageType::SMA);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
        assert!(ind.volatility() > 0.0);
    }

    #[test]
    fn test_volatility_stop_reset() {
        let mut ind = VolatilityStop::with_params(10, 2.0, VolatilityType::StandardDeviation, MovingAverageType::SMA);
        for i in 0..20 {
            ind.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
} 