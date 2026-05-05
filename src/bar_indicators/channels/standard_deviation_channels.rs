//! standard_deviation_channels.rs: High-Performance Standard Deviation Channels
//! Каналы стандартного отклонения - статистически обоснованные каналы
//!
//! Особенности:
//! - Использует готовый LinearRegressionMA компонент для центральной линии
//! - Полосы стандартного отклонения (1σ, 2σ, 3σ) от регрессии
//! - Circular buffer O(1) operations
//! - Адаптивные режимы расчета

use crate::bar_indicators::average::lr::LinearRegressionMA;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;
use arrayvec::ArrayVec;
use serde::{Serialize, Deserialize};

/// Режимы расчета стандартного отклонения
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum StandardDeviationMode {
    /// Простое стандартное отклонение
    Simple,
    /// Популяционное стандартное отклонение (n-1)
    Population,
    /// Адаптивное к волатильности
    Adaptive,
}

/// Источник данных для линейной регрессии
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RegressionSource {
    /// Цена закрытия
    Close,
    /// Типичная цена (H+L+C)/3
    Typical,
    /// Средняя цена (H+L)/2
    Median,
    /// Взвешенная цена (H+L+2*C)/4
    Weighted,
    /// OHLC4 цена (O+H+L+C)/4
    Ohlc4,
}

/// Сигналы каналов стандартного отклонения
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum StandardDeviationSignal {
    /// Сильный пробой вверх (выше 2σ)
    StrongBreakoutUp,
    /// Пробой вверх (выше 1σ)
    BreakoutUp,
    /// Возврат к среднему (к линии регрессии)
    MeanReversion,
    /// Пробой вниз (ниже -1σ)
    BreakoutDown,
    /// Сильный пробой вниз (ниже -2σ)
    StrongBreakoutDown,
    /// Внутри канала
    WithinBands,
}

/// High-Performance Standard Deviation Channels
/// Архитектура: LinearRegressionMA для центральной линии + circular buffer для std dev
#[derive(Debug, Clone)]
pub struct StandardDeviationChannels {
    // Параметры
    period: usize,
    std_multiplier: f64,
    mode: StandardDeviationMode,
    source: RegressionSource,
    ohlcv_source: OhlcvField,
    
    // Компоненты (используем готовые!)
    regression_ma: LinearRegressionMA,  // ✅ Линейная регрессия через готовый компонент
    
    // Circular buffer для расчета отклонений от регрессии - O(1) operations
    price_buffer: ArrayVec<f64, 512>,
    price_index: usize,
    buffer_filled: bool,
    
    // Стандартное отклонение
    std_deviation: f64,
    
    // Полосы каналов
    upper_band_1: f64,   // +1σ
    upper_band_2: f64,   // +2σ
    upper_band_3: f64,   // +3σ
    lower_band_1: f64,   // -1σ
    lower_band_2: f64,   // -2σ
    lower_band_3: f64,   // -3σ
    
    // Адаптивные параметры
    volatility_factor: f64,
    adaptive_multiplier: f64,
    
    // Статистика
    bar_count: usize,
}

impl StandardDeviationChannels {
    /// Создать каналы стандартного отклонения со стандартными параметрами
    pub fn new(period: usize) -> Self {
        Self::new_custom(
            period,
            2.0,
            StandardDeviationMode::Simple,
            RegressionSource::Close
        )
    }
    
    /// Создать каналы с кастомными параметрами
    /// period - период для линейной регрессии и std dev
    /// std_multiplier - множитель стандартного отклонения (обычно 2.0)
    /// mode - режим расчета std dev (простое, популяционное, адаптивное)
    /// source - источник данных (Close, Typical, etc.)
    pub fn new_custom(
        period: usize,
        std_multiplier: f64,
        mode: StandardDeviationMode,
        source: RegressionSource
    ) -> Self {
        assert!(period > 1 && period <= 512, "Period must be between 2 and 512");
        assert!(std_multiplier > 0.0, "Standard deviation multiplier must be positive");

        Self {
            period,
            std_multiplier,
            mode,
            source,
            ohlcv_source: OhlcvField::Close,
            regression_ma: LinearRegressionMA::new(period),  // ✅ Используем готовый компонент
            price_buffer: ArrayVec::new(),
            price_index: 0,
            buffer_filled: false,
            std_deviation: 0.0,
            upper_band_1: 0.0,
            upper_band_2: 0.0,
            upper_band_3: 0.0,
            lower_band_1: 0.0,
            lower_band_2: 0.0,
            lower_band_3: 0.0,
            volatility_factor: 1.0,
            adaptive_multiplier: 1.0,
            bar_count: 0,
        }
    }

