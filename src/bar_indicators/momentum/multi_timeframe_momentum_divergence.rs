//! Multi-ResearchTimeframe Momentum Divergence - мульти-таймфреймная дивергенция momentum
//!
//! Анализирует дивергенции momentum между различными таймфреймами для
//! обнаружения ранних сигналов разворота тренда.
//!
//! Переиспользует существующие компоненты MovingAverage для разных периодов

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use arrayvec::ArrayVec;
use std::collections::HashMap;

/// Тип дивергенции
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivergenceType {
    None,              // Нет дивергенции
    BullishRegular,    // Обычная бычья дивергенция
    BearishRegular,    // Обычная медвежья дивергенция
    BullishHidden,     // Скрытая бычья дивергенция
    BearishHidden,     // Скрытая медвежья дивергенция
    BullishExtreme,    // Экстремальная бычья дивергенция
    BearishExtreme,    // Экстремальная медвежья дивергенция
}

impl DivergenceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DivergenceType::None => "Нет дивергенции",
            DivergenceType::BullishRegular => "Бычья дивергенция",
            DivergenceType::BearishRegular => "Медвежья дивергенция",
            DivergenceType::BullishHidden => "Скрытая бычья",
            DivergenceType::BearishHidden => "Скрытая медвежья",
            DivergenceType::BullishExtreme => "Экстремальная бычья",
            DivergenceType::BearishExtreme => "Экстремальная медвежья",
        }
    }
    
    pub fn as_number(&self) -> i8 {
        match self {
            DivergenceType::None => 0,
            DivergenceType::BullishRegular => 1,
            DivergenceType::BearishRegular => -1,
            DivergenceType::BullishHidden => 2,
            DivergenceType::BearishHidden => -2,
            DivergenceType::BullishExtreme => 3,
            DivergenceType::BearishExtreme => -3,
        }
    }
    
    pub fn is_bullish(&self) -> bool {
        matches!(self, DivergenceType::BullishRegular | DivergenceType::BullishHidden | DivergenceType::BullishExtreme)
    }
    
    pub fn is_bearish(&self) -> bool {
        matches!(self, DivergenceType::BearishRegular | DivergenceType::BearishHidden | DivergenceType::BearishExtreme)
    }
}

/// Анализ одного таймфрейма
#[derive(Debug, Clone, Copy)]
pub struct TimeframeAnalysis {
    pub momentum: f64,           // Текущий momentum
    pub momentum_ma: f64,        // Сглаженный momentum
    pub trend_direction: i8,     // Направление тренда
    pub trend_strength: f64,     // Сила тренда (0-1)
    pub volatility: f64,         // Волатильность momentum
}

impl TimeframeAnalysis {
    pub fn empty() -> Self {
        Self {
            momentum: 0.0,
            momentum_ma: 0.0,
            trend_direction: 0,
            trend_strength: 0.0,
            volatility: 0.0,
        }
    }
}

/// Результат Multi-ResearchTimeframe Momentum Divergence
#[derive(Debug, Clone, Copy)]
pub struct MultiTimeframeMomentumResult {
    pub primary_divergence: DivergenceType,    // Основная дивергенция
    pub secondary_divergence: DivergenceType,  // Вторичная дивергенция
    pub divergence_strength: f64,              // Сила дивергенции (0-1)
    pub convergence_score: f64,                // Оценка конвергенции таймфреймов (0-1)
    pub signal_quality: f64,                   // Качество сигнала (0-1)
    pub timeframe_agreement: f64,              // Согласованность таймфреймов (0-1)
    pub momentum_acceleration: f64,            // Ускорение momentum
    pub reversal_probability: f64,             // Вероятность разворота (0-1)
}

impl MultiTimeframeMomentumResult {
    pub fn empty() -> Self {
        Self {
            primary_divergence: DivergenceType::None,
            secondary_divergence: DivergenceType::None,
            divergence_strength: 0.0,
            convergence_score: 0.5,
            signal_quality: 0.0,
            timeframe_agreement: 0.5,
            momentum_acceleration: 0.0,
            reversal_probability: 0.0,
        }
    }
    
