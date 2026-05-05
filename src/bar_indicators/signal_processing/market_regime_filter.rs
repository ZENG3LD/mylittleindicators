//! Market Regime Filter - фильтр рыночных режимов
//!
//! Определяет текущий режим рынка: тренд, флэт, высокая волатильность, спокойствие.
//! Использует комбинацию индикаторов для классификации рыночных условий.
//!
//! Переиспользует существующие компоненты: MovingAverageProvider, ATR, momentum индикаторы

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;
use arrayvec::ArrayVec;
use std::collections::HashMap;

/// Типы рыночных режимов
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketRegime {
    UpTrend,        // Восходящий тренд
    DownTrend,      // Нисходящий тренд
    SidewaysFlat,   // Боковой флэт (низкая волатильность)
    ChoppyFlat,     // Рваный флэт (средняя волатильность)
    HighVolatility, // Высокая волатильность
    Quiet,          // Спокойный рынок
    Transition,     // Переходное состояние
}

impl MarketRegime {
    pub fn as_str(&self) -> &'static str {
        match self {
            MarketRegime::UpTrend => "Восходящий тренд",
            MarketRegime::DownTrend => "Нисходящий тренд", 
            MarketRegime::SidewaysFlat => "Боковой флэт",
            MarketRegime::ChoppyFlat => "Рваный флэт",
            MarketRegime::HighVolatility => "Высокая волатильность",
            MarketRegime::Quiet => "Спокойный рынок",
            MarketRegime::Transition => "Переходное состояние",
        }
    }
    
    pub fn as_number(&self) -> i8 {
        match self {
            MarketRegime::UpTrend => 2,
            MarketRegime::DownTrend => -2,
            MarketRegime::SidewaysFlat => 0,
            MarketRegime::ChoppyFlat => 1,
            MarketRegime::HighVolatility => 3,
            MarketRegime::Quiet => -1,
            MarketRegime::Transition => 99,
        }
    }
}

/// Результат Market Regime Filter
#[derive(Debug, Clone, Copy)]
pub struct MarketRegimeResult {
    pub regime: MarketRegime,        // Текущий режим рынка
    pub confidence: f64,             // Уверенность в определении (0.0-1.0)
    pub trend_strength: f64,         // Сила тренда (0.0-1.0)
    pub volatility_level: f64,       // Уровень волатильности (0.0-1.0+)
    pub momentum_score: f64,         // Оценка momentum (-1.0 до 1.0)
    pub stability_index: f64,        // Индекс стабильности (0.0-1.0)
    pub regime_duration: usize,      // Продолжительность текущего режима в барах
    pub next_regime_probability: f64, // Вероятность смены режима (0.0-1.0)
}

impl MarketRegimeResult {
    pub fn empty() -> Self {
        Self {
            regime: MarketRegime::Transition,
            confidence: 0.0,
            trend_strength: 0.0,
            volatility_level: 0.5,
            momentum_score: 0.0,
            stability_index: 0.5,
            regime_duration: 0,
            next_regime_probability: 0.5,
        }
    }
}

/// Market Regime Filter индикатор
#[derive(Clone)]
pub struct MarketRegimeFilter {
    // Переиспользуем существующие компоненты
    fast_ma: MovingAverageProvider,          // Быстрая MA для тренда
    slow_ma: MovingAverageProvider,          // Медленная MA для тренда
    atr: Atr,                        // ATR для волатильности
    volatility_ma: MovingAverageProvider,    // MA для сглаживания волатильности
    momentum_ma: MovingAverageProvider,      // MA для momentum
    stability_ma: MovingAverageProvider,     // MA для стабильности
    
    // Буферы для анализа
    prices: ArrayVec<f64, 64>,
    highs: ArrayVec<f64, 32>,
    lows: ArrayVec<f64, 32>,
    ranges: ArrayVec<f64, 32>,
    regime_history: ArrayVec<MarketRegime, 32>,
    