    /// Создать каналы с настраиваемым источником данных OHLCV
    pub fn with_source(
        period: usize,
        std_multiplier: f64,
        mode: StandardDeviationMode,
        source: RegressionSource,
        ohlcv_source: OhlcvField
    ) -> Self {
        assert!(period > 1 && period <= 512, "Period must be between 2 and 512");
        assert!(std_multiplier > 0.0, "Standard deviation multiplier must be positive");

        Self {
            period,
            std_multiplier,
            mode,
            source,
            ohlcv_source,
            regression_ma: LinearRegressionMA::new(period),
            price_buffer: ArrayVec::new(),
            price_index: 0,
            buffer_filled: false,
            std_deviation: 0.0,
            upper_band_1: 0.0,
            upper_band_2: 0.0,
            upper_band_3: 0.0,
            lower_band_1: 0.0,
            lower_band_2: 0.0,
            lower_band_3: 0.0,
            volatility_factor: 1.0,
            adaptive_multiplier: 1.0,
            bar_count: 0,
        }
    }
    
    /// Создать каналы с простыми параметрами
    pub fn new_simple(period: usize, std_multiplier: f64) -> Self {
        Self::new_custom(
            period,
            std_multiplier,
            StandardDeviationMode::Simple,
            RegressionSource::Close
        )
    }
    
    /// Создать адаптивные каналы
    pub fn new_adaptive(period: usize) -> Self {
        Self::new_custom(
            period,
            2.0,
            StandardDeviationMode::Adaptive,
            RegressionSource::Typical
        )
    }
    
    /// Обновить каналы новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64, f64) {
        self.bar_count += 1;

        // Получаем цену согласно источнику
        let price = self.get_price_by_source(open, high, low, close, volume);
        
        // ✅ Обновляем линейную регрессию через готовый компонент
        let regression_value = self.regression_ma.update_bar(
            price,    // Передаем нужную цену как open
            price,    // high  
            price,    // low
            price,    // close
            volume
        );
        
        // Обновляем circular buffer для std dev
        self.update_price_buffer(price);
        
        if self.buffer_filled && self.regression_ma.is_ready() {
            // Рассчитываем стандартное отклонение от регрессионной линии
            self.calculate_standard_deviation();
            
            // Обновляем адаптивные параметры
            self.update_adaptive_parameters(high, low);
            
            // Рассчитываем полосы каналов
            self.calculate_bands(regression_value);
        }
        
