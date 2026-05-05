//! Volatility Breakout Detector - детектор пробоев волатильности
//!
//! Обнаруживает моменты резкого увеличения волатильности, которые часто
//! предшествуют крупным движениям цены. Использует многоуровневый анализ
//! волатильности и momentum.
//!
//! Переиспользует существующие компоненты ATR и MovingAverage

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;
use arrayvec::ArrayVec;

/// Тип пробоя волатильности
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakoutType {
    None,           // Нет пробоя
    Mild,           // Умеренный пробой
    Strong,         // Сильный пробой
    Extreme,        // Экстремальный пробой
    Squeeze,        // Сжатие волатильности
}

impl BreakoutType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BreakoutType::None => "Нет пробоя",
            BreakoutType::Mild => "Умеренный пробой",
            BreakoutType::Strong => "Сильный пробой",
            BreakoutType::Extreme => "Экстремальный пробой",
            BreakoutType::Squeeze => "Сжатие волатильности",
        }
    }
    
    pub fn as_number(&self) -> i8 {
        match self {
            BreakoutType::None => 0,
            BreakoutType::Mild => 1,
            BreakoutType::Strong => 2,
            BreakoutType::Extreme => 3,
            BreakoutType::Squeeze => -1,
        }
    }
}

/// Результат Volatility Breakout Detector
#[derive(Debug, Clone, Copy)]
pub struct VolatilityBreakoutResult {
    pub breakout_type: BreakoutType,     // Тип пробоя
    pub volatility_ratio: f64,           // Отношение текущей к средней волатильности
    pub breakout_strength: f64,          // Сила пробоя (0.0-3.0+)
    pub momentum_factor: f64,            // Фактор momentum (-1.0 до 1.0)
    pub squeeze_duration: usize,         // Продолжительность сжатия в барах
    pub breakout_probability: f64,       // Вероятность пробоя (0.0-1.0)
    pub direction_bias: i8,              // Направление пробоя: 1 (вверх), -1 (вниз), 0 (неопределенно)
    pub persistence_score: f64,          // Оценка устойчивости пробоя (0.0-1.0)
}

impl VolatilityBreakoutResult {
    pub fn empty() -> Self {
        Self {
            breakout_type: BreakoutType::None,
            volatility_ratio: 1.0,
            breakout_strength: 0.0,
            momentum_factor: 0.0,
            squeeze_duration: 0,
            breakout_probability: 0.0,
            direction_bias: 0,
            persistence_score: 0.0,
        }
    }
    
    /// Получить описание силы пробоя
    pub fn strength_description(&self) -> &'static str {
        match self.breakout_strength {
            x if x < 0.5 => "Очень слабый",
            x if x < 1.0 => "Слабый",
            x if x < 1.5 => "Умеренный",
            x if x < 2.0 => "Сильный",
            x if x < 2.5 => "Очень сильный",
            _ => "Экстремальный",
        }
    }
    
    /// Получить описание направления
    pub fn direction_description(&self) -> &'static str {
        match self.direction_bias {
            1 => "Восходящий пробой",
            -1 => "Нисходящий пробой",
            _ => "Неопределенное направление",
        }
    }
    
    /// Получить рекомендацию по торговле
    pub fn trading_recommendation(&self) -> &'static str {
        match (self.breakout_type, self.persistence_score) {
            (BreakoutType::Extreme, score) if score > 0.7 => "Агрессивная торговля на пробое",
            (BreakoutType::Strong, score) if score > 0.6 => "Торговля на пробое",
            (BreakoutType::Mild, score) if score > 0.5 => "Осторожная торговля",
            (BreakoutType::Squeeze, _) => "Подготовка к пробою",
            _ => "Ожидание",
        }
    }
}

