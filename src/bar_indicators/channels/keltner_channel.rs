//! keltner_channel.rs: High-Performance Keltner Channels
//! Каналы Кельтнера - ATR-based адаптивные каналы
//! 
//! Особенности:
//! - Использует готовые MovingAverage и Atr компоненты
//! - 3 режима расчета центральной линии
//! - ALL 19 MA types для центральной линии и ATR
//! - Полная поддержка оптимизации через перебор типов

use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;
use crate::bar_indicators::volatility::atr::Atr;
use serde::{Serialize, Deserialize};

/// Режимы расчета Keltner Channel
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub enum KeltnerMode {
    /// Classic - использует Typical Price (HLC/3) для средней линии
    #[default]
    Classic,
    /// Close - использует Close для средней линии  
    Close,
    /// HLC - использует (High + Low + Close) / 3 для средней линии
    HLC,
}


/// High-Performance Keltner Channel
/// Архитектура: MovingAverageProvider для центральной линии + Atr для полос
#[derive(Debug, Clone)]
pub struct KeltnerChannel {
    // Параметры
    period: usize,
    multiplier: f64,
    mode: KeltnerMode,
    ma_type: MovingAverageType,
    atr_ma_type: MovingAverageType,
    source: OhlcvField,

    // Компоненты (используем готовые!)
    center_ma: MovingAverageProvider,     // ✅ Центральная линия через MovingAverage
    atr: Atr,                     // ✅ ATR через готовый компонент

    // Текущие значения канала
    upper: f64,
    middle: f64,
    lower: f64,
}

impl KeltnerChannel {
    /// Создать Keltner Channel с указанными параметрами
    /// period - период для MA и ATR
    /// multiplier - множитель ATR для ширины канала
    /// mode - режим расчета цены для центральной линии
    /// ma_type - тип MA для центральной линии (SMA, EMA, KAMA, etc.)
    /// atr_ma_type - тип MA для сглаживания ATR (Wilder, SMA, EMA, etc.)
    pub fn new(
        period: usize,
        multiplier: f64,
        mode: KeltnerMode,
        ma_type: MovingAverageType,
        atr_ma_type: MovingAverageType
    ) -> Self {
        Self::with_source(period, multiplier, mode, ma_type, atr_ma_type, OhlcvField::Close)
    }

    /// Создать Keltner Channel с указанными параметрами и пользовательским источником
    /// period - период для MA и ATR
    /// multiplier - множитель ATR для ширины канала
    /// mode - режим расчета цены для центральной линии
    /// ma_type - тип MA для центральной линии (SMA, EMA, KAMA, etc.)
    /// atr_ma_type - тип MA для сглаживания ATR (Wilder, SMA, EMA, etc.)
    /// source - источник данных (Close, HL2, HLC3, etc.)
    pub fn with_source(
        period: usize,
        multiplier: f64,
        mode: KeltnerMode,
        ma_type: MovingAverageType,
        atr_ma_type: MovingAverageType,
        source: OhlcvField,
    ) -> Self {
        assert!(period > 0, "Period must be positive");
        assert!(multiplier > 0.0, "Multiplier must be positive");

        Self {
            period,
            multiplier,
            mode,
            ma_type,
            atr_ma_type,
            source,
            center_ma: MovingAverageProvider::new(ma_type, period),
            atr: Atr::new(period, atr_ma_type),
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }
    
    /// Создать Classic Keltner Channel (Typical Price, SMA, Wilder ATR)
    pub fn new_classic(period: usize, multiplier: f64) -> Self {
        Self::new_classic_with_source(period, multiplier, OhlcvField::Close)
    }

    /// Создать Classic Keltner Channel с указанным источником данных
    pub fn new_classic_with_source(period: usize, multiplier: f64, source: OhlcvField) -> Self {
        Self::with_source(
            period,
            multiplier,
            KeltnerMode::Classic,
            MovingAverageType::SMA,
            MovingAverageType::RMA,
            source,
        )
    }
    
    /// Создать Keltner Channel с SMA для средней линии
    pub fn new_sma(period: usize, multiplier: f64, atr_ma_type: MovingAverageType) -> Self {
        Self::new(
            period, 
            multiplier, 
            KeltnerMode::Classic, 
            MovingAverageType::SMA, 
            atr_ma_type
        )
    }
    
    /// Создать Keltner Channel с EMA для средней линии
    pub fn new_ema(period: usize, multiplier: f64, atr_ma_type: MovingAverageType) -> Self {
        Self::new(
            period, 
            multiplier, 
            KeltnerMode::Classic, 
            MovingAverageType::EMA, 
            atr_ma_type
        )
    }
    
    /// Создать Keltner Channel с KAMA для средней линии
    pub fn new_kama(period: usize, multiplier: f64, atr_ma_type: MovingAverageType) -> Self {
        Self::new(
            period, 
            multiplier, 
            KeltnerMode::Classic, 
            MovingAverageType::AMA, 
            atr_ma_type
        )
    }
    
    /// Обновить канал новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64, f64) {
        // Используем настроенный источник данных для центральной линии
        let center_price = self.source.extract(open, high, low, close, volume);

        // Обновляем компоненты через готовые индикаторы
        // ✅ MovingAverage автоматически обрабатывает ВСЕ типы MA
        self.middle = self.center_ma.update_bar(
            center_price,  // Передаем нужную цену как open
            center_price,  // high
            center_price,  // low
            center_price,  // close
            volume
        );

        // ✅ Atr автоматически использует указанный тип MA для сглаживания
        let atr_value = self.atr.update_bar(open, high, low, close, volume);

        // Рассчитываем границы канала
        if self.is_ready() {
            self.upper = self.middle + self.multiplier * atr_value;
            self.lower = self.middle - self.multiplier * atr_value;
        } else {
            self.upper = 0.0;
            self.lower = 0.0;
        }

        (self.upper, self.middle, self.lower)
    }
    