        (self.upper_band_2, regression_value, self.lower_band_2)
    }
    
    /// Получить цену согласно источнику
    fn get_price_by_source(&self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        // Используем настраиваемый источник OHLCV
        self.ohlcv_source.extract(open, high, low, close, volume)
    }
    
    /// Обновить circular buffer цен - O(1) операция
    fn update_price_buffer(&mut self, price: f64) {
        if self.buffer_filled {
            // Перезаписываем старые значения циклически
            self.price_buffer[self.price_index] = price;
        } else {
            // Заполняем буфер в первый раз
            self.price_buffer.push(price);
        }
        
        // Обновляем индекс циклически
        self.price_index = (self.price_index + 1) % self.period;
        
        // Проверяем заполненность буфера
        if self.price_buffer.len() == self.period && !self.buffer_filled {
            self.buffer_filled = true;
        }
    }
    
    /// Рассчитать стандартное отклонение от регрессионной линии
    fn calculate_standard_deviation(&mut self) {
        let buffer_len = if self.buffer_filled { self.period } else { self.price_buffer.len() };
        let regression_value = self.regression_ma.value();
        
        // Рассчитываем отклонения от регрессионной линии
        let variance = self.price_buffer.iter()
            .take(buffer_len)
            .map(|&price| {
                let diff = price - regression_value.main();
                diff * diff
            })
            .sum::<f64>();
        
        // Применяем режим расчета
        let denominator = match self.mode {
            StandardDeviationMode::Simple => buffer_len as f64,
            StandardDeviationMode::Population => (buffer_len - 1) as f64,
            StandardDeviationMode::Adaptive => {
                // Адаптивное деление с учетом волатильности
                buffer_len as f64 * self.volatility_factor
            }
        };
        
        self.std_deviation = (variance / denominator).sqrt();
    }
    
    /// Обновить адаптивные параметры
    fn update_adaptive_parameters(&mut self, high: f64, low: f64) {
        if matches!(self.mode, StandardDeviationMode::Adaptive) {
            // Используем True Range как меру волатильности
            let true_range = high - low;
            let avg_price = (high + low) / 2.0;
            
            if avg_price > 0.0 {
                let volatility_pct = true_range / avg_price;
                
                // Адаптивный фактор: высокая волатильность = больше чувствительности
                self.volatility_factor = (1.0 + volatility_pct * 10.0).clamp(0.5, 2.0);
                self.adaptive_multiplier = (1.0 + volatility_pct * 2.0).clamp(0.8, 1.5);
            }
        }
    }
    
    /// Рассчитать полосы каналов
    fn calculate_bands(&mut self, regression_value: f64) {
        let effective_std = self.std_deviation * self.adaptive_multiplier;
        
        // Рассчитываем полосы с разными множителями
        self.upper_band_1 = regression_value + 1.0 * effective_std;
        self.lower_band_1 = regression_value - 1.0 * effective_std;
        
        self.upper_band_2 = regression_value + self.std_multiplier * effective_std;
        self.lower_band_2 = regression_value - self.std_multiplier * effective_std;
        
        self.upper_band_3 = regression_value + 3.0 * effective_std;
        self.lower_band_3 = regression_value - 3.0 * effective_std;
    }
    
    /// Получить основные значения (2σ канал)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.upper_band_2,
            middle: self.regression_ma.value().main(),
            lower: self.lower_band_2,
        }
    }

    /// Получить основные значения как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper_band_2, self.regression_ma.value().main(), self.lower_band_2)
    }
    
    /// Получить все полосы (1σ, 2σ, 3σ)
    pub fn all_bands(&self) -> (f64, f64, f64, f64, f64, f64, f64) {
        (
            self.upper_band_3,
            self.upper_band_2,
            self.upper_band_1,
            self.regression_ma.value().main(),
            self.lower_band_1,
            self.lower_band_2,
            self.lower_band_3,
        )
    }
    
    /// Получить значение регрессионной линии
    pub fn regression_line(&self) -> f64 {
        self.regression_ma.value().main()
    }
    
    /// Получить ширину канала
    pub fn channel_width(&self) -> f64 {
        self.upper_band_2 - self.lower_band_2
    }
    
    /// Получить позицию цены в канале
    pub fn position_in_channel(&self, price: f64) -> f64 {
        let width = self.channel_width();
        if width > 0.0 {
            (price - self.lower_band_2) / width
        } else {
            0.5
        }
    }
    
    /// Получить статистику регрессии (slope, intercept, r2, std_dev)
    pub fn regression_stats(&self) -> (f64, f64, f64, f64) {
        (
            self.regression_ma.slope(),
            self.regression_ma.intercept(),
            self.regression_ma.r2(),
            self.std_deviation
        )
    }
    
    /// Получить стандартное отклонение
    pub fn standard_deviation(&self) -> f64 {
        self.std_deviation
    }
    
    /// Генерировать сигнал
    pub fn generate_signal(&self, price: f64) -> StandardDeviationSignal {
        if !self.is_ready() {
            return StandardDeviationSignal::WithinBands;
        }
        
        if price > self.upper_band_2 {
            StandardDeviationSignal::StrongBreakoutUp
        } else if price > self.upper_band_1 {
            StandardDeviationSignal::BreakoutUp
        } else if price < self.lower_band_2 {
            StandardDeviationSignal::StrongBreakoutDown
        } else if price < self.lower_band_1 {
            StandardDeviationSignal::BreakoutDown
        } else {
            let regression_value = self.regression_ma.value().main();
            let distance_to_regression = (price - regression_value).abs();
            let std_distance = distance_to_regression / self.std_deviation;
            
            if std_distance < 0.5 {
                StandardDeviationSignal::MeanReversion
            } else {
                StandardDeviationSignal::WithinBands
            }
        }
    }
    
    /// Проверить пробой уровня
    pub fn is_breakout(&self, price: f64, sigma_level: f64) -> Option<bool> {
        if !self.is_ready() {
            return None;
        }

        let regression_value = self.regression_ma.value().main();
        let threshold = regression_value + sigma_level * self.std_deviation;

        if price > threshold {
            Some(true)  // Пробой вверх
        } else if price < (regression_value - sigma_level * self.std_deviation) {
            Some(false) // Пробой вниз
        } else {
            None // Нет пробоя
        }
    }
    
    /// Проверить сигнал возврата к среднему
    pub fn is_mean_reversion_signal(&self, price: f64, prev_price: f64) -> bool {
        if !self.is_ready() {
            return false;
        }

        let regression_value = self.regression_ma.value().main();

        // Цена движется к регрессионной линии
        let prev_distance = (prev_price - regression_value).abs();
        let current_distance = (price - regression_value).abs();

        current_distance < prev_distance && current_distance < self.std_deviation
    }
    
    /// Получить направление тренда (на основе slope)
    pub fn trend_direction(&self) -> i8 {
        let slope = self.regression_ma.slope();
        
        if slope > 0.001 {
            1  // Восходящий тренд
        } else if slope < -0.001 {
            -1 // Нисходящий тренд
        } else {
            0  // Боковое движение
        }
    }
    
    /// Получить силу тренда (R²)
    pub fn trend_strength(&self) -> f64 {
        self.regression_ma.r2()
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.regression_ma.is_ready() && self.buffer_filled
    }
    
    /// Получить параметры
    pub fn get_params(&self) -> (usize, f64, StandardDeviationMode, RegressionSource) {
        (self.period, self.std_multiplier, self.mode, self.source)
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.regression_ma.reset();
        self.price_buffer.clear();
        self.price_index = 0;
        self.buffer_filled = false;
        self.std_deviation = 0.0;
        self.upper_band_1 = 0.0;
        self.upper_band_2 = 0.0;
        self.upper_band_3 = 0.0;
        self.lower_band_1 = 0.0;
        self.lower_band_2 = 0.0;
        self.lower_band_3 = 0.0;
        self.volatility_factor = 1.0;
        self.adaptive_multiplier = 1.0;
        self.bar_count = 0;
    }
}