/// Volatility Breakout Detector индикатор
#[derive(Clone)]
pub struct VolatilityBreakoutDetector {
    // Переиспользуем существующие компоненты
    atr: Atr,                            // ATR для основной волатильности
    atr_short: Atr,                      // Короткий ATR для быстрых изменений
    volatility_ma: MovingAverageProvider,        // MA для сглаживания волатильности
    momentum_ma: MovingAverageProvider,          // MA для momentum анализа
    squeeze_ma: MovingAverageProvider,           // MA для детекции сжатия
    
    // Буферы для анализа
    prices: ArrayVec<f64, 64>,
    highs: ArrayVec<f64, 32>,
    lows: ArrayVec<f64, 32>,
    ranges: ArrayVec<f64, 32>,           // True Range значения
    volatility_ratios: ArrayVec<f64, 32>, // История отношений волатильности
    breakout_history: ArrayVec<BreakoutType, 16>, // История пробоев
    
    // Параметры детекции
    mild_threshold: f64,                 // Порог умеренного пробоя
    strong_threshold: f64,               // Порог сильного пробоя
    extreme_threshold: f64,              // Порог экстремального пробоя
    squeeze_threshold: f64,              // Порог сжатия
    
    // Состояние сжатия
    squeeze_start: Option<usize>,        // Начало текущего сжатия
    last_breakout_bar: Option<usize>,    // Последний бар с пробоем
    
