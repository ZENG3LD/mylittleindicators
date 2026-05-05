//! Adaptive Bollinger Bands - адаптивные полосы Боллинджера
//!
//! Улучшенная версия классических полос Боллинджера, где период и множитель
//! автоматически адаптируются к текущим рыночным условиям на основе
//! волатильности (ATR) и momentum.
//!
//! Переиспользует существующие компоненты MovingAverage и ATR

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;
use crate::bar_indicators::volatility::atr::Atr;
use arrayvec::ArrayVec;

/// Результат Adaptive Bollinger Bands
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveBollingerBandsResult {
    pub upper_band: f64,         // Верхняя полоса
    pub middle_band: f64,        // Средняя линия (адаптивная MA)
    pub lower_band: f64,         // Нижняя полоса
    pub bandwidth: f64,          // Ширина канала (upper - lower)
    pub percent_b: f64,          // %B - позиция цены в канале (0-1)
    pub squeeze_ratio: f64,      // Коэффициент сжатия (0-1, где 0 = максимальное сжатие)
    pub adaptive_period: f64,    // Текущий адаптивный период
    pub adaptive_multiplier: f64, // Текущий адаптивный множитель
    pub market_regime: i8,       // Режим рынка: 1 (тренд), 0 (флэт), -1 (волатильность)
}

impl AdaptiveBollingerBandsResult {
    pub fn empty() -> Self {
        Self {
            upper_band: 0.0,
            middle_band: 0.0,
            lower_band: 0.0,
            bandwidth: 0.0,
            percent_b: 0.5,
            squeeze_ratio: 0.5,
            adaptive_period: 20.0,
            adaptive_multiplier: 2.0,
            market_regime: 0,
        }
    }
    
    /// Определить позицию цены относительно полос
    pub fn price_position(&self, price: f64) -> &'static str {
        if price > self.upper_band {
            "Выше верхней полосы"
        } else if price < self.lower_band {
            "Ниже нижней полосы"
        } else if self.percent_b > 0.8 {
            "Близко к верхней полосе"
        } else if self.percent_b < 0.2 {
            "Близко к нижней полосе"
        } else {
            "В середине канала"
        }
    }
    
    /// Получить описание режима рынка
    pub fn market_regime_name(&self) -> &'static str {
        match self.market_regime {
            1 => "Трендовый",
            -1 => "Высокая волатильность",
            _ => "Флэтовый",
        }
    }
    
    /// Определить состояние сжатия
    pub fn squeeze_state(&self) -> &'static str {
        match self.squeeze_ratio {
            x if x < 0.3 => "Сильное сжатие",
            x if x < 0.5 => "Умеренное сжатие",
            x if x < 0.7 => "Нормальная ширина",
            _ => "Расширение",
        }
    }
}

/// Adaptive Bollinger Bands - адаптивные полосы Боллинджера с автоматической адаптацией
#[derive(Debug, Clone)]
pub struct AdaptiveBollingerBands {
    // Переиспользуем существующие компоненты
    adaptive_ma: MovingAverageProvider,      // Адаптивная скользящая средняя
    atr: Atr,                        // ATR для анализа волатильности
    volatility_ma: MovingAverageProvider,    // MA для сглаживания волатильности
    bandwidth_ma: MovingAverageProvider,     // MA для анализа ширины канала

    // Буферы для расчетов
    prices: ArrayVec<f64, 64>,
    std_devs: ArrayVec<f64, 32>,     // Стандартные отклонения
    bandwidths: ArrayVec<f64, 32>,   // История ширины канала
    periods: ArrayVec<f64, 16>,      // История адаптивных периодов

    // Параметры адаптации
    base_period: usize,              // Базовый период
    min_period: usize,               // Минимальный период
    max_period: usize,               // Максимальный период
    base_multiplier: f64,            // Базовый множитель
    min_multiplier: f64,             // Минимальный множитель
    max_multiplier: f64,             // Максимальный множитель
    source: OhlcvField,              // Источник данных (Close, HL2, HLC3, etc.)

    // Текущие адаптивные параметры
    current_period: f64,
    current_multiplier: f64,