    /// Получить текущие значения канала как типизированный IndicatorValue
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.upper,
            middle: self.middle,
            lower: self.lower,
        }
    }

    /// Получить текущие значения канала как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper, self.middle, self.lower)
    }
    
    /// Получить верхнюю границу канала
    pub fn upper(&self) -> f64 {
        self.upper
    }
    
    /// Получить среднюю линию канала
    pub fn middle(&self) -> f64 {
        self.middle
    }
    
    /// Получить нижнюю границу канала
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
        let width = self.channel_width();
        if width > 0.0 {
            (price - self.lower) / width
        } else {
            0.5 // Нейтральная позиция если канал не готов
        }
    }
    
    /// Получить расстояние цены от центральной линии в единицах ATR
    pub fn distance_from_center_atr(&self, price: f64) -> f64 {
        if self.is_ready() && self.atr.value().main() > 0.0 {
            (price - self.middle) / self.atr.value().main()
        } else {
            0.0
        }
    }
    
    /// Проверить пробой верхней границы
    pub fn is_upper_breakout(&self, price: f64) -> bool {
        self.is_ready() && price > self.upper
    }
    
    /// Проверить пробой нижней границы  
    pub fn is_lower_breakout(&self, price: f64) -> bool {
        self.is_ready() && price < self.lower
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.center_ma.is_ready() && self.atr.is_ready()
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.center_ma.reset();
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
    
    /// Получить режим
    pub fn mode(&self) -> KeltnerMode {
        self.mode
    }
    
    /// Получить тип MA для центральной линии
    pub fn ma_type(&self) -> MovingAverageType {
        self.ma_type
    }
    
    /// Получить тип MA для ATR
    pub fn atr_ma_type(&self) -> MovingAverageType {
        self.atr_ma_type
    }
    
    /// Получить текущее значение ATR
    pub fn atr_value(&self) -> f64 {
        self.atr.value().main()
    }
}

impl Default for KeltnerChannel {
    fn default() -> Self {
        Self::new_classic(20, 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keltner_channel_creation() {
        let kc = KeltnerChannel::new_classic(20, 2.0);
        assert!(!kc.is_ready());
        assert_eq!(kc.period(), 20);
        assert_eq!(kc.multiplier(), 2.0);
    }

    #[test]
    fn test_keltner_channel_warmup() {
        let mut kc = KeltnerChannel::new_classic(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(kc.is_ready());
    }

    #[test]
    fn test_keltner_channel_values() {
        let mut kc = KeltnerChannel::new_classic(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, middle, lower) = kc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if kc.is_ready() {
                assert!(upper > middle, "Upper should be > middle");
                assert!(middle > lower, "Middle should be > lower");
            }
        }
    }

    #[test]
    fn test_keltner_channel_reset() {
        let mut kc = KeltnerChannel::new_classic(20, 2.0);
        for i in 0..25 {
            kc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        kc.reset();
        assert!(!kc.is_ready());
    }

    #[test]
    fn test_keltner_channel_with_source() {
        // Test with Close (default)
        let mut kc_close = KeltnerChannel::new_classic_with_source(3, 2.0, OhlcvField::Close);

        // Test with HL2
        let mut kc_hl2 = KeltnerChannel::new_classic_with_source(3, 2.0, OhlcvField::HL2);

        // Test data: (open, high, low, close, volume)
        let bars = vec![
            (100.0, 110.0, 90.0, 105.0, 1000.0),
            (105.0, 115.0, 95.0, 110.0, 1200.0),
            (110.0, 120.0, 100.0, 115.0, 800.0),
            (115.0, 125.0, 105.0, 120.0, 900.0),
        ];

        for (o, h, l, c, v) in &bars {
            kc_close.update_bar(*o, *h, *l, *c, *v);
            kc_hl2.update_bar(*o, *h, *l, *c, *v);
        }

        // Values should be different because different sources are used
        assert_ne!(kc_close.middle(), kc_hl2.middle(),
                   "Middle values should differ when using different sources");

        // Both should be ready after 4 bars (period = 3)
        assert!(kc_close.is_ready());
        assert!(kc_hl2.is_ready());
    }
} 






