    // Параметры анализа
    trend_threshold: f64,            // Порог для определения тренда
    volatility_threshold: f64,       // Порог высокой волатильности
    quiet_threshold: f64,            // Порог спокойного рынка
    
    // Текущее состояние
    current_regime: MarketRegime,
    regime_start_time: usize,
    
    // Результат
    current_result: MarketRegimeResult,
    
    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl MarketRegimeFilter {
    /// Создать новый Market Regime Filter с параметрами по умолчанию
    pub fn new() -> Self {
        Self::with_parameters(10, 30, 0.02, 1.5, 0.3)
    }
    
    /// Создать с настраиваемыми параметрами
    pub fn with_parameters(
        fast_period: usize,
        slow_period: usize,
        trend_threshold: f64,
        volatility_threshold: f64,
        quiet_threshold: f64
    ) -> Self {
        assert!(fast_period > 0 && slow_period > fast_period, "Invalid MA periods");
        assert!(trend_threshold > 0.0, "Trend threshold must be positive");
        assert!(volatility_threshold > 1.0, "Volatility threshold must be > 1.0");
        assert!(quiet_threshold > 0.0 && quiet_threshold < 1.0, "Invalid quiet threshold");
        
        Self {
            // Переиспользуем MovingAverage и ATR
            fast_ma: MovingAverageProvider::new(MovingAverageType::EMA, fast_period),
            slow_ma: MovingAverageProvider::new(MovingAverageType::EMA, slow_period),
            atr: Atr::new_wilder(14),
            volatility_ma: MovingAverageProvider::new(MovingAverageType::SMA, 20),
            momentum_ma: MovingAverageProvider::new(MovingAverageType::EMA, 8),
            stability_ma: MovingAverageProvider::new(MovingAverageType::SMA, 15),
            
            prices: ArrayVec::new(),
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            ranges: ArrayVec::new(),
            regime_history: ArrayVec::new(),
            
            trend_threshold,
            volatility_threshold,
            quiet_threshold,
            
            current_regime: MarketRegime::Transition,
            regime_start_time: 0,
            
            current_result: MarketRegimeResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> MarketRegimeResult {
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
        
        let range = high - low;
        if self.ranges.len() >= 32 {
            self.ranges.remove(0);
        }
        self.ranges.push(range);
        
        // 1. Обновляем все индикаторы (переиспользуем существующие компоненты)
        let fast_ma_value = self.fast_ma.update_bar(open, high, low, close, volume);
        let slow_ma_value = self.slow_ma.update_bar(open, high, low, close, volume);
        let atr_value = self.atr.update_bar(open, high, low, close, volume);
        
        // 2. Анализируем тренд
        let trend_analysis = self.analyze_trend(close, fast_ma_value, slow_ma_value);
        
        // 3. Анализируем волатильность
        let volatility_analysis = self.analyze_volatility(atr_value);
        
        // 4. Анализируем momentum
        let momentum_analysis = self.analyze_momentum(close);
        
        // 5. Анализируем стабильность
        let stability_analysis = self.analyze_stability();
        
        // 6. Определяем режим рынка
        let new_regime = self.determine_regime(
            trend_analysis,
            volatility_analysis,
            momentum_analysis,
            stability_analysis
        );
        
        // 7. Обновляем состояние режима
        self.update_regime_state(new_regime);
        
        // 8. Рассчитываем уверенность и дополнительные метрики
        self.calculate_confidence_and_metrics(
            trend_analysis,
            volatility_analysis,
            momentum_analysis,
            stability_analysis
        );
        
        // Готов после накопления достаточных данных
        if self.fast_ma.is_ready() && self.slow_ma.is_ready() && self.atr.is_ready() {
            self.is_ready = true;
        }
        
        self.update_count += 1;
        self.current_result
    }
    
    /// Анализировать тренд
    fn analyze_trend(&self, price: f64, fast_ma: f64, slow_ma: f64) -> (f64, i8) {
        // Сила тренда на основе расстояния между MA
        let ma_distance = if slow_ma != 0.0 {
            (fast_ma - slow_ma).abs() / slow_ma
        } else {
            0.0
        };
        
        let trend_strength = (ma_distance / self.trend_threshold).min(1.0);
        
        // Направление тренда
        let trend_direction = if fast_ma > slow_ma && price > fast_ma {
            1  // Восходящий
        } else if fast_ma < slow_ma && price < fast_ma {
            -1 // Нисходящий
        } else {
            0  // Боковой
        };
        
        (trend_strength, trend_direction)
    }
    
    /// Анализировать волатильность
    fn analyze_volatility(&mut self, atr_value: f64) -> f64 {
        // Сглаживаем волатильность
        let smoothed_volatility = self.volatility_ma.update_bar(0.0, 0.0, 0.0, atr_value, 0.0);
        
        // Нормализуем волатильность
        if smoothed_volatility > 0.0 {
            atr_value / smoothed_volatility
        } else {
            1.0
        }
    }
    
    /// Анализировать momentum
    fn analyze_momentum(&mut self, price: f64) -> f64 {
        if self.prices.len() < 5 {
            return 0.0;
        }
        
        let len = self.prices.len();
        let momentum = price - self.prices[len - 5];
        
        // Сглаживаем momentum
        let smoothed_momentum = self.momentum_ma.update_bar(0.0, 0.0, 0.0, momentum, 0.0);
        
        // Нормализуем momentum
        if price != 0.0 {
            (smoothed_momentum / price).clamp(-1.0, 1.0)
        } else {
            0.0
        }
    }
    
    /// Анализировать стабильность
    fn analyze_stability(&mut self) -> f64 {
        if self.ranges.len() < 10 {
            return 0.5;
        }
        
        // Коэффициент вариации диапазонов
        let recent_ranges = &self.ranges[self.ranges.len() - 10..];
        let mean_range: f64 = recent_ranges.iter().sum::<f64>() / recent_ranges.len() as f64;
        
        if mean_range > 0.0 {
            let variance: f64 = recent_ranges.iter()
                .map(|&r| (r - mean_range).powi(2))
                .sum::<f64>() / recent_ranges.len() as f64;
            
            let cv = variance.sqrt() / mean_range;
            let stability = (1.0 - cv.min(1.0)).max(0.0);
            
            // Сглаживаем стабильность
            self.stability_ma.update_bar(0.0, 0.0, 0.0, stability, 0.0)
        } else {
            0.5
        }
    }
    
    /// Определить режим рынка
    fn determine_regime(
        &self,
        trend: (f64, i8),
        volatility: f64,
        _momentum: f64,
        stability: f64
    ) -> MarketRegime {
        let (trend_strength, trend_direction) = trend;
        
        // Логика определения режима
        if volatility > self.volatility_threshold {
            MarketRegime::HighVolatility
        } else if volatility < self.quiet_threshold {
            MarketRegime::Quiet
        } else if trend_strength > 0.6 && stability > 0.5 {
            match trend_direction {
                1 => MarketRegime::UpTrend,
                -1 => MarketRegime::DownTrend,
                _ => MarketRegime::Transition,
            }
        } else if trend_strength < 0.3 {
            if stability > 0.6 {
                MarketRegime::SidewaysFlat
            } else {
                MarketRegime::ChoppyFlat
            }
        } else {
            MarketRegime::Transition
        }
    }
    
    /// Обновить состояние режима
    fn update_regime_state(&mut self, new_regime: MarketRegime) {
        if new_regime != self.current_regime {
            // Сохраняем предыдущий режим в истории
            if self.regime_history.len() >= 32 {
                self.regime_history.remove(0);
            }
            self.regime_history.push(self.current_regime);
            
            // Обновляем текущий режим
            self.current_regime = new_regime;
            self.regime_start_time = self.update_count;
        }
        
        // Обновляем результат
        self.current_result.regime = self.current_regime;
        self.current_result.regime_duration = self.update_count - self.regime_start_time;
    }
    
    /// Рассчитать уверенность и дополнительные метрики
    fn calculate_confidence_and_metrics(
        &mut self,
        trend: (f64, i8),
        volatility: f64,
        momentum: f64,
        stability: f64
    ) {
        let (trend_strength, _trend_direction) = trend;
        
        // Уверенность на основе согласованности метрик
        let volatility_confidence = match self.current_regime {
            MarketRegime::HighVolatility => (volatility - self.volatility_threshold).min(1.0),
            MarketRegime::Quiet => (self.quiet_threshold - volatility).max(0.0) / self.quiet_threshold,
            _ => 1.0 - (volatility - 1.0).abs().min(1.0),
        };
        
        let trend_confidence = match self.current_regime {
            MarketRegime::UpTrend | MarketRegime::DownTrend => trend_strength,
            MarketRegime::SidewaysFlat | MarketRegime::ChoppyFlat => 1.0 - trend_strength,
            _ => 0.5,
        };
        
        let stability_confidence = match self.current_regime {
            MarketRegime::SidewaysFlat | MarketRegime::Quiet => stability,
            MarketRegime::ChoppyFlat | MarketRegime::HighVolatility => 1.0 - stability,
            _ => 0.7,
        };
        
        // Общая уверенность
        self.current_result.confidence = (volatility_confidence + trend_confidence + stability_confidence) / 3.0;
        
        // Остальные метрики
        self.current_result.trend_strength = trend_strength;
        self.current_result.volatility_level = volatility;
        self.current_result.momentum_score = momentum;
        self.current_result.stability_index = stability;
        
        // Вероятность смены режима
        self.calculate_regime_change_probability();
    }
    
    /// Рассчитать вероятность смены режима
    fn calculate_regime_change_probability(&mut self) {
        let base_probability = match self.current_result.regime_duration {
            0..=5 => 0.1,   // Новый режим - низкая вероятность смены
            6..=15 => 0.3,  // Установившийся режим
            16..=30 => 0.5, // Зрелый режим
            _ => 0.7,       // Старый режим - высокая вероятность смены
        };
        
        // Корректируем на основе уверенности
        let confidence_factor = 1.0 - self.current_result.confidence;
        self.current_result.next_regime_probability = (base_probability + confidence_factor * 0.3).min(1.0);
    }
    
    /// Получить текущий режим как MarketRegime (legacy)
    pub fn regime(&self) -> MarketRegime {
        self.current_result.regime
    }

    /// Получить значение в виде IndicatorValue (Signal)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.current_result.regime.as_number())
    }
    
    /// Получить полный результат
    pub fn result(&self) -> MarketRegimeResult {
        self.current_result
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.fast_ma.reset();
        self.slow_ma.reset();
        self.atr.reset();
        self.volatility_ma.reset();
        self.momentum_ma.reset();
        self.stability_ma.reset();
        
        self.prices.clear();
        self.highs.clear();
        self.lows.clear();
        self.ranges.clear();
        self.regime_history.clear();
        
        self.current_regime = MarketRegime::Transition;
        self.regime_start_time = 0;
        
        self.current_result = MarketRegimeResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }
    
    /// Получить период (условный)
    pub fn period(&self) -> usize {
        self.slow_ma.period()
    }
    

    
    /// Получить информацию о текущем состоянии
    pub fn info(&self) -> String {
        let result = self.current_result;
        
        format!(
            "Режим: {}, Уверенность: {:.1}%, Продолжительность: {} баров, Тренд: {:.1}%, Волатильность: {:.1}%",
            result.regime.as_str(),
            result.confidence * 100.0,
            result.regime_duration,
            result.trend_strength * 100.0,
            result.volatility_level * 100.0
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> HashMap<String, f64> {
        let mut values = HashMap::new();
        values.insert("regime_number".to_string(), self.current_result.regime.as_number() as f64);
        values.insert("confidence".to_string(), self.current_result.confidence);
        values.insert("trend_strength".to_string(), self.current_result.trend_strength);
        values.insert("volatility_level".to_string(), self.current_result.volatility_level);
        values.insert("momentum_score".to_string(), self.current_result.momentum_score);
        values.insert("stability_index".to_string(), self.current_result.stability_index);
        values.insert("regime_duration".to_string(), self.current_result.regime_duration as f64);
        values.insert("change_probability".to_string(), self.current_result.next_regime_probability);
        values
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    
    /// Получить историю режимов
    pub fn regime_history(&self) -> Vec<MarketRegime> {
        self.regime_history.iter().copied().collect()
    }
    
    /// Получить параметры
    pub fn parameters(&self) -> (usize, usize, f64, f64, f64) {
        (
            self.fast_ma.period(),
            self.slow_ma.period(),
            self.trend_threshold,
            self.volatility_threshold,
            self.quiet_threshold
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_regime_filter_creation() {
        let mrf = MarketRegimeFilter::new();
        assert!(!mrf.is_ready());
        assert_eq!(mrf.regime(), MarketRegime::Transition);
    }
    
    #[test]
    fn test_market_regime_filter_with_parameters() {
        let mrf = MarketRegimeFilter::with_parameters(5, 20, 0.01, 2.0, 0.2);
        assert_eq!(mrf.parameters(), (5, 20, 0.01, 2.0, 0.2));
    }
    
    #[test]
    fn test_regime_detection() {
        let mut mrf = MarketRegimeFilter::new();
        
        // Тестируем восходящий тренд
        for i in 0..30 {
            let price = 100.0 + i as f64 * 0.5; // Четкий восходящий тренд
            let result = mrf.update_bar(price, price + 0.2, price - 0.2, price, 1000.0);
            
            if i > 25 && mrf.is_ready() {
                // Должен определить восходящий тренд
                assert!(matches!(result.regime, MarketRegime::UpTrend | MarketRegime::Transition));
                assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
                assert!(result.trend_strength >= 0.0 && result.trend_strength <= 1.0);
            }
        }
    }
    
    #[test]
    fn test_volatility_detection() {
        let mut mrf = MarketRegimeFilter::new();
        
        // Тестируем высокую волатильность
        for i in 0..25 {
            let base_price = 100.0;
            let volatility = 5.0 * (i as f64 * 0.3).sin(); // Высокая волатильность
            let price = base_price + volatility;
            
            let result = mrf.update_bar(
                price,
                price + 3.0,
                price - 3.0,
                price,
                1000.0
            );
            
            if i > 20 && mrf.is_ready() {
                assert!(result.volatility_level > 0.0);
                assert!(result.regime_duration <= i);
            }
        }
    }
    
    #[test]
    fn test_regime_transitions() {
        let mut mrf = MarketRegimeFilter::new();
        let mut previous_regime = MarketRegime::Transition;
        let mut regime_changes = 0;
        
        // Тестируем различные рыночные условия
        for i in 0..50 {
            let price = match i {
                0..=15 => 100.0 + i as f64 * 0.1,     // Слабый тренд
                16..=25 => 101.5 + (i as f64 * 0.1).sin(), // Флэт
                26..=35 => 102.0 + i as f64 * 0.5,    // Сильный тренд
                _ => 120.0 + (i as f64 * 0.5).sin() * 3.0, // Волатильность
            };
            
            let result = mrf.update_bar(price, price + 0.5, price - 0.5, price, 1000.0);
            
            if mrf.is_ready() && result.regime != previous_regime {
                regime_changes += 1;
                previous_regime = result.regime;
            }
        }
        
        // Должно быть несколько смен режимов
        assert!(regime_changes > 0);
    }
    
    #[test]
    fn test_confidence_calculation() {
        let mut mrf = MarketRegimeFilter::new();
        
        // Стабильные условия должны давать высокую уверенность
        for i in 0..30 {
            let price = 100.0 + i as f64 * 0.3; // Стабильный тренд
            let result = mrf.update_bar(price, price + 0.1, price - 0.1, price, 1000.0);
            
            if i > 25 && mrf.is_ready() {
                // Уверенность должна расти со временем
                assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
                assert!(result.regime_duration > 0);
            }
        }
    }
} 






