    // Результат
    current_result: VolatilityBreakoutResult,
    
    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl VolatilityBreakoutDetector {
    /// Создать новый Volatility Breakout Detector с параметрами по умолчанию
    pub fn new() -> Self {
        Self::with_parameters(1.2, 1.8, 2.5, 0.7)
    }
    
    /// Создать с настраиваемыми параметрами
    pub fn with_parameters(
        mild_threshold: f64,
        strong_threshold: f64,
        extreme_threshold: f64,
        squeeze_threshold: f64
    ) -> Self {
        assert!(mild_threshold > 1.0, "Mild threshold must be > 1.0");
        assert!(strong_threshold > mild_threshold, "Strong threshold must be > mild threshold");
        assert!(extreme_threshold > strong_threshold, "Extreme threshold must be > strong threshold");
        assert!(squeeze_threshold > 0.0 && squeeze_threshold < 1.0, "Squeeze threshold must be 0-1");
        
        Self {
            // Переиспользуем существующие компоненты
            atr: Atr::new_wilder(14),
            atr_short: Atr::new_wilder(5),
            volatility_ma: MovingAverageProvider::new(MovingAverageType::EMA, 20),
            momentum_ma: MovingAverageProvider::new(MovingAverageType::EMA, 8),
            squeeze_ma: MovingAverageProvider::new(MovingAverageType::SMA, 10),
            
            prices: ArrayVec::new(),
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            ranges: ArrayVec::new(),
            volatility_ratios: ArrayVec::new(),
            breakout_history: ArrayVec::new(),
            
            mild_threshold,
            strong_threshold,
            extreme_threshold,
            squeeze_threshold,
            
            squeeze_start: None,
            last_breakout_bar: None,
            
            current_result: VolatilityBreakoutResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> VolatilityBreakoutResult {
        // Добавляем данные в буферы
        if self.prices.len() >= 64 {
            self.prices.remove(0);
        }
        self.prices.push(close);
        
        if self.highs.len() >= 32 {
            self.highs.remove(0);
        }
        self.highs.push(high);
        
        if self.lows.len() >= 32 {
            self.lows.remove(0);
        }
        self.lows.push(low);
        
        // 1. Обновляем ATR индикаторы (переиспользуем существующие компоненты)
        let atr_long = self.atr.update_bar(open, high, low, close, volume);
        let atr_short = self.atr_short.update_bar(open, high, low, close, volume);
        
        // Сохраняем True Range в буфер для истории (используем централизованный ATR)
        if self.ranges.len() >= 32 {
            self.ranges.remove(0);
        }
        self.ranges.push(atr_long);
        
        // 2. Анализируем волатильность
        let volatility_analysis = self.analyze_volatility(atr_long, atr_short);
        
        // 3. Определяем тип пробоя
        let breakout_type = self.determine_breakout_type(volatility_analysis.0);
        
        // 4. Анализируем momentum и направление
        self.analyze_momentum_and_direction(close);
        
        // 5. Рассчитываем вероятность пробоя
        self.calculate_breakout_probability(volatility_analysis.0, breakout_type);
        
        // 6. Анализируем устойчивость пробоя
        self.analyze_persistence(breakout_type);
        
        // 7. Обновляем состояние сжатия
        self.update_squeeze_state(volatility_analysis.0, breakout_type);
        
        // Обновляем результат
        self.current_result.breakout_type = breakout_type;
        self.current_result.volatility_ratio = volatility_analysis.0;
        self.current_result.breakout_strength = volatility_analysis.1;
        
        // Готов после накопления достаточных данных
        if self.atr.is_ready() && self.atr_short.is_ready() && self.ranges.len() >= 10 {
            self.is_ready = true;
        }
        
        self.update_count += 1;
        self.current_result
    }
    

    
    /// Анализировать волатильность
    fn analyze_volatility(&mut self, atr_long: f64, atr_short: f64) -> (f64, f64) {
        // Сглаживаем долгосрочную волатильность
        let smoothed_volatility = self.volatility_ma.update_bar(0.0, 0.0, 0.0, atr_long, 0.0);
        
        // Отношение краткосрочной к долгосрочной волатильности
        let volatility_ratio = if smoothed_volatility > 0.0 {
            atr_short / smoothed_volatility
        } else {
            1.0
        };
        
        // Сохраняем отношение
        if self.volatility_ratios.len() >= 32 {
            self.volatility_ratios.remove(0);
        }
        self.volatility_ratios.push(volatility_ratio);
        
        // Сила пробоя (логарифмическое масштабирование)
        let breakout_strength = if volatility_ratio > 1.0 {
            (volatility_ratio - 1.0) * 2.0
        } else {
            0.0
        };
        
        (volatility_ratio, breakout_strength)
    }
    
    /// Определить тип пробоя
    fn determine_breakout_type(&self, volatility_ratio: f64) -> BreakoutType {
        if volatility_ratio >= self.extreme_threshold {
            BreakoutType::Extreme
        } else if volatility_ratio >= self.strong_threshold {
            BreakoutType::Strong
        } else if volatility_ratio >= self.mild_threshold {
            BreakoutType::Mild
        } else if volatility_ratio <= self.squeeze_threshold {
            BreakoutType::Squeeze
        } else {
            BreakoutType::None
        }
    }
    
    /// Анализировать momentum и направление
    fn analyze_momentum_and_direction(&mut self, current_price: f64) {
        if self.prices.len() < 5 {
            return;
        }
        
        let len = self.prices.len();
        let momentum = current_price - self.prices[len - 5];
        
        // Сглаживаем momentum
        let smoothed_momentum = self.momentum_ma.update_bar(0.0, 0.0, 0.0, momentum, 0.0);
        
        // Нормализуем momentum
        let momentum_factor = if current_price > 0.0 {
            (smoothed_momentum / current_price).clamp(-1.0, 1.0)
        } else {
            0.0
        };
        
        self.current_result.momentum_factor = momentum_factor;
        
        // Определяем направление пробоя
        self.current_result.direction_bias = if momentum_factor > 0.1 {
            1  // Восходящий пробой
        } else if momentum_factor < -0.1 {
            -1 // Нисходящий пробой
        } else {
            0  // Неопределенное направление
        };
    }
    
    /// Рассчитать вероятность пробоя
    fn calculate_breakout_probability(&mut self, _volatility_ratio: f64, breakout_type: BreakoutType) {
        // Базовая вероятность на основе текущего отношения волатильности
        let base_probability = match breakout_type {
            BreakoutType::Extreme => 0.9,
            BreakoutType::Strong => 0.7,
            BreakoutType::Mild => 0.4,
            BreakoutType::Squeeze => 0.8, // Высокая вероятность после сжатия
            BreakoutType::None => 0.1,
        };
        
        // Корректируем на основе истории волатильности
        let volatility_trend = if self.volatility_ratios.len() >= 5 {
            let recent = &self.volatility_ratios[self.volatility_ratios.len() - 5..];
            let trend: f64 = recent.windows(2)
                .map(|w| if w[1] > w[0] { 1.0 } else { -1.0 })
                .sum();
            trend / 4.0 // Нормализуем
        } else {
            0.0
        };
        
        // Корректируем на основе продолжительности сжатия
        let squeeze_factor = if let Some(start) = self.squeeze_start {
            let duration = self.update_count - start;
            (duration as f64 / 20.0).min(1.0) // Максимум 1.0 после 20 баров сжатия
        } else {
            0.0
        };
        
        // Итоговая вероятность
        let probability = (base_probability + 
                          volatility_trend.abs() * 0.2 + 
                          squeeze_factor * 0.3).min(1.0);
        
        self.current_result.breakout_probability = probability;
    }
    
    /// Анализировать устойчивость пробоя
    fn analyze_persistence(&mut self, breakout_type: BreakoutType) {
        // Сохраняем историю пробоев
        if self.breakout_history.len() >= 16 {
            self.breakout_history.remove(0);
        }
        self.breakout_history.push(breakout_type);
        
        if self.breakout_history.len() < 3 {
            self.current_result.persistence_score = 0.5;
            return;
        }
        
        // Анализируем последние пробои
        let recent_breakouts = &self.breakout_history[self.breakout_history.len() - 3..];
        
        // Подсчитываем силу последних пробоев
        let strength_sum: i8 = recent_breakouts.iter()
            .map(|bt| bt.as_number().max(0))
            .sum();
        
        // Проверяем консистентность направления
        let consistency = if self.volatility_ratios.len() >= 3 {
            let recent_ratios = &self.volatility_ratios[self.volatility_ratios.len() - 3..];
            let increasing = recent_ratios.windows(2).all(|w| w[1] >= w[0]);
            let decreasing = recent_ratios.windows(2).all(|w| w[1] <= w[0]);
            
            if increasing || decreasing { 1.0 } else { 0.5 }
        } else {
            0.5
        };
        
        // Рассчитываем оценку устойчивости
        let persistence_score = ((strength_sum as f64 / 9.0) * 0.6 + consistency * 0.4).min(1.0);
        
        self.current_result.persistence_score = persistence_score;
    }
    
    /// Обновить состояние сжатия
    fn update_squeeze_state(&mut self, _volatility_ratio: f64, breakout_type: BreakoutType) {
        match breakout_type {
            BreakoutType::Squeeze => {
                // Начинаем или продолжаем сжатие
                if self.squeeze_start.is_none() {
                    self.squeeze_start = Some(self.update_count);
                }
            }
            BreakoutType::Mild | BreakoutType::Strong | BreakoutType::Extreme => {
                // Пробой - заканчиваем сжатие
                if self.squeeze_start.is_some() {
                    self.squeeze_start = None;
                }
                self.last_breakout_bar = Some(self.update_count);
            }
            _ => {}
        }
        
        // Обновляем продолжительность сжатия
        self.current_result.squeeze_duration = if let Some(start) = self.squeeze_start {
            self.update_count - start
        } else {
            0
        };
    }
    
    /// Получить текущий тип пробоя как BreakoutType (legacy)
    pub fn breakout_type(&self) -> BreakoutType {
        self.current_result.breakout_type
    }

    /// Получить значение в виде IndicatorValue (Signal)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.current_result.breakout_type.as_number())
    }
    
    /// Получить полный результат
    pub fn result(&self) -> VolatilityBreakoutResult {
        self.current_result
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.atr.reset();
        self.atr_short.reset();
        self.volatility_ma.reset();
        self.momentum_ma.reset();
        self.squeeze_ma.reset();
        
        self.prices.clear();
        self.highs.clear();
        self.lows.clear();
        self.ranges.clear();
        self.volatility_ratios.clear();
        self.breakout_history.clear();
        
        self.squeeze_start = None;
        self.last_breakout_bar = None;
        
        self.current_result = VolatilityBreakoutResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }
    
    /// Получить период (условный)
    pub fn period(&self) -> usize {
        self.atr.period()
    }
    
    /// Генерировать торговый сигнал
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let result = self.current_result;
        
        // Сигналы только при высокой вероятности и устойчивости
        if result.breakout_probability < 0.6 || result.persistence_score < 0.5 {
            return 0;
        }
        
        match result.breakout_type {
            BreakoutType::Strong | BreakoutType::Extreme => {
                result.direction_bias
            }
            BreakoutType::Mild => {
                if result.persistence_score > 0.7 {
                    result.direction_bias
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
    
    /// Генерировать сигнал готовности к пробою
    pub fn pre_breakout_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let result = self.current_result;
        
        // Сигнал перед пробоем при длительном сжатии
        if matches!(result.breakout_type, BreakoutType::Squeeze) && 
           result.squeeze_duration > 10 &&
           result.breakout_probability > 0.7 {
            return result.direction_bias;
        }
        
        0
    }
    
    /// Генерировать сигнал силы пробоя
    pub fn strength_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let strength = self.current_result.breakout_strength;
        
        if strength > 2.0 {
            return 3; // Экстремальная сила
        } else if strength > 1.5 {
            return 2; // Высокая сила
        } else if strength > 1.0 {
            return 1; // Умеренная сила
        }
        
        0
    }
    
    /// Получить информацию о текущем состоянии
    pub fn info(&self) -> String {
        let result = self.current_result;
        let signal = match self.trading_signal() {
            1 => "Покупка на пробое",
            -1 => "Продажа на пробое",
            _ => "Нет сигнала",
        };
        
        format!(
            "Volatility Breakout: {}, Сила: {} ({:.1}), Вероятность: {:.1}%, {}, Сжатие: {} баров, Сигнал: {}",
            result.breakout_type.as_str(),
            result.strength_description(),
            result.breakout_strength,
            result.breakout_probability * 100.0,
            result.direction_description(),
            result.squeeze_duration,
            signal
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("breakout_type".to_string(), self.current_result.breakout_type.as_number() as f64);
        values.insert("volatility_ratio".to_string(), self.current_result.volatility_ratio);
        values.insert("breakout_strength".to_string(), self.current_result.breakout_strength);
        values.insert("momentum_factor".to_string(), self.current_result.momentum_factor);
        values.insert("squeeze_duration".to_string(), self.current_result.squeeze_duration as f64);
        values.insert("breakout_probability".to_string(), self.current_result.breakout_probability);
        values.insert("direction_bias".to_string(), self.current_result.direction_bias as f64);
        values.insert("persistence_score".to_string(), self.current_result.persistence_score);
        values
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    
    /// Получить параметры
    pub fn parameters(&self) -> (f64, f64, f64, f64) {
        (self.mild_threshold, self.strong_threshold, self.extreme_threshold, self.squeeze_threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volatility_breakout_detector_creation() {
        let vbd = VolatilityBreakoutDetector::new();
        assert!(!vbd.is_ready());
        assert_eq!(vbd.breakout_type(), BreakoutType::None);
    }
    
    #[test]
    fn test_volatility_breakout_detector_with_parameters() {
        let vbd = VolatilityBreakoutDetector::with_parameters(1.3, 2.0, 3.0, 0.6);
        assert_eq!(vbd.parameters(), (1.3, 2.0, 3.0, 0.6));
    }
    
    #[test]
    fn test_breakout_detection() {
        let mut vbd = VolatilityBreakoutDetector::new();
        
        // Период низкой волатильности (сжатие)
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.01);
            let _result = vbd.update_bar(price, price + 0.01, price - 0.01, price, 1000.0);
        }
        
        // Резкое увеличение волатильности
        for i in 15..25 {
            let base_price = 100.0;
            let high_vol = 5.0;
            let price = base_price + (i as f64 * 0.5);
            let result = vbd.update_bar(
                price, 
                price + high_vol, 
                price - high_vol, 
                price, 
                1000.0
            );
            
            if i > 20 && vbd.is_ready() {
                // Должен обнаружить пробой волатильности
                assert!(result.volatility_ratio > 1.0);
                assert!(result.breakout_strength >= 0.0);
                assert!(result.breakout_probability >= 0.0 && result.breakout_probability <= 1.0);
            }
        }
    }
    
    #[test]
    fn test_squeeze_detection() {
        let mut vbd = VolatilityBreakoutDetector::new();
        
        // Длительный период низкой волатильности
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.001); // Очень маленькие изменения
            let result = vbd.update_bar(price, price + 0.001, price - 0.001, price, 1000.0);
            
            if i > 20 && vbd.is_ready() {
                // Должен обнаружить сжатие
                assert!(result.volatility_ratio <= 1.0);
                if matches!(result.breakout_type, BreakoutType::Squeeze) {
                    assert!(result.squeeze_duration > 0);
                }
            }
        }
    }
    
    #[test]
    fn test_direction_bias() {
        let mut vbd = VolatilityBreakoutDetector::new();
        
        // Восходящий тренд с увеличивающейся волатильностью
        for i in 0..25 {
            let price = 100.0 + i as f64 * 0.5; // Четкий восходящий тренд
            let volatility = 1.0 + (i as f64 / 10.0); // Увеличивающаяся волатильность
            
            let result = vbd.update_bar(
                price, 
                price + volatility, 
                price - volatility, 
                price, 
                1000.0
            );
            
            if i > 20 && vbd.is_ready() {
                // При восходящем тренде должно быть положительное смещение
                if result.breakout_strength > 0.5 {
                    assert!(result.direction_bias >= 0);
                }
            }
        }
    }
    
    #[test]
    fn test_trading_signals() {
        let mut vbd = VolatilityBreakoutDetector::new();
        
        // Создаем условия для сильного пробоя
        for i in 0..20 {
            let base_price = 100.0;
            let volatility = if i > 15 { 3.0 } else { 0.5 };
            let price = base_price + i as f64 * 0.2;

            let _result = vbd.update_bar(
                price,
                price + volatility,
                price - volatility,
                price,
                1000.0
            );
            
            if i > 18 && vbd.is_ready() {
                let signal = vbd.trading_signal();
                let pre_signal = vbd.pre_breakout_signal();
                let strength_signal = vbd.strength_signal();
                
                assert!(signal >= -1 && signal <= 1);
                assert!(pre_signal >= -1 && pre_signal <= 1);
                assert!(strength_signal >= 0 && strength_signal <= 3);
            }
        }
    }
    
    #[test]
    fn test_persistence_analysis() {
        let mut vbd = VolatilityBreakoutDetector::new();
        
        // Последовательные пробои для тестирования устойчивости
        for i in 0..30 {
            let base_price = 100.0;
            let volatility = if i % 5 == 0 { 2.0 } else { 0.8 }; // Периодические пробои
            let price = base_price + i as f64 * 0.1;
            
            let result = vbd.update_bar(
                price, 
                price + volatility, 
                price - volatility, 
                price, 
                1000.0
            );
            
            if i > 25 && vbd.is_ready() {
                assert!(result.persistence_score >= 0.0 && result.persistence_score <= 1.0);
            }
        }
    }
} 






