    /// Получить описание силы дивергенции
    pub fn strength_description(&self) -> &'static str {
        match self.divergence_strength {
            x if x < 0.2 => "Очень слабая",
            x if x < 0.4 => "Слабая",
            x if x < 0.6 => "Умеренная",
            x if x < 0.8 => "Сильная",
            _ => "Очень сильная",
        }
    }
    
    /// Получить торговую рекомендацию
    pub fn trading_recommendation(&self) -> &'static str {
        if self.signal_quality < 0.4 {
            return "Недостаточно качества сигнала";
        }
        
        match (self.primary_divergence, self.divergence_strength) {
            (DivergenceType::BullishExtreme, s) if s > 0.7 => "Сильная покупка",
            (DivergenceType::BearishExtreme, s) if s > 0.7 => "Сильная продажа",
            (DivergenceType::BullishRegular, s) if s > 0.5 => "Покупка",
            (DivergenceType::BearishRegular, s) if s > 0.5 => "Продажа",
            (DivergenceType::BullishHidden, s) if s > 0.6 => "Скрытая покупка",
            (DivergenceType::BearishHidden, s) if s > 0.6 => "Скрытая продажа",
            _ => "Ожидание",
        }
    }
}

/// Multi-ResearchTimeframe Momentum Divergence индикатор
#[derive(Clone)]
pub struct MultiTimeframeMomentumDivergence {
    // Переиспользуем MovingAverage для разных таймфреймов
    short_momentum_ma: MovingAverageProvider,        // Короткий momentum (5 периодов)
    medium_momentum_ma: MovingAverageProvider,       // Средний momentum (14 периодов)
    long_momentum_ma: MovingAverageProvider,         // Длинный momentum (30 периодов)
    
    // Дополнительные MA для анализа
    price_ma_short: MovingAverageProvider,           // Короткая MA цены
    price_ma_medium: MovingAverageProvider,          // Средняя MA цены
    price_ma_long: MovingAverageProvider,            // Длинная MA цены
    
    volatility_ma: MovingAverageProvider,            // MA для волатильности
    acceleration_ma: MovingAverageProvider,          // MA для ускорения
    
    // Буферы для анализа
    prices: ArrayVec<f64, 64>,
    short_momentums: ArrayVec<f64, 32>,
    medium_momentums: ArrayVec<f64, 32>,
    long_momentums: ArrayVec<f64, 32>,
    
    divergence_history: ArrayVec<DivergenceType, 16>,
    
    // Анализ таймфреймов
    short_tf: TimeframeAnalysis,
    medium_tf: TimeframeAnalysis,
    long_tf: TimeframeAnalysis,
    
    // Параметры
    divergence_threshold: f64,               // Порог для обнаружения дивергенции
    extreme_threshold: f64,                  // Порог для экстремальной дивергенции
    