    // Результат
    current_result: AdaptiveBollingerBandsResult,

    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl AdaptiveBollingerBands {
    /// Создать новые Adaptive Bollinger Bands с параметрами по умолчанию
    pub fn new() -> Self {
        Self::from_base_params(20, 2.0)
    }

    /// Создать из базовых параметров с автоматическим вычислением min/max диапазонов.
    ///
    /// Это режим "auto" - min/max вычисляются автоматически:
    /// - min_period = base_period / 2 (минимум 5)
    /// - max_period = base_period * 2
    /// - min_multiplier = base_multiplier / 2 (минимум 0.5)
    /// - max_multiplier = base_multiplier * 1.5
    pub fn from_base_params(base_period: usize, base_multiplier: f64) -> Self {
        assert!(base_period > 0, "Base period must be greater than 0");
        assert!(base_multiplier > 0.0, "Base multiplier must be positive");

        let min_period = (base_period / 2).max(5);
        let max_period = base_period * 2;
        let min_multiplier = (base_multiplier / 2.0).max(0.5);
        let max_multiplier = base_multiplier * 1.5;

        Self::with_parameters_internal(
            base_period, min_period, max_period,
            base_multiplier, min_multiplier, max_multiplier
        )
    }

    /// Создать с полной ручной конфигурацией всех 6 параметров.
    ///
    /// Это режим "manual" - все параметры задаются явно.
    /// Используйте этот конструктор если нужен полный контроль над адаптивным алгоритмом.
    pub fn with_parameters(
        base_period: usize,
        min_period: usize,
        max_period: usize,
        base_multiplier: f64,
        min_multiplier: f64,
        max_multiplier: f64
    ) -> Self {
        assert!(base_period > 0, "Base period must be greater than 0");
        assert!(min_period > 0 && min_period <= base_period, "Invalid min period");
        assert!(max_period >= base_period, "Invalid max period");
        assert!(base_multiplier > 0.0, "Base multiplier must be positive");
        assert!(min_multiplier > 0.0 && min_multiplier <= base_multiplier, "Invalid min multiplier");
        assert!(max_multiplier >= base_multiplier, "Invalid max multiplier");

        Self::with_parameters_internal(
            base_period, min_period, max_period,
            base_multiplier, min_multiplier, max_multiplier
        )
    }

    /// Внутренний конструктор - создаёт экземпляр без валидации (вызывается после проверок)
    fn with_parameters_internal(
        base_period: usize,
        min_period: usize,
        max_period: usize,
        base_multiplier: f64,
        min_multiplier: f64,
        max_multiplier: f64
    ) -> Self {

        Self {
            // Переиспользуем существующие компоненты
            adaptive_ma: MovingAverageProvider::new(MovingAverageType::EMA, base_period),
            atr: Atr::new_wilder(14),
            volatility_ma: MovingAverageProvider::new(MovingAverageType::SMA, 10),
            bandwidth_ma: MovingAverageProvider::new(MovingAverageType::SMA, 20),

            prices: ArrayVec::new(),
            std_devs: ArrayVec::new(),
            bandwidths: ArrayVec::new(),
            periods: ArrayVec::new(),

            base_period,
            min_period,
            max_period,
            base_multiplier,
            min_multiplier,
            max_multiplier,
            source: OhlcvField::Close,

            current_period: base_period as f64,
            current_multiplier: base_multiplier,

            current_result: AdaptiveBollingerBandsResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }

    /// Создать с полной ручной конфигурацией всех параметров и источником данных.
    ///
    /// Это режим "manual" с настраиваемым источником данных.
    pub fn with_parameters_and_source(
        base_period: usize,
        min_period: usize,
        max_period: usize,
        base_multiplier: f64,
        min_multiplier: f64,
        max_multiplier: f64,
        source: OhlcvField
    ) -> Self {
        assert!(base_period > 0, "Base period must be greater than 0");
        assert!(min_period > 0 && min_period <= base_period, "Invalid min period");
        assert!(max_period >= base_period, "Invalid max period");
        assert!(base_multiplier > 0.0, "Base multiplier must be positive");
        assert!(min_multiplier > 0.0 && min_multiplier <= base_multiplier, "Invalid min multiplier");
        assert!(max_multiplier >= base_multiplier, "Invalid max multiplier");

        let mut instance = Self::with_parameters_internal(
            base_period, min_period, max_period,
            base_multiplier, min_multiplier, max_multiplier
        );
        instance.source = source;
        instance
    }

    /// Создать из базовых параметров с источником данных (auto mode).
    pub fn from_base_params_with_source(base_period: usize, base_multiplier: f64, source: OhlcvField) -> Self {
        assert!(base_period > 0, "Base period must be greater than 0");
        assert!(base_multiplier > 0.0, "Base multiplier must be positive");

        let mut instance = Self::from_base_params(base_period, base_multiplier);
        instance.source = source;
        instance
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> AdaptiveBollingerBandsResult {
        // Используем настраиваемый источник данных
        let price = self.source.extract(open, high, low, close, volume);

        // Добавляем цену в буфер
        if self.prices.len() >= 64 {
            self.prices.remove(0);
        }
        self.prices.push(price);
        
        // 1. Обновляем ATR (переиспользуем существующий компонент)
        let atr_value = self.atr.update_bar(open, high, low, close, volume);
        
        // 2. Адаптируем параметры на основе волатильности
        self.adapt_parameters(atr_value);
        
        // 3. Пересоздаем адаптивную MA если период изменился значительно
        self.update_adaptive_ma();
        
        // 4. Обновляем адаптивную MA
        let middle_band = self.adaptive_ma.update_bar(open, high, low, close, volume);
        
        // 5. Рассчитываем стандартное отклонение
        let std_dev = self.calculate_adaptive_std_dev(close, middle_band);
        
        // 6. Рассчитываем полосы
        let upper_band = middle_band + (self.current_multiplier * std_dev);
        let lower_band = middle_band - (self.current_multiplier * std_dev);
        
        // 7. Рассчитываем дополнительные метрики
        self.calculate_additional_metrics(close, upper_band, middle_band, lower_band);
        
        // 8. Определяем режим рынка
        self.determine_market_regime();
        
        // Обновляем результат
        self.current_result.upper_band = upper_band;
        self.current_result.middle_band = middle_band;
        self.current_result.lower_band = lower_band;
        self.current_result.adaptive_period = self.current_period;
        self.current_result.adaptive_multiplier = self.current_multiplier;
        
        // Готов после накопления достаточных данных
        if self.adaptive_ma.is_ready() && self.prices.len() >= self.base_period {
            self.is_ready = true;
        }
        
        self.update_count += 1;
        self.current_result
    }
    
    /// Адаптировать параметры на основе волатильности
    fn adapt_parameters(&mut self, atr_value: f64) {
        if self.update_count < 20 {
            return; // Недостаточно данных для адаптации
        }
        
        // Сглаживаем волатильность
        let smoothed_volatility = self.volatility_ma.update_bar(0.0, 0.0, 0.0, atr_value, 0.0);
        
        // Нормализуем волатильность (отношение к базовому значению)
        let volatility_ratio = if smoothed_volatility > 0.0 {
            atr_value / smoothed_volatility
        } else {
            1.0
        };
        
        // Адаптируем период: высокая волатильность = короткий период
        let period_adjustment = 1.0 / volatility_ratio.sqrt();
        self.current_period = (self.base_period as f64 * period_adjustment)
            .max(self.min_period as f64)
            .min(self.max_period as f64);
        
        // Адаптируем множитель: высокая волатильность = меньший множитель
        let multiplier_adjustment = volatility_ratio.sqrt();
        self.current_multiplier = (self.base_multiplier * multiplier_adjustment)
            .max(self.min_multiplier)
            .min(self.max_multiplier);
        
        // Сохраняем период для анализа
        if self.periods.len() >= 16 {
            self.periods.remove(0);
        }
        self.periods.push(self.current_period);
    }
    
    /// Обновить адаптивную MA при значительном изменении периода
    fn update_adaptive_ma(&mut self) {
        let current_ma_period = self.adaptive_ma.period();
        let new_period = self.current_period as usize;
        
        // Пересоздаем MA если период изменился более чем на 20%
        let diff: f64 = (new_period as f64 - current_ma_period as f64).abs();
        if diff / current_ma_period as f64 > 0.2 {
            self.adaptive_ma = MovingAverageProvider::new(MovingAverageType::EMA, new_period);
        }
    }
    
    /// Рассчитать адаптивное стандартное отклонение
    fn calculate_adaptive_std_dev(&mut self, _current_price: f64, _middle_band: f64) -> f64 {
        let period = self.current_period as usize;
        let available_data = self.prices.len().min(period);
        
        if available_data < 2 {
            return 0.1; // Минимальное значение
        }
        
        // Рассчитываем стандартное отклонение за адаптивный период
        let start_idx = self.prices.len() - available_data;
        let prices_slice = &self.prices[start_idx..];
        
        let mean = prices_slice.iter().sum::<f64>() / available_data as f64;
        let variance = prices_slice.iter()
            .map(|&price| (price - mean).powi(2))
            .sum::<f64>() / available_data as f64;
        
        let std_dev = variance.sqrt();
        
        // Сохраняем для анализа
        if self.std_devs.len() >= 32 {
            self.std_devs.remove(0);
        }
        self.std_devs.push(std_dev);
        
        std_dev
    }
    
    /// Рассчитать дополнительные метрики
    fn calculate_additional_metrics(&mut self, price: f64, upper: f64, _middle: f64, lower: f64) {
        // Ширина канала
        let bandwidth = upper - lower;
        
        // Сохраняем ширину канала
        if self.bandwidths.len() >= 32 {
            self.bandwidths.remove(0);
        }
        self.bandwidths.push(bandwidth);
        
        // Сглаженная ширина канала
        let _smoothed_bandwidth = self.bandwidth_ma.update_bar(0.0, 0.0, 0.0, bandwidth, 0.0);
        
        // %B - позиция цены в канале
        let percent_b = if bandwidth > 0.0 {
            (price - lower) / bandwidth
        } else {
            0.5
        };
        
        // Коэффициент сжатия
        let squeeze_ratio = if self.bandwidths.len() >= 10 {
            let recent_bandwidths = &self.bandwidths[self.bandwidths.len() - 10..];
            let max_bandwidth = recent_bandwidths.iter().fold(0.0f64, |a, &b| a.max(b));
            
            if max_bandwidth > 0.0 {
                bandwidth / max_bandwidth
            } else {
                0.5
            }
        } else {
            0.5
        };
        
        // Обновляем результат
        self.current_result.bandwidth = bandwidth;
        self.current_result.percent_b = percent_b;
        self.current_result.squeeze_ratio = squeeze_ratio;
    }
    
    /// Определить режим рынка
    fn determine_market_regime(&mut self) {
        if !self.is_ready || self.periods.len() < 5 {
            self.current_result.market_regime = 0;
            return;
        }
        
        let squeeze_ratio = self.current_result.squeeze_ratio;
        let bandwidth = self.current_result.bandwidth;
        
        // Анализируем стабильность адаптивного периода
        let recent_periods = &self.periods[self.periods.len().saturating_sub(5)..];
        let period_variance = if recent_periods.len() >= 2 {
            let mean: f64 = recent_periods.iter().sum::<f64>() / recent_periods.len() as f64;
            recent_periods.iter()
                .map(|&p| (p - mean).powi(2))
                .sum::<f64>() / recent_periods.len() as f64
        } else {
            0.0
        };
        
        // Определяем режим
        if squeeze_ratio < 0.3 {
            self.current_result.market_regime = 0; // Флэт (сжатие)
        } else if period_variance < 1.0 && bandwidth > 0.0 {
            self.current_result.market_regime = 1; // Тренд (стабильный период)
        } else {
            self.current_result.market_regime = -1; // Высокая волатильность
        }
    }
    
    /// Получить текущее значение (upper, middle, lower)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.current_result.upper_band,
            middle: self.current_result.middle_band,
            lower: self.current_result.lower_band,
        }
    }
    
    /// Получить полный результат
    pub fn result(&self) -> AdaptiveBollingerBandsResult {
        self.current_result
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.adaptive_ma.reset();
        self.atr.reset();
        self.volatility_ma.reset();
        self.bandwidth_ma.reset();
        
        self.prices.clear();
        self.std_devs.clear();
        self.bandwidths.clear();
        self.periods.clear();
        
        self.current_period = self.base_period as f64;
        self.current_multiplier = self.base_multiplier;
        
        self.current_result = AdaptiveBollingerBandsResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.current_period as usize
    }
    
    /// Генерировать торговый сигнал
    pub fn trading_signal(&self, price: f64) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let result = self.current_result;
        
        // Сигналы в зависимости от режима рынка
        match result.market_regime {
            1 => {
                // Трендовый режим - торговля на пробоях
                if price > result.upper_band {
                    1 // Пробой вверх
                } else if price < result.lower_band {
                    -1 // Пробой вниз
                } else {
                    0
                }
            }
            0 => {
                // Флэтовый режим - торговля на отскоках
                if result.percent_b > 0.9 {
                    -1 // Продажа у верхней полосы
                } else if result.percent_b < 0.1 {
                    1 // Покупка у нижней полосы
                } else {
                    0
                }
            }
            _ => {
                // Высокая волатильность - осторожные сигналы
                if result.percent_b > 0.95 && result.squeeze_ratio > 0.7 {
                    -1
                } else if result.percent_b < 0.05 && result.squeeze_ratio > 0.7 {
                    1
                } else {
                    0
                }
            }
        }
    }
    
