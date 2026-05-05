//! Dynamic Volatility Regime - Advanced volatility regime detection
//!
//! This indicator detects and classifies different volatility regimes using
//! multiple methodologies: GARCH-like analysis, regime switching models,
//! and adaptive thresholds. It provides early warning of volatility shifts.
//!
//! Переиспользует существующие компоненты MovingAverage и ATR

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::utils::math::percentile::quickselect_nth;
use arrayvec::ArrayVec;

/// Тип режима волатильности
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolatilityRegime {
    VeryLow,      // Очень низкая волатильность
    Low,          // Низкая волатильность
    Normal,       // Нормальная волатильность
    High,         // Высокая волатильность
    VeryHigh,     // Очень высокая волатильность
    Extreme,      // Экстремальная волатильность
}

impl VolatilityRegime {
    pub fn as_str(&self) -> &'static str {
        match self {
            VolatilityRegime::VeryLow => "Очень низкая",
            VolatilityRegime::Low => "Низкая",
            VolatilityRegime::Normal => "Нормальная",
            VolatilityRegime::High => "Высокая",
            VolatilityRegime::VeryHigh => "Очень высокая",
            VolatilityRegime::Extreme => "Экстремальная",
        }
    }
    
    pub fn as_f64(&self) -> f64 {
        match self {
            VolatilityRegime::VeryLow => 1.0,
            VolatilityRegime::Low => 2.0,
            VolatilityRegime::Normal => 3.0,
            VolatilityRegime::High => 4.0,
            VolatilityRegime::VeryHigh => 5.0,
            VolatilityRegime::Extreme => 6.0,
        }
    }
}

/// Результат Dynamic Volatility Regime
#[derive(Debug, Clone, Copy)]
pub struct DynamicVolatilityRegimeResult {
    pub current_regime: VolatilityRegime,    // Текущий режим
    pub regime_probability: f64,             // Вероятность текущего режима (0.0-1.0)
    pub volatility_score: f64,               // Оценка волатильности (0.0-10.0)
    pub regime_persistence: f64,             // Устойчивость режима (0.0-1.0)
    pub transition_probability: f64,         // Вероятность смены режима (0.0-1.0)
    pub volatility_trend: f64,               // Тренд волатильности (-1.0 до 1.0)
    pub adaptive_threshold_low: f64,         // Адаптивный нижний порог
    pub adaptive_threshold_high: f64,        // Адаптивный верхний порог
    pub garch_volatility: f64,               // GARCH-подобная волатильность
    pub regime_signal: i8,                   // Сигнал: 1 (рост волат.), -1 (падение), 0 (стабильно)
}

impl DynamicVolatilityRegimeResult {
    pub fn empty() -> Self {
        Self {
            current_regime: VolatilityRegime::Normal,
            regime_probability: 0.0,
            volatility_score: 3.0,
            regime_persistence: 0.0,
            transition_probability: 0.0,
            volatility_trend: 0.0,
            adaptive_threshold_low: 0.0,
            adaptive_threshold_high: 0.0,
            garch_volatility: 0.0,
            regime_signal: 0,
        }
    }
}

/// Dynamic Volatility Regime индикатор
#[derive(Clone)]
pub struct DynamicVolatilityRegime {
    // Переиспользуем существующие компоненты
    atr: Atr,                               // ATR для базовой волатильности
    volatility_ma: MovingAverageProvider,           // MA для сглаживания волатильности
    long_term_vol: MovingAverageProvider,           // Долгосрочная волатильность
    regime_smoother: MovingAverageProvider,         // Сглаживание режима
    
    // Буферы для расчетов
    returns: ArrayVec<f64, 64>,             // Логарифмические доходности
    volatilities: ArrayVec<f64, 32>,        // Значения волатильности
    regime_scores: ArrayVec<f64, 16>,       // Оценки режимов
    regime_history: ArrayVec<VolatilityRegime, 16>, // История режимов
    
    // GARCH-подобные параметры
    garch_alpha: f64,                       // Коэффициент для GARCH (0.0-1.0)
    garch_beta: f64,                        // Коэффициент для GARCH (0.0-1.0)
    garch_omega: f64,                       // Константа для GARCH
    conditional_variance: f64,              // Условная дисперсия
    
    // Адаптивные пороги
    threshold_adaptation_speed: f64,        // Скорость адаптации порогов
    
    // Результат
    current_result: DynamicVolatilityRegimeResult,
    
    // Состояние
    prev_price: Option<f64>,
    is_ready: bool,
    update_count: usize,
}