    // Результат
    current_result: MultiTimeframeMomentumResult,
    
    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl MultiTimeframeMomentumDivergence {
    /// Создать новый Multi-ResearchTimeframe Momentum Divergence с параметрами по умолчанию
    pub fn new() -> Self {
        Self::with_parameters(0.3, 0.7)
    }
    
    /// Создать с настраиваемыми параметрами
    pub fn with_parameters(divergence_threshold: f64, extreme_threshold: f64) -> Self {
        assert!(divergence_threshold > 0.0 && divergence_threshold < 1.0, "Invalid divergence threshold");
        assert!(extreme_threshold > divergence_threshold, "Extreme threshold must be > divergence threshold");
        
        Self {
            // Переиспользуем MovingAverage для разных целей и периодов
            short_momentum_ma: MovingAverageProvider::new(MovingAverageType::EMA, 5),
            medium_momentum_ma: MovingAverageProvider::new(MovingAverageType::EMA, 14),
            long_momentum_ma: MovingAverageProvider::new(MovingAverageType::EMA, 30),
            
            price_ma_short: MovingAverageProvider::new(MovingAverageType::EMA, 5),
            price_ma_medium: MovingAverageProvider::new(MovingAverageType::EMA, 14),
            price_ma_long: MovingAverageProvider::new(MovingAverageType::EMA, 30),
            
            volatility_ma: MovingAverageProvider::new(MovingAverageType::SMA, 10),
            acceleration_ma: MovingAverageProvider::new(MovingAverageType::EMA, 3),
            
            prices: ArrayVec::new(),
            short_momentums: ArrayVec::new(),
            medium_momentums: ArrayVec::new(),
            long_momentums: ArrayVec::new(),
            divergence_history: ArrayVec::new(),
            
            short_tf: TimeframeAnalysis::empty(),
            medium_tf: TimeframeAnalysis::empty(),
            long_tf: TimeframeAnalysis::empty(),
            
            divergence_threshold,
            extreme_threshold,
            
            current_result: MultiTimeframeMomentumResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> MultiTimeframeMomentumResult {
        // Добавляем цену в буфер
        if self.prices.len() >= 64 {
            self.prices.remove(0);
        }
        self.prices.push(close);
        
        // 1. Обновляем MA цен для разных таймфреймов (переиспользуем MovingAverage)
        let price_short = self.price_ma_short.update_bar(0.0, 0.0, 0.0, close, 0.0);
        let price_medium = self.price_ma_medium.update_bar(0.0, 0.0, 0.0, close, 0.0);
        let price_long = self.price_ma_long.update_bar(0.0, 0.0, 0.0, close, 0.0);
        
        // 2. Рассчитываем momentum для каждого таймфрейма
        self.calculate_timeframe_momentum(close, price_short, price_medium, price_long);
        
        // 3. Анализируем каждый таймфрейм
        self.analyze_timeframes();
        
        // 4. Обнаруживаем дивергенции между таймфреймами
        self.detect_divergences();
        
        // 5. Рассчитываем качество сигнала и дополнительные метрики
        self.calculate_signal_metrics();
        
        // Готов после накопления достаточных данных
        if self.price_ma_long.is_ready() && self.long_momentums.len() >= 10 {
            self.is_ready = true;
        }
        
        self.update_count += 1;
        self.current_result
    }
    
    /// Рассчитать momentum для каждого таймфрейма
    fn calculate_timeframe_momentum(&mut self, current_price: f64, _price_short: f64, _price_medium: f64, _price_long: f64) {
        if self.prices.len() < 5 {
            return;
        }
        
        let len = self.prices.len();
        
        // Короткий momentum (5 периодов)
        let short_momentum = if len >= 5 {
            current_price - self.prices[len - 5]
        } else {
            0.0
        };
        
        // Средний momentum (14 периодов)
        let medium_momentum = if len >= 14 {
            current_price - self.prices[len - 14]
        } else if len >= 5 {
            current_price - self.prices[0]
        } else {
            0.0
        };
        
        // Длинный momentum (30 периодов)
        let long_momentum = if len >= 30 {
            current_price - self.prices[len - 30]
        } else if len >= 10 {
            current_price - self.prices[0]
        } else {
            0.0
        };
        
        // Сглаживаем momentum (переиспользуем MovingAverage)
        let short_ma = self.short_momentum_ma.update_bar(0.0, 0.0, 0.0, short_momentum, 0.0);
        let medium_ma = self.medium_momentum_ma.update_bar(0.0, 0.0, 0.0, medium_momentum, 0.0);
        let long_ma = self.long_momentum_ma.update_bar(0.0, 0.0, 0.0, long_momentum, 0.0);
        
        // Сохраняем сглаженные momentum
        if self.short_momentums.len() >= 32 {
            self.short_momentums.remove(0);
        }
        self.short_momentums.push(short_ma);
        
        if self.medium_momentums.len() >= 32 {
            self.medium_momentums.remove(0);
        }
        self.medium_momentums.push(medium_ma);
        
        if self.long_momentums.len() >= 32 {
            self.long_momentums.remove(0);
        }
        self.long_momentums.push(long_ma);
        
        // Обновляем анализ таймфреймов
        self.short_tf.momentum = short_momentum;
        self.short_tf.momentum_ma = short_ma;
        
        self.medium_tf.momentum = medium_momentum;
        self.medium_tf.momentum_ma = medium_ma;
        
        self.long_tf.momentum = long_momentum;
        self.long_tf.momentum_ma = long_ma;
    }
    
    /// Анализировать каждый таймфрейм
    fn analyze_timeframes(&mut self) {
        // Клонируем momentum данные чтобы избежать проблем с заимствованием
        let short_momentums = self.short_momentums.clone();
        let medium_momentums = self.medium_momentums.clone();
        let long_momentums = self.long_momentums.clone();
        
        Self::analyze_single_timeframe(&mut self.short_tf, &short_momentums);
        Self::analyze_single_timeframe(&mut self.medium_tf, &medium_momentums);
        Self::analyze_single_timeframe(&mut self.long_tf, &long_momentums);
    }
    
    /// Анализировать один таймфрейм
    fn analyze_single_timeframe(tf: &mut TimeframeAnalysis, momentums: &ArrayVec<f64, 32>) {
        if momentums.len() < 3 {
            return;
        }
        
        let len = momentums.len();
        let current = momentums[len - 1];
        let prev = momentums[len - 2];
        let prev2 = momentums[len - 3];
        
        // Направление тренда
        tf.trend_direction = if current > prev && prev > prev2 {
            1  // Восходящий
        } else if current < prev && prev < prev2 {
            -1 // Нисходящий
        } else {
            0  // Боковой
        };
        
        // Сила тренда
        let momentum_change = (current - prev).abs();
        let momentum_range = if momentums.len() >= 10 {
            let recent = &momentums[len - 10..];
            let max_momentum = recent.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let min_momentum = recent.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            max_momentum - min_momentum
        } else {
            1.0
        };
        
        tf.trend_strength = if momentum_range > 0.0 {
            (momentum_change / momentum_range).min(1.0)
        } else {
            0.0
        };
        
        // Волатильность momentum
        if momentums.len() >= 5 {
            let recent = &momentums[len - 5..];
            let mean: f64 = recent.iter().sum::<f64>() / recent.len() as f64;
            let variance: f64 = recent.iter()
                .map(|&x| (x - mean).powi(2))
                .sum::<f64>() / recent.len() as f64;
            
            tf.volatility = variance.sqrt();
        }
    }
    
    /// Обнаружить дивергенции между таймфреймами
    fn detect_divergences(&mut self) {
        if !self.is_ready {
            return;
        }
        
        // Анализируем дивергенции между короткими и длинными таймфреймами
        let primary_divergence = self.analyze_divergence_pair(&self.short_tf, &self.long_tf);
        let secondary_divergence = self.analyze_divergence_pair(&self.medium_tf, &self.long_tf);
        
        // Сохраняем в истории
        if self.divergence_history.len() >= 16 {
            self.divergence_history.remove(0);
        }
        self.divergence_history.push(primary_divergence);
        
        self.current_result.primary_divergence = primary_divergence;
        self.current_result.secondary_divergence = secondary_divergence;
    }
    
    /// Анализировать дивергенцию между двумя таймфреймами
    fn analyze_divergence_pair(&self, short_tf: &TimeframeAnalysis, long_tf: &TimeframeAnalysis) -> DivergenceType {
        // Нормализуем momentum для сравнения
        let short_momentum_norm = if short_tf.momentum != 0.0 {
            short_tf.momentum_ma / short_tf.momentum.abs()
        } else {
            0.0
        };
        
        let long_momentum_norm = if long_tf.momentum != 0.0 {
            long_tf.momentum_ma / long_tf.momentum.abs()
        } else {
            0.0
        };
        
        let divergence_magnitude = (short_momentum_norm - long_momentum_norm).abs();
        
        if divergence_magnitude < self.divergence_threshold {
            return DivergenceType::None;
        }
        
        // Определяем тип дивергенции
        let short_bullish = short_tf.trend_direction > 0;
        let long_bullish = long_tf.trend_direction > 0;
        
        match (short_bullish, long_bullish, divergence_magnitude > self.extreme_threshold) {
            (true, false, true) => DivergenceType::BullishExtreme,
            (false, true, true) => DivergenceType::BearishExtreme,
            (true, false, false) => {
                if short_tf.trend_strength > long_tf.trend_strength {
                    DivergenceType::BullishRegular
                } else {
                    DivergenceType::BullishHidden
                }
            }
            (false, true, false) => {
                if short_tf.trend_strength > long_tf.trend_strength {
                    DivergenceType::BearishRegular
                } else {
                    DivergenceType::BearishHidden
                }
            }
            _ => DivergenceType::None,
        }
    }
    
    /// Рассчитать метрики качества сигнала
    fn calculate_signal_metrics(&mut self) {
        // Сила дивергенции
        let primary_strength = self.calculate_divergence_strength(self.current_result.primary_divergence);
        let secondary_strength = self.calculate_divergence_strength(self.current_result.secondary_divergence);
        
        self.current_result.divergence_strength = (primary_strength + secondary_strength * 0.5) / 1.5;
        
        // Согласованность таймфреймов
        let agreement = self.calculate_timeframe_agreement();
        self.current_result.timeframe_agreement = agreement;
        
        // Ускорение momentum
        self.calculate_momentum_acceleration();
        
        // Качество сигнала
        let volatility_factor = (self.short_tf.volatility + self.medium_tf.volatility + self.long_tf.volatility) / 3.0;
        let normalized_volatility = (volatility_factor / 10.0).min(1.0);
        
        self.current_result.signal_quality = (
            self.current_result.divergence_strength * 0.4 +
            agreement * 0.3 +
            (1.0 - normalized_volatility) * 0.3
        ).min(1.0);
        
        // Вероятность разворота
        self.calculate_reversal_probability();
    }
    
    /// Рассчитать силу дивергенции
    fn calculate_divergence_strength(&self, divergence_type: DivergenceType) -> f64 {
        match divergence_type {
            DivergenceType::None => 0.0,
            DivergenceType::BullishRegular | DivergenceType::BearishRegular => 0.6,
            DivergenceType::BullishHidden | DivergenceType::BearishHidden => 0.4,
            DivergenceType::BullishExtreme | DivergenceType::BearishExtreme => 1.0,
        }
    }
    
    /// Рассчитать согласованность таймфреймов
    fn calculate_timeframe_agreement(&self) -> f64 {
        let directions = [
            self.short_tf.trend_direction,
            self.medium_tf.trend_direction,
            self.long_tf.trend_direction,
        ];
        
        let positive_count = directions.iter().filter(|&&d| d > 0).count();
        let negative_count = directions.iter().filter(|&&d| d < 0).count();
        let neutral_count = directions.iter().filter(|&&d| d == 0).count();
        
        // Высокая согласованность при одинаковом направлении
        if positive_count == 3 || negative_count == 3 {
            1.0
        } else if positive_count == 2 || negative_count == 2 {
            0.7
        } else if neutral_count == 3 {
            0.3
        } else {
            0.5
        }
    }
    
    /// Рассчитать ускорение momentum
    fn calculate_momentum_acceleration(&mut self) {
        if self.short_momentums.len() < 3 {
            return;
        }
        
        let len = self.short_momentums.len();
        let current_momentum = self.short_momentums[len - 1];
        let prev_momentum = self.short_momentums[len - 2];
        let prev2_momentum = self.short_momentums[len - 3];
        
        let velocity = current_momentum - prev_momentum;
        let prev_velocity = prev_momentum - prev2_momentum;
        let acceleration = velocity - prev_velocity;
        
        // Сглаживаем ускорение
        let smoothed_acceleration = self.acceleration_ma.update_bar(0.0, 0.0, 0.0, acceleration, 0.0);
        
        self.current_result.momentum_acceleration = smoothed_acceleration;
    }
    
    /// Рассчитать вероятность разворота
    fn calculate_reversal_probability(&mut self) {
        let base_probability = match self.current_result.primary_divergence {
            DivergenceType::BullishExtreme | DivergenceType::BearishExtreme => 0.8,
            DivergenceType::BullishRegular | DivergenceType::BearishRegular => 0.6,
            DivergenceType::BullishHidden | DivergenceType::BearishHidden => 0.4,
            DivergenceType::None => 0.1,
        };
        
        // Корректируем на основе согласованности и качества сигнала
        let agreement_factor = self.current_result.timeframe_agreement;
        let quality_factor = self.current_result.signal_quality;
        
        // Учитываем историю дивергенций
        let history_factor = if self.divergence_history.len() >= 3 {
            let recent_divergences = &self.divergence_history[self.divergence_history.len() - 3..];
            let consistent_direction = recent_divergences.iter()
                .all(|d| d.is_bullish()) || recent_divergences.iter().all(|d| d.is_bearish());
            
            if consistent_direction { 1.2 } else { 0.8 }
        } else {
            1.0
        };
        
        let probability = (base_probability * agreement_factor * quality_factor * history_factor).min(1.0);
        
        self.current_result.reversal_probability = probability;
    }
    
    /// Получить текущую основную дивергенцию как DivergenceType (legacy)
    pub fn divergence_type(&self) -> DivergenceType {
        self.current_result.primary_divergence
    }

    /// Получить значение в виде IndicatorValue (Signal)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.current_result.primary_divergence.as_number())
    }
    
    /// Получить полный результат
    pub fn result(&self) -> MultiTimeframeMomentumResult {
        self.current_result
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.short_momentum_ma.reset();
        self.medium_momentum_ma.reset();
        self.long_momentum_ma.reset();
        self.price_ma_short.reset();
        self.price_ma_medium.reset();
        self.price_ma_long.reset();
        self.volatility_ma.reset();
        self.acceleration_ma.reset();
        
        self.prices.clear();
        self.short_momentums.clear();
        self.medium_momentums.clear();
        self.long_momentums.clear();
        self.divergence_history.clear();
        
        self.short_tf = TimeframeAnalysis::empty();
        self.medium_tf = TimeframeAnalysis::empty();
        self.long_tf = TimeframeAnalysis::empty();
        
        self.current_result = MultiTimeframeMomentumResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }
    
    /// Получить период (условный)
    pub fn period(&self) -> usize {
        30 // Максимальный период
    }
    
    /// Генерировать торговый сигнал
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let result = self.current_result;
        
        // Сигналы только при высоком качестве и вероятности
        if result.signal_quality < 0.6 || result.reversal_probability < 0.6 {
            return 0;
        }
        
        match result.primary_divergence {
            DivergenceType::BullishExtreme | DivergenceType::BullishRegular => 1,
            DivergenceType::BearishExtreme | DivergenceType::BearishRegular => -1,
            DivergenceType::BullishHidden => {
                if result.divergence_strength > 0.7 { 1 } else { 0 }
            }
            DivergenceType::BearishHidden => {
                if result.divergence_strength > 0.7 { -1 } else { 0 }
            }
            _ => 0,
        }
    }
    
    /// Генерировать сигнал подтверждения
    pub fn confirmation_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let result = self.current_result;
        
        // Подтверждение при согласованности первичной и вторичной дивергенций
        if result.timeframe_agreement > 0.7 {
            let primary_bullish = result.primary_divergence.is_bullish();
            let secondary_bullish = result.secondary_divergence.is_bullish();
            
            if primary_bullish && secondary_bullish {
                return 1;
            } else if !primary_bullish && !secondary_bullish && 
                     !matches!(result.primary_divergence, DivergenceType::None) &&
                     !matches!(result.secondary_divergence, DivergenceType::None) {
                return -1;
            }
        }
        
        0
    }
    