impl Default for StandardDeviationChannels {
    fn default() -> Self {
        Self::new(20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_deviation_channels_creation() {
        let sdc = StandardDeviationChannels::new(20);
        assert!(!sdc.is_ready());
        assert_eq!(sdc.channel_width(), 0.0);
    }

    #[test]
    fn test_standard_deviation_channels_warmup() {
        let mut sdc = StandardDeviationChannels::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sdc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sdc.is_ready());
    }

    #[test]
    fn test_standard_deviation_channels_bands() {
        let mut sdc = StandardDeviationChannels::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            sdc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        let (u3, u2, u1, mid, l1, l2, l3) = sdc.all_bands();
        assert!(u3 >= u2);
        assert!(u2 >= u1);
        assert!(u1 >= mid);
        assert!(mid >= l1);
        assert!(l1 >= l2);
        assert!(l2 >= l3);
    }

    #[test]
    fn test_standard_deviation_channels_adaptive() {
        let mut sdc = StandardDeviationChannels::new_adaptive(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            sdc.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
        }
        assert!(sdc.is_ready());
        assert!(sdc.standard_deviation() > 0.0);
    }

    #[test]
    fn test_standard_deviation_channels_reset() {
        let mut sdc = StandardDeviationChannels::new(20);
        for i in 0..25 {
            sdc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        sdc.reset();
        assert!(!sdc.is_ready());
        assert_eq!(sdc.channel_width(), 0.0);
    }
} 






