impl DynamicVolatilityRegime {
    /// Создать новый Dynamic Volatility Regime с параметрами по умолчанию
    pub fn new() -> Self {
        Self::with_parameters(0.1, 0.85, 0.01, 0.05)
    }
    
    /// Создать с настраиваемыми параметрами
    pub fn with_parameters(
        garch_alpha: f64, 
        garch_beta: f64, 
        garch_omega: f64, 
        threshold_adaptation_speed: f64
    ) -> Self {
        assert!(garch_alpha > 0.0 && garch_alpha < 1.0, "GARCH alpha must be between 0.0 and 1.0");
        assert!(garch_beta > 0.0 && garch_beta < 1.0, "GARCH beta must be between 0.0 and 1.0");
        assert!(garch_alpha + garch_beta < 1.0, "GARCH alpha + beta must be less than 1.0");
        assert!(garch_omega > 0.0, "GARCH omega must be positive");
        assert!(threshold_adaptation_speed > 0.0 && threshold_adaptation_speed <= 1.0, 
                "Threshold adaptation speed must be between 0.0 and 1.0");
        
        Self {
            // Переиспользуем компоненты
            atr: Atr::new(14, MovingAverageType::SMA),
            volatility_ma: MovingAverageProvider::new(MovingAverageType::EMA, 10),
            long_term_vol: MovingAverageProvider::new(MovingAverageType::SMA, 50),
            regime_smoother: MovingAverageProvider::new(MovingAverageType::EMA, 5),
            
            returns: ArrayVec::new(),
            volatilities: ArrayVec::new(),
            regime_scores: ArrayVec::new(),
            regime_history: ArrayVec::new(),
            
            garch_alpha,
            garch_beta,
            garch_omega,
            conditional_variance: 0.0,
            
            threshold_adaptation_speed,
            
            current_result: DynamicVolatilityRegimeResult::empty(),
            
            prev_price: None,
            is_ready: false,
            update_count: 0,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> DynamicVolatilityRegimeResult {
        // 1. Рассчитываем ATR
        let atr_value = self.atr.update_bar(open, high, low, close, volume);
        
        // 2. Рассчитываем логарифмические доходности
        if let Some(prev_price) = self.prev_price {
            let log_return = (close / prev_price).ln();
            
            if self.returns.len() >= 64 {
                self.returns.remove(0);
            }
            self.returns.push(log_return);
            
            // 3. Обновляем GARCH-подобную волатильность
            self.update_garch_volatility(log_return);
            
            // 4. Рассчитываем комплексную волатильность
            let volatility = self.calculate_composite_volatility(atr_value);
            
            // 5. Обновляем адаптивные пороги
            self.update_adaptive_thresholds(volatility);
            
            // 6. Определяем режим волатильности
            let regime = self.classify_volatility_regime(volatility);
            
            // 7. Анализируем устойчивость режима
            self.analyze_regime_persistence(regime);
            
            // 8. Рассчитываем тренд волатильности
            self.calculate_volatility_trend();
            
            // 9. Генерируем сигналы
            self.generate_regime_signals();
            
            // Обновляем результат
            self.current_result.current_regime = regime;
            self.current_result.garch_volatility = self.conditional_variance.sqrt();
            
            // Проверяем готовность
            if self.returns.len() >= 20 && self.volatilities.len() >= 10 {
                self.is_ready = true;
            }
        }
        
        self.prev_price = Some(close);
        self.update_count += 1;
        self.current_result
    }
    
    /// Обновить GARCH-подобную условную дисперсию
    fn update_garch_volatility(&mut self, log_return: f64) {
        if self.update_count == 1 {
            // Инициализация
            self.conditional_variance = log_return * log_return;
        } else {
            // GARCH(1,1) формула: σ²(t) = ω + α*ε²(t-1) + β*σ²(t-1)
            let squared_return = log_return * log_return;
            self.conditional_variance = self.garch_omega + 
                                       self.garch_alpha * squared_return + 
                                       self.garch_beta * self.conditional_variance;
        }
    }
    
    /// Рассчитать комплексную волатильность
    fn calculate_composite_volatility(&mut self, atr_value: f64) -> f64 {
        if self.returns.len() < 2 {
            return atr_value;
        }
        
        // 1. Реализованная волатильность (стандартное отклонение доходностей)
        let realized_vol = self.calculate_realized_volatility();
        
        // 2. GARCH волатильность
        let garch_vol = self.conditional_variance.sqrt();
        
        // 3. ATR-основанная волатильность
        let atr_vol = atr_value;
        
        // Комбинируем разные меры волатильности
        let composite_vol = realized_vol * 0.4 + garch_vol * 0.4 + atr_vol * 0.2;
        
        // Сглаживаем результат
        let smoothed_vol = self.volatility_ma.update_bar(0.0, 0.0, 0.0, composite_vol, 0.0);
        
        // Сохраняем в буфер
        if self.volatilities.len() >= 32 {
            self.volatilities.remove(0);
        }
        self.volatilities.push(smoothed_vol);
        
        smoothed_vol
    }
    
    /// Рассчитать реализованную волатильность
    fn calculate_realized_volatility(&self) -> f64 {
        if self.returns.len() < 10 {
            return 0.0;
        }
        
        let window_size = 20.min(self.returns.len());
        let start = self.returns.len() - window_size;
        
        // Среднее значение
        let mean: f64 = self.returns[start..].iter().sum::<f64>() / window_size as f64;
        
        // Дисперсия
        let variance: f64 = self.returns[start..].iter()
            .map(|&r| (r - mean).powi(2))
            .sum::<f64>() / (window_size - 1) as f64;
        
        variance.sqrt()
    }
    
    /// Обновить адаптивные пороги
    fn update_adaptive_thresholds(&mut self, current_volatility: f64) {
        if self.volatilities.len() < 10 {
            self.current_result.adaptive_threshold_low = current_volatility * 0.7;
            self.current_result.adaptive_threshold_high = current_volatility * 1.3;
            return;
        }
        
        // Рассчитываем долгосрочную волатильность
        let _long_term_vol = self.long_term_vol.update_bar(0.0, 0.0, 0.0, current_volatility, 0.0);
        
        // Адаптивно обновляем пороги на основе исторических данных
        let vol_percentiles = self.calculate_volatility_percentiles();
        
        let new_low_threshold = vol_percentiles.0;
        let new_high_threshold = vol_percentiles.1;
        
        // Сглаживаем изменения порогов
        let speed = self.threshold_adaptation_speed;
        self.current_result.adaptive_threshold_low = 
            speed * new_low_threshold + (1.0 - speed) * self.current_result.adaptive_threshold_low;
        self.current_result.adaptive_threshold_high = 
            speed * new_high_threshold + (1.0 - speed) * self.current_result.adaptive_threshold_high;
    }
    
    /// Рассчитать перцентили волатильности
    fn calculate_volatility_percentiles(&self) -> (f64, f64) {
        if self.volatilities.len() < 10 {
            return (0.0, 0.0);
        }

        // 🚀 O(n) quickselect instead of O(n log n) sorting
        let mut sorted_vols: Vec<f64> = self.volatilities.iter().cloned().collect();

        let len = sorted_vols.len();
        let percentile_25 = quickselect_nth(&mut sorted_vols, len / 4);
        let percentile_75 = quickselect_nth(&mut sorted_vols, 3 * len / 4);

        (percentile_25, percentile_75)
    }
    
    /// Классифицировать режим волатильности
    fn classify_volatility_regime(&mut self, volatility: f64) -> VolatilityRegime {
        let low_threshold = self.current_result.adaptive_threshold_low;
        let high_threshold = self.current_result.adaptive_threshold_high;
        
        // Определяем дополнительные пороги
        let very_low_threshold = low_threshold * 0.7;
        let very_high_threshold = high_threshold * 1.3;
        let extreme_threshold = high_threshold * 1.8;
        
        let regime = if volatility <= very_low_threshold {
            VolatilityRegime::VeryLow
        } else if volatility <= low_threshold {
            VolatilityRegime::Low
        } else if volatility <= high_threshold {
            VolatilityRegime::Normal
        } else if volatility <= very_high_threshold {
            VolatilityRegime::High
        } else if volatility <= extreme_threshold {
            VolatilityRegime::VeryHigh
        } else {
            VolatilityRegime::Extreme
        };
        
        // Рассчитываем вероятность режима
        self.current_result.regime_probability = self.calculate_regime_probability(volatility, regime);
        
        // Рассчитываем оценку волатильности
        self.current_result.volatility_score = self.regime_smoother.update_bar(0.0, 0.0, 0.0, regime.as_f64(), 0.0);
        
        // Сохраняем в историю
        if self.regime_history.len() >= 16 {
            self.regime_history.remove(0);
        }
        self.regime_history.push(regime);
        
        regime
    }
    
    /// Рассчитать вероятность режима
    fn calculate_regime_probability(&self, volatility: f64, regime: VolatilityRegime) -> f64 {
        let low_threshold = self.current_result.adaptive_threshold_low;
        let high_threshold = self.current_result.adaptive_threshold_high;
        
        // Вероятность основана на расстоянии до границ режима
        match regime {
            VolatilityRegime::VeryLow => {
                let threshold = low_threshold * 0.7;
                if volatility <= threshold {
                    1.0 - (volatility / threshold).min(1.0)
                } else {
                    0.0
                }
            },
            VolatilityRegime::Low => {
                let center = (low_threshold * 0.7 + low_threshold) / 2.0;
                let distance = (volatility - center).abs();
                let max_distance = (low_threshold - low_threshold * 0.7) / 2.0;
                (1.0 - distance / max_distance).max(0.0)
            },
            VolatilityRegime::Normal => {
                let center = (low_threshold + high_threshold) / 2.0;
                let distance = (volatility - center).abs();
                let max_distance = (high_threshold - low_threshold) / 2.0;
                (1.0 - distance / max_distance).max(0.0)
            },
            VolatilityRegime::High => {
                let center = (high_threshold + high_threshold * 1.3) / 2.0;
                let distance = (volatility - center).abs();
                let max_distance = (high_threshold * 1.3 - high_threshold) / 2.0;
                (1.0 - distance / max_distance).max(0.0)
            },
            VolatilityRegime::VeryHigh => {
                let center = (high_threshold * 1.3 + high_threshold * 1.8) / 2.0;
                let distance = (volatility - center).abs();
                let max_distance = (high_threshold * 1.8 - high_threshold * 1.3) / 2.0;
                (1.0 - distance / max_distance).max(0.0)
            },
            VolatilityRegime::Extreme => {
                let threshold = high_threshold * 1.8;
                if volatility >= threshold {
                    (volatility / threshold - 1.0).min(1.0)
                } else {
                    0.0
                }
            },
        }
    }
    
    /// Анализировать устойчивость режима
    fn analyze_regime_persistence(&mut self, current_regime: VolatilityRegime) {
        if self.regime_history.len() < 5 {
            self.current_result.regime_persistence = 0.0;
            self.current_result.transition_probability = 1.0;
            return;
        }
        
        // Подсчитываем, сколько последних периодов режим остается тем же
        let mut persistence_count = 0;
        for &regime in self.regime_history.iter().rev() {
            if regime == current_regime {
                persistence_count += 1;
            } else {
                break;
            }
        }
        
        // Устойчивость режима
        self.current_result.regime_persistence = (persistence_count as f64 / self.regime_history.len() as f64).min(1.0);
        
        // Вероятность смены режима (обратная к устойчивости)
        self.current_result.transition_probability = 1.0 - self.current_result.regime_persistence;
    }
    
    /// Рассчитать тренд волатильности
    fn calculate_volatility_trend(&mut self) {
        if self.volatilities.len() < 10 {
            self.current_result.volatility_trend = 0.0;
            return;
        }
        
        let len = self.volatilities.len();
        let recent_window = 5.min(len);
        let older_window = 10.min(len);
        
        // Средняя волатильность за последние периоды
        let recent_avg: f64 = self.volatilities[len - recent_window..].iter().sum::<f64>() / recent_window as f64;
        let older_avg: f64 = self.volatilities[len - older_window..len - recent_window].iter().sum::<f64>() / (older_window - recent_window) as f64;
        
        // Тренд как нормализованное изменение
        if older_avg > 0.0 {
            self.current_result.volatility_trend = ((recent_avg - older_avg) / older_avg).clamp(-1.0, 1.0);
        } else {
            self.current_result.volatility_trend = 0.0;
        }
    }
    
    /// Генерировать сигналы смены режима
    fn generate_regime_signals(&mut self) {
        if !self.is_ready || self.regime_history.len() < 2 {
            self.current_result.regime_signal = 0;
            return;
        }
        
        let current_regime = self.current_result.current_regime;
        let prev_regime = self.regime_history[self.regime_history.len() - 2];
        
        // Сигналы при значительной смене режима
        match (prev_regime, current_regime) {
            // Переход к высокой волатильности
            (VolatilityRegime::Low | VolatilityRegime::Normal, VolatilityRegime::High | VolatilityRegime::VeryHigh | VolatilityRegime::Extreme) => {
                self.current_result.regime_signal = 1;
            },
            // Переход к низкой волатильности
            (VolatilityRegime::High | VolatilityRegime::VeryHigh | VolatilityRegime::Extreme, VolatilityRegime::Low | VolatilityRegime::VeryLow) => {
                self.current_result.regime_signal = -1;
            },
            _ => {
                self.current_result.regime_signal = 0;
            }
        }
    }
    
    /// Получить текущий режим
    pub fn current_regime(&self) -> VolatilityRegime {
        self.current_result.current_regime
    }
    
    /// Получить полный результат
    pub fn result(&self) -> DynamicVolatilityRegimeResult {
        self.current_result
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.atr.reset();
        self.volatility_ma.reset();
        self.long_term_vol.reset();
        self.regime_smoother.reset();
        
        self.returns.clear();
        self.volatilities.clear();
        self.regime_scores.clear();
        self.regime_history.clear();
        
        self.conditional_variance = 0.0;
        self.current_result = DynamicVolatilityRegimeResult::empty();
        
        self.prev_price = None;
        self.is_ready = false;
        self.update_count = 0;
    }
    
    /// Генерировать торговый сигнал
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        self.current_result.regime_signal
    }
    
    /// Получить информацию о текущем состоянии
    pub fn info(&self) -> String {
        let result = self.current_result;
        
        format!(
            "Volatility Regime: {} (prob: {:.2}), Score: {:.1}, Persistence: {:.2}, Trend: {:.2}",
            result.current_regime.as_str(),
            result.regime_probability,
            result.volatility_score,
            result.regime_persistence,
            result.volatility_trend
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("regime".to_string(), self.current_result.current_regime.as_f64());
        values.insert("regime_probability".to_string(), self.current_result.regime_probability);
        values.insert("volatility_score".to_string(), self.current_result.volatility_score);
        values.insert("regime_persistence".to_string(), self.current_result.regime_persistence);
        values.insert("transition_probability".to_string(), self.current_result.transition_probability);
        values.insert("volatility_trend".to_string(), self.current_result.volatility_trend);
        values.insert("adaptive_threshold_low".to_string(), self.current_result.adaptive_threshold_low);
        values.insert("adaptive_threshold_high".to_string(), self.current_result.adaptive_threshold_high);
        values.insert("garch_volatility".to_string(), self.current_result.garch_volatility);
        values
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    
    /// Получить GARCH параметры
    pub fn garch_parameters(&self) -> (f64, f64, f64) {
        (self.garch_alpha, self.garch_beta, self.garch_omega)
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.volatility_score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_volatility_regime_creation() {
        let dvr = DynamicVolatilityRegime::new();
        assert!(!dvr.is_ready());
        assert_eq!(dvr.garch_parameters(), (0.1, 0.85, 0.01));
    }
    
    #[test]
    fn test_dynamic_volatility_regime_with_parameters() {
        let dvr = DynamicVolatilityRegime::with_parameters(0.15, 0.8, 0.02, 0.1);
        assert_eq!(dvr.garch_parameters(), (0.15, 0.8, 0.02));
    }
    
    #[test]
    fn test_dynamic_volatility_regime_update() {
        let mut dvr = DynamicVolatilityRegime::new();
        
        // Добавляем данные с изменяющейся волатильностью
        for i in 0..30 {
            let base_price = 100.0;
            let volatility_factor = if i < 15 { 0.5 } else { 2.0 }; // Смена режима
            let price = base_price + (i as f64 * 0.1).sin() * volatility_factor;
            
            let result = dvr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            
            if i > 25 {
                assert!(dvr.is_ready());
                assert!(result.volatility_score >= 1.0 && result.volatility_score <= 6.0);
                assert!(result.regime_probability >= 0.0 && result.regime_probability <= 1.0);
            }
        }
    }
    
    #[test]
    fn test_volatility_regime_classification() {
        let mut dvr = DynamicVolatilityRegime::new();
        
        // Генерируем данные с известными режимами волатильности
        for i in 0..50 {
            let price = if i < 25 {
                100.0 + (i as f64 * 0.01).sin() * 0.1  // Низкая волатильность
            } else {
                100.0 + (i as f64 * 0.1).sin() * 5.0   // Высокая волатильность
            };
            
            dvr.update_bar(price, price + 0.1, price - 0.1, price, 1000.0);
        }
        
        assert!(dvr.is_ready());
        // После смены на высокую волатильность режим должен измениться
        let regime = dvr.current_regime();
        assert!(matches!(regime, VolatilityRegime::High | VolatilityRegime::VeryHigh | VolatilityRegime::Extreme));
    }
    
    #[test]
    fn test_dynamic_volatility_regime_reset() {
        let mut dvr = DynamicVolatilityRegime::new();
        
        for i in 0..25 {
            let price = 100.0 + i as f64;
            dvr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        
        dvr.reset();
        assert!(!dvr.is_ready());
        assert_eq!(dvr.update_count(), 0);
    }
} 






