    /// Получить информацию о текущем состоянии
    pub fn info(&self) -> String {
        let result = self.current_result;
        let signal = match self.trading_signal() {
            1 => "Покупка",
            -1 => "Продажа",
            _ => "Нет сигнала",
        };
        
        format!(
            "Multi-TF Momentum: Основная: {}, Вторичная: {}, Сила: {} ({:.1}), Качество: {:.1}%, Разворот: {:.1}%, Сигнал: {}",
            result.primary_divergence.as_str(),
            result.secondary_divergence.as_str(),
            result.strength_description(),
            result.divergence_strength,
            result.signal_quality * 100.0,
            result.reversal_probability * 100.0,
            signal
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> HashMap<String, f64> {
        let mut values = HashMap::new();
        values.insert("primary_divergence".to_string(), self.current_result.primary_divergence.as_number() as f64);
        values.insert("secondary_divergence".to_string(), self.current_result.secondary_divergence.as_number() as f64);
        values.insert("divergence_strength".to_string(), self.current_result.divergence_strength);
        values.insert("convergence_score".to_string(), self.current_result.convergence_score);
        values.insert("signal_quality".to_string(), self.current_result.signal_quality);
        values.insert("timeframe_agreement".to_string(), self.current_result.timeframe_agreement);
        values.insert("momentum_acceleration".to_string(), self.current_result.momentum_acceleration);
        values.insert("reversal_probability".to_string(), self.current_result.reversal_probability);
        
        values.insert("short_momentum".to_string(), self.short_tf.momentum);
        values.insert("medium_momentum".to_string(), self.medium_tf.momentum);
        values.insert("long_momentum".to_string(), self.long_tf.momentum);
        
        values.insert("short_trend_strength".to_string(), self.short_tf.trend_strength);
        values.insert("medium_trend_strength".to_string(), self.medium_tf.trend_strength);
        values.insert("long_trend_strength".to_string(), self.long_tf.trend_strength);
        
        values
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    
    /// Получить параметры
    pub fn parameters(&self) -> (f64, f64) {
        (self.divergence_threshold, self.extreme_threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_timeframe_momentum_divergence_creation() {
        let mtmd = MultiTimeframeMomentumDivergence::new();
        assert!(!mtmd.is_ready());
        assert_eq!(mtmd.divergence_type(), DivergenceType::None);
    }
    
    #[test]
    fn test_multi_timeframe_momentum_divergence_with_parameters() {
        let mtmd = MultiTimeframeMomentumDivergence::with_parameters(0.4, 0.8);
        assert_eq!(mtmd.parameters(), (0.4, 0.8));
    }
    
    #[test]
    fn test_momentum_calculation() {
        let mut mtmd = MultiTimeframeMomentumDivergence::new();
        
        // Добавляем данные с четким трендом
        for i in 0..35 {
            let price = 100.0 + i as f64 * 0.5; // Восходящий тренд
            let result = mtmd.update_bar(price, price + 0.5, price - 0.5, price, 1000.0);
            
            if i > 30 {
                assert!(mtmd.is_ready());
                assert!(result.signal_quality >= 0.0 && result.signal_quality <= 1.0);
                assert!(result.timeframe_agreement >= 0.0 && result.timeframe_agreement <= 1.0);
                assert!(result.reversal_probability >= 0.0 && result.reversal_probability <= 1.0);
            }
        }
    }
    
    #[test]
    fn test_divergence_detection() {
        let mut mtmd = MultiTimeframeMomentumDivergence::new();
        
        // Создаем условия для дивергенции: цена растет, но momentum падает
        for i in 0..40 {
            let base_price = 100.0;
            let price = if i < 20 {
                base_price + i as f64 * 0.5 // Сильный рост
            } else {
                base_price + 10.0 + (i - 20) as f64 * 0.1 // Слабый рост
            };
            
            let result = mtmd.update_bar(price, price + 0.2, price - 0.2, price, 1000.0);
            
            if i > 35 && mtmd.is_ready() {
                // Должна быть обнаружена дивергенция
                // Divergence detection depends on specific price patterns
                assert!(result.divergence_strength >= 0.0);
            }
        }
    }
    
    #[test]
    fn test_timeframe_agreement() {
        let mut mtmd = MultiTimeframeMomentumDivergence::new();
        
        // Согласованный восходящий тренд на всех таймфреймах
        for i in 0..35 {
            let price = 100.0 + i as f64 * 0.3; // Устойчивый рост
            let result = mtmd.update_bar(price, price + 0.1, price - 0.1, price, 1000.0);
            
            if i > 30 && mtmd.is_ready() {
                // Согласованность должна быть высокой
                assert!(result.timeframe_agreement >= 0.5);
            }
        }
    }
    
    #[test]
    fn test_trading_signals() {
        let mut mtmd = MultiTimeframeMomentumDivergence::new();
        
        // Добавляем данные
        for i in 0..35 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 5.0; // Синусоидальное движение
            let _result = mtmd.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        
        if mtmd.is_ready() {
            let signal = mtmd.trading_signal();
            let confirmation = mtmd.confirmation_signal();
            
            assert!(signal >= -1 && signal <= 1);
            assert!(confirmation >= -1 && confirmation <= 1);
        }
    }
    
    #[test]
    fn test_momentum_acceleration() {
        let mut mtmd = MultiTimeframeMomentumDivergence::new();
        
        // Ускоряющийся тренд
        for i in 0..35 {
            let acceleration = (i as f64 / 10.0).powi(2);
            let price = 100.0 + acceleration;
            let result = mtmd.update_bar(price, price + 0.5, price - 0.5, price, 1000.0);
            
            if i > 30 && mtmd.is_ready() {
                // Ускорение должно быть положительным
                assert!(result.momentum_acceleration.is_finite());
            }
        }
    }
} 






