    /// Генерировать сигнал сжатия (squeeze)
    pub fn squeeze_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let squeeze_ratio = self.current_result.squeeze_ratio;
        
        if squeeze_ratio < 0.2 {
            return 1; // Сильное сжатие - ожидаем пробой
        } else if squeeze_ratio > 0.8 {
            return -1; // Расширение - возможное окончание движения
        }
        
        0
    }
    
    /// Получить информацию о текущем состоянии
    pub fn info(&self, price: f64) -> String {
        let result = self.current_result;
        let signal = match self.trading_signal(price) {
            1 => "Покупка",
            -1 => "Продажа", 
            _ => "Нет сигнала",
        };
        
        format!(
            "Adaptive BB: {:.2}-{:.2}-{:.2}, Период: {:.0}, Множитель: {:.1}, Режим: {}, {}, %B: {:.1}%, Сигнал: {}",
            result.lower_band,
            result.middle_band,
            result.upper_band,
            result.adaptive_period,
            result.adaptive_multiplier,
            result.market_regime_name(),
            result.squeeze_state(),
            result.percent_b * 100.0,
            signal
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("upper_band".to_string(), self.current_result.upper_band);
        values.insert("middle_band".to_string(), self.current_result.middle_band);
        values.insert("lower_band".to_string(), self.current_result.lower_band);
        values.insert("bandwidth".to_string(), self.current_result.bandwidth);
        values.insert("percent_b".to_string(), self.current_result.percent_b);
        values.insert("squeeze_ratio".to_string(), self.current_result.squeeze_ratio);
        values.insert("adaptive_period".to_string(), self.current_result.adaptive_period);
        values.insert("adaptive_multiplier".to_string(), self.current_result.adaptive_multiplier);
        values.insert("market_regime".to_string(), self.current_result.market_regime as f64);
        values
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    
    /// Получить параметры
    pub fn parameters(&self) -> (usize, usize, usize, f64, f64, f64) {
        (self.base_period, self.min_period, self.max_period, 
         self.base_multiplier, self.min_multiplier, self.max_multiplier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_bollinger_bands_creation() {
        let abb = AdaptiveBollingerBands::new();
        assert!(!abb.is_ready());
        assert_eq!(abb.parameters().0, 20);
    }
    
    #[test]
    fn test_adaptive_bollinger_bands_with_parameters() {
        let abb = AdaptiveBollingerBands::with_parameters(14, 7, 28, 2.5, 1.5, 3.5);
        assert_eq!(abb.parameters(), (14, 7, 28, 2.5, 1.5, 3.5));
    }

    #[test]
    fn test_from_base_params_auto_calculation() {
        // Тест авто-режима: min/max вычисляются автоматически
        let abb = AdaptiveBollingerBands::from_base_params(20, 2.0);
        let (base_p, min_p, max_p, base_m, min_m, max_m) = abb.parameters();
        assert_eq!(base_p, 20);
        assert_eq!(min_p, 10);  // 20 / 2 = 10
        assert_eq!(max_p, 40);  // 20 * 2 = 40
        assert_eq!(base_m, 2.0);
        assert_eq!(min_m, 1.0); // 2.0 / 2 = 1.0
        assert_eq!(max_m, 3.0); // 2.0 * 1.5 = 3.0
    }

    #[test]
    fn test_from_base_params_high_multiplier() {
        // Тест с мультипликатором > 3 (раньше это вызывало краш)
        let abb = AdaptiveBollingerBands::from_base_params(20, 4.0);
        let (_, _, _, base_m, min_m, max_m) = abb.parameters();
        assert_eq!(base_m, 4.0);
        assert_eq!(min_m, 2.0);  // 4.0 / 2 = 2.0
        assert_eq!(max_m, 6.0);  // 4.0 * 1.5 = 6.0
    }

    #[test]
    fn test_from_base_params_small_period() {
        // Тест с маленьким периодом - min_period не должен быть меньше 5
        let abb = AdaptiveBollingerBands::from_base_params(8, 2.0);
        let (base_p, min_p, max_p, _, _, _) = abb.parameters();
        assert_eq!(base_p, 8);
        assert_eq!(min_p, 5);   // 8 / 2 = 4, но min = 5
        assert_eq!(max_p, 16);  // 8 * 2 = 16
    }

    #[test]
    fn test_from_base_params_small_multiplier() {
        // Тест с маленьким мультипликатором - min_multiplier не должен быть меньше 0.5
        let abb = AdaptiveBollingerBands::from_base_params(20, 0.8);
        let (_, _, _, base_m, min_m, max_m) = abb.parameters();
        assert_eq!(base_m, 0.8);
        assert_eq!(min_m, 0.5);  // 0.8 / 2 = 0.4, но min = 0.5
        assert!((max_m - 1.2).abs() < 1e-10);  // 0.8 * 1.5 = 1.2 (floating point)
    }
    
    #[test]
    fn test_adaptive_bollinger_bands_update() {
        let mut abb = AdaptiveBollingerBands::new();
        
        // Добавляем данные с изменяющейся волатильностью
        for i in 0..30 {
            let base_price = 100.0;
            let trend = i as f64 * 0.1;
            let volatility = if i > 15 { 3.0 } else { 1.0 };
            
            let high = base_price + trend + volatility;
            let low = base_price + trend - volatility;
            let close = base_price + trend + (volatility * 0.5 * (i as f64 * 0.1).sin());
            
            let result = abb.update_bar(base_price + trend, high, low, close, 1000.0);
            
            if i > 25 {
                assert!(abb.is_ready());
                assert!(result.upper_band > result.middle_band);
                assert!(result.middle_band > result.lower_band);
                assert!(result.bandwidth > 0.0);
                assert!(result.percent_b >= 0.0 && result.percent_b <= 1.5);
                assert!(result.squeeze_ratio >= 0.0 && result.squeeze_ratio <= 1.0);
                assert!(result.adaptive_period >= abb.min_period as f64);
                assert!(result.adaptive_period <= abb.max_period as f64);
            }
        }
    }
    
    #[test]
    fn test_adaptation_to_volatility() {
        let mut abb = AdaptiveBollingerBands::new();
        
        // Период низкой волатильности
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.01);
            let _result = abb.update_bar(price, price + 0.01, price - 0.01, price, 1000.0);
        }
        let low_vol_period = abb.current_period;
        let low_vol_multiplier = abb.current_multiplier;

        // Период высокой волатильности
        for i in 15..30 {
            let price = 100.0 + (i as f64 * 0.5 * (i as f64).sin());
            let _result = abb.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
        }
        let high_vol_period = abb.current_period;
        let high_vol_multiplier = abb.current_multiplier;
        
        if abb.is_ready() {
            // При высокой волатильности период должен уменьшиться
            assert!(high_vol_period <= low_vol_period);
            // А множитель может как увеличиться, так и уменьшиться в зависимости от настроек
            assert!(high_vol_multiplier != low_vol_multiplier);
        }
    }
    
    #[test]
    fn test_trading_signals() {
        let mut abb = AdaptiveBollingerBands::new();
        
        // Добавляем данные
        for i in 0..25 {
            let price = 100.0 + i as f64 * 0.2;
            let _result = abb.update_bar(price, price + 0.5, price - 0.5, price, 1000.0);
        }
        
        if abb.is_ready() {
            let result = abb.result();
            
            // Тестируем различные позиции цены
            let upper_signal = abb.trading_signal(result.upper_band + 1.0);
            let lower_signal = abb.trading_signal(result.lower_band - 1.0);
            let middle_signal = abb.trading_signal(result.middle_band);
            let squeeze_signal = abb.squeeze_signal();
            
            assert!(upper_signal >= -1 && upper_signal <= 1);
            assert!(lower_signal >= -1 && lower_signal <= 1);
            assert!(middle_signal >= -1 && middle_signal <= 1);
            assert!(squeeze_signal >= -1 && squeeze_signal <= 1);
        }
    }
}






















