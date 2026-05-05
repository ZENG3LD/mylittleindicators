//! adaptive_channels.rs: High-Performance Adaptive Channels
//! Адаптивные каналы - автоматическая адаптация к рыночным условиям
//! 
//! Особенности:
//! - Использует готовые компоненты: KaufmanAdaptiveMA, LinearRegressionMA, Atr
//! - Adaptive ATR для динамической ширины каналов
//! - Machine Learning inspired volatility clustering detection
//! - Автоматическая адаптация к рыночным режимам

use crate::bar_indicators::adaptive::kaufman_adaptive_ma::KaufmanAdaptiveMA;
use crate::bar_indicators::average::lr::LinearRegressionMA;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::volatility::atr::Atr;
use arrayvec::ArrayVec;
use serde::{Serialize, Deserialize};

/// Режимы адаптации каналов
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AdaptationMode {
    /// Адаптация к волатильности
    Volatility,
    /// Адаптация к тренду (сильнее в трендах, уже в боковиках)
    Trend,
    /// Адаптация к циклам (на основе доминирующего цикла)
    Cycle,
    /// Комбинированная адаптация (все факторы)
    Combined,
    /// Machine Learning адаптация (кластеризация волатильности)
    MachineLearning,
}

/// Тип центральной линии
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CenterLineType {
    /// Kaufman's Adaptive Moving Average - СТАНДАРТНАЯ
    KAMA,
    /// Быстрая KAMA
    FastKAMA,
    /// Медленная KAMA
    SlowKAMA,
    /// Linear Regression (адаптивный период)
    AdaptiveLinReg,
}

/// Сигналы адаптивных каналов
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AdaptiveSignal {
    /// Расширение каналов (рост волатильности)
    ChannelExpansion,
    /// Сужение каналов (снижение волатильности)
    ChannelContraction,
    /// Пробой при высокой адаптации (сильный сигнал)
    HighAdaptiveBreakout,
    /// Пробой при низкой адаптации (слабый сигнал)
    LowAdaptiveBreakout,
    /// Возврат к адаптивному центру
    ReturnToAdaptiveCenter,
    /// Адаптивный отскок от границы
    AdaptiveBounce,
    /// Вход в режим высокой волатильности
    HighVolatilityRegime,
    /// Вход в режим низкой волатильности
    LowVolatilityRegime,
}

/// Рыночный режим
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MarketRegime {
    /// Трендовый рынок
    Trending,
    /// Боковой рынок
    Ranging,
    /// Волатильный рынок
    Volatile,
    /// Спокойный рынок
    Quiet,
    /// Переходный режим
    Transition,
}

/// High-Performance Adaptive Channels
/// Архитектура: KaufmanAdaptiveMA для центральной линии + Atr для ширины + ML адаптация
#[derive(Debug, Clone)]
pub struct AdaptiveChannels {
    // Параметры
    period: usize,
    adaptation_mode: AdaptationMode,
    center_line_type: CenterLineType,
    volatility_lookback: usize,
    
    // Компоненты (используем готовые!)
    kama: KaufmanAdaptiveMA,                // ✅ Готовая мощная KAMA
    adaptive_atr: Atr,                      // ✅ ATR через готовый компонент
    regression_ma: Option<LinearRegressionMA>, // ✅ Для AdaptiveLinReg режима
    
    // Circular buffers для ML адаптации
    price_buffer: ArrayVec<f64, 512>,
    volatility_clusters: ArrayVec<f64, 100>,
    cluster_index: usize,
    cluster_filled: bool,
    
    // Адаптивные параметры
    volatility_factor: f64,
    trend_strength: f64,
    cycle_factor: f64,
    ml_adaptation_factor: f64,
    
    // Каналы
    upper_channel: f64,
    lower_channel: f64,
    channel_width: f64,
    
    // Дополнительные уровни
    upper_channel_2: f64,  // 2x адаптивная ширина
    lower_channel_2: f64,  // 2x адаптивная ширина
    
    // Рыночный режим
    current_regime: MarketRegime,
    regime_confidence: f64,
    
    // Статистика адаптации
    adaptation_level: f64,  // 0.0-1.0 (насколько сильно адаптируемся)
    volatility_percentile: f64,
    
    // Циклический анализ
    dominant_cycle: f64,
    cycle_strength: f64,
    
    // Машинное обучение компоненты
    volatility_mean: f64,
    volatility_std: f64,
    current_vol_cluster: f64,
    
    // Счетчики
    buffer_index: usize,
    buffer_filled: bool,
    bar_count: usize,
}

impl AdaptiveChannels {
    /// Создать адаптивные каналы со стандартными параметрами
    pub fn new() -> Self {
        Self::new_custom(
            30,  // period
            AdaptationMode::Combined,
            CenterLineType::KAMA,
            50   // volatility lookback
        )
    }
    
    /// Создать адаптивные каналы с кастомными параметрами
    /// period - период для KAMA и ATR
    /// adaptation_mode - режим адаптации (волатильность, тренд, комбинированный)
    /// center_line_type - тип центральной линии (KAMA, FastKAMA, SlowKAMA, etc.)
    /// volatility_lookback - период для анализа волатильности
    pub fn new_custom(
        period: usize,
        adaptation_mode: AdaptationMode,
        center_line_type: CenterLineType,
        volatility_lookback: usize
    ) -> Self {
        assert!(period > 0 && period <= 512, "Period must be between 1 and 512");
        assert!(volatility_lookback > 0 && volatility_lookback <= 512, "Volatility lookback must be between 1 and 512");
        
        // ✅ Выбираем конфигурацию KAMA
        let kama = match center_line_type {
            CenterLineType::KAMA => KaufmanAdaptiveMA::default(),       // 10, 2, 30
            CenterLineType::FastKAMA => KaufmanAdaptiveMA::fast(),      // 5, 1, 15
            CenterLineType::SlowKAMA => KaufmanAdaptiveMA::slow(),      // 20, 5, 50
            CenterLineType::AdaptiveLinReg => KaufmanAdaptiveMA::default(), // Fallback
        };
        
        // Создаем LinearRegressionMA только для AdaptiveLinReg режима
        let regression_ma = if matches!(center_line_type, CenterLineType::AdaptiveLinReg) {
            Some(LinearRegressionMA::new(period))
        } else {
            None
        };
        
        Self {
            period,
            adaptation_mode,
            center_line_type,
            volatility_lookback,
            kama,  // ✅ Готовая мощная KAMA!
            adaptive_atr: Atr::new(period, crate::bar_indicators::average::moving_average::MovingAverageType::RMA),
            regression_ma,
            price_buffer: ArrayVec::new(),
            volatility_clusters: ArrayVec::new(),
            cluster_index: 0,
            cluster_filled: false,
            volatility_factor: 1.0,
            trend_strength: 0.0,
            cycle_factor: 1.0,
            ml_adaptation_factor: 1.0,
            upper_channel: 0.0,
            lower_channel: 0.0,
            channel_width: 0.0,
            upper_channel_2: 0.0,
            lower_channel_2: 0.0,
            current_regime: MarketRegime::Transition,
            regime_confidence: 0.0,
            adaptation_level: 0.5,
            volatility_percentile: 0.5,
            dominant_cycle: 0.0,
            cycle_strength: 0.0,
            volatility_mean: 0.0,
            volatility_std: 0.0,
            current_vol_cluster: 0.0,
            buffer_index: 0,
            buffer_filled: false,
            bar_count: 0,
        }
    }
    
    /// Создать KAMA адаптивные каналы
    pub fn new_kama_adaptive() -> Self {
        Self::new_custom(
            20,
            AdaptationMode::Combined,
            CenterLineType::KAMA,
            40
        )
    }
    
    /// Создать быстрые KAMA каналы
    pub fn new_fast_kama() -> Self {
        Self::new_custom(
            15,
            AdaptationMode::Volatility,
            CenterLineType::FastKAMA,
            30
        )
    }
    
    /// Создать медленные KAMA каналы
    pub fn new_slow_kama() -> Self {
        Self::new_custom(
            50,
            AdaptationMode::Trend,
            CenterLineType::SlowKAMA,
            100
        )
    }
    
    /// Создать ML адаптивные каналы
    pub fn new_ml_adaptive() -> Self {
        Self::new_custom(
            30,
            AdaptationMode::MachineLearning,
            CenterLineType::KAMA,
            100
        )
    }
    
    /// Обновить каналы новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64, f64) {
        self.bar_count += 1;
        
        // Обновляем буферы
        self.update_buffers(open, high, low, close, volume);
        
        // ✅ Обновляем центральную линию через готовые компоненты
        let center_line = self.update_center_line(open, high, low, close, volume);
        
        // ✅ Обновляем adaptive ATR через готовый компонент
        let atr_value = self.adaptive_atr.update_bar(open, high, low, close, volume);
        
        // Определяем рыночный режим используя мощь KAMA
        self.determine_market_regime(high, low, close);
        
        // Обновляем факторы адаптации используя статистику KAMA
        self.update_adaptation_factors(high, low, close);
        
        // Обновляем ML адаптацию
        self.update_ml_adaptation(high, low);
        
        // Рассчитываем адаптивные каналы
        self.calculate_adaptive_channels(center_line, atr_value);
        
        (self.upper_channel, center_line, self.lower_channel)
    }
    
    /// Обновить буферы
    fn update_buffers(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) {
        // Добавляем цену в буфер
        if self.buffer_filled {
            self.price_buffer[self.buffer_index] = close;
        } else {
            self.price_buffer.push(close);
        }
        
        self.buffer_index = (self.buffer_index + 1) % self.volatility_lookback;
        
        if self.price_buffer.len() == self.volatility_lookback && !self.buffer_filled {
            self.buffer_filled = true;
        }
    }
    
    /// Обновить центральную линию
    fn update_center_line(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> f64 {
        match self.center_line_type {
            CenterLineType::AdaptiveLinReg => {
                // Используем LinearRegressionMA
                if let Some(ref mut regression) = self.regression_ma {
                    regression.update_bar(close, close, close, close, _volume)
                } else {
                    close
                }
            }
            _ => {
                // ✅ Используем готовую мощную KAMA!
                self.kama.update(close)
            }
        }
    }
    
    /// Определить рыночный режим используя мощь KAMA
    fn determine_market_regime(&mut self, high: f64, low: f64, _close: f64) {
        if !self.is_ready() {
            return;
        }
        
        // ✅ Используем статистику KAMA для анализа тренда
        let efficiency_ratio = self.kama.efficiency_ratio();
        let trend_consistency = self.kama.trend_consistency();
        
        // Анализируем волатильность
        let range = high - low;
        let avg_range = self.adaptive_atr.value().main();
        let volatility_ratio = if avg_range > 0.0 { range / avg_range } else { 1.0 };
        
        // Определяем режим на основе KAMA статистики
        self.current_regime = if efficiency_ratio > 0.7 && trend_consistency > 0.6 {
            // Высокая эффективность + консистентность = тренд
            MarketRegime::Trending
        } else if volatility_ratio > 1.5 {
            // Высокая волатильность
            if efficiency_ratio > 0.4 {
                MarketRegime::Volatile
            } else {
                MarketRegime::Ranging  // Волатильный боковик
            }
        } else if volatility_ratio < 0.7 && efficiency_ratio < 0.3 {
            // Низкая волатильность + низкая эффективность = спокойно
            MarketRegime::Quiet
        } else if efficiency_ratio < 0.4 && trend_consistency < 0.3 {
            // Слабый тренд = боковик
            MarketRegime::Ranging
        } else {
            MarketRegime::Transition
        };
        
        // Обновляем уверенность в режиме на основе KAMA метрик
        self.regime_confidence = match self.current_regime {
            MarketRegime::Trending => (efficiency_ratio + trend_consistency) / 2.0,
            MarketRegime::Volatile => volatility_ratio.min(1.0),
            MarketRegime::Quiet => 1.0 - volatility_ratio,
            MarketRegime::Ranging => 1.0 - efficiency_ratio,
            MarketRegime::Transition => 0.5,
        };
    }
    
    /// Обновить факторы адаптации используя статистику KAMA
    fn update_adaptation_factors(&mut self, high: f64, low: f64, _close: f64) {
        // ✅ Фактор волатильности
        let current_range = high - low;
        let avg_range = self.adaptive_atr.value().main();
        self.volatility_factor = if avg_range > 0.0 {
            (current_range / avg_range).clamp(0.3, 3.0)
        } else {
            1.0
        };
        
        // ✅ Фактор тренда из KAMA
        self.trend_strength = self.kama.efficiency_ratio();
        
        // ✅ Комбинированный уровень адаптации с учетом KAMA
        let kama_adaptive_period = self.kama.adaptive_period();
        let period_factor = if self.period as f64 > 0.0 {
            (kama_adaptive_period / self.period as f64).clamp(0.1, 2.0)
        } else {
            1.0
        };
        
        self.adaptation_level = match self.adaptation_mode {
            AdaptationMode::Volatility => self.volatility_factor / 3.0,
            AdaptationMode::Trend => self.trend_strength,
            AdaptationMode::Combined => {
                // ✅ Учитываем период адаптации KAMA
                (self.volatility_factor / 3.0 + self.trend_strength + period_factor / 2.0) / 3.0
            }
            AdaptationMode::MachineLearning => self.ml_adaptation_factor,
            _ => 0.5,
        }.clamp(0.0, 1.0);
    }
    
    /// Обновить ML адаптацию
    fn update_ml_adaptation(&mut self, high: f64, low: f64) {
        let volatility = high - low;
        
        // Добавляем в кластеры волатильности
        if self.cluster_filled {
            self.volatility_clusters[self.cluster_index] = volatility;
        } else {
            self.volatility_clusters.push(volatility);
        }
        
        self.cluster_index = (self.cluster_index + 1) % self.volatility_clusters.capacity();
        
        if self.volatility_clusters.len() == self.volatility_clusters.capacity() && !self.cluster_filled {
            self.cluster_filled = true;
        }
        
        // Простая кластеризация волатильности
        if self.cluster_filled {
            self.volatility_mean = self.volatility_clusters.iter().sum::<f64>() / self.volatility_clusters.len() as f64;
            
            let variance = self.volatility_clusters.iter()
                .map(|&x| (x - self.volatility_mean).powi(2))
                .sum::<f64>() / self.volatility_clusters.len() as f64;
            self.volatility_std = variance.sqrt();
            
            // ML адаптационный фактор на основе Z-score
            if self.volatility_std > 0.0 {
                let z_score = (volatility - self.volatility_mean) / self.volatility_std;
                self.ml_adaptation_factor = (1.0 + z_score.abs() / 3.0).clamp(0.5, 2.0);
                self.current_vol_cluster = z_score;
            }
        }
    }
    
    /// Рассчитать адаптивные каналы
    fn calculate_adaptive_channels(&mut self, center_line: f64, atr_value: f64) {
        if !self.is_ready() {
            return;
        }
        
        // ✅ Базовая ширина на основе ATR с учетом KAMA статистики
        let base_width = atr_value * 2.0;
        
        // ✅ Дополнительный фактор на основе дисперсии efficiency ratio KAMA
        let efficiency_variance_factor = (1.0 + self.kama.efficiency_variance()).clamp(0.5, 2.0);
        
        // Адаптивная ширина в зависимости от режима
        let adaptive_width = base_width * self.adaptation_level * efficiency_variance_factor * match self.current_regime {
            MarketRegime::Trending => 1.2, // Шире в трендах
            MarketRegime::Volatile => 1.5,  // Еще шире в волатильности
            MarketRegime::Ranging => 0.8,   // Уже в боковиках
            MarketRegime::Quiet => 0.6,     // Очень узко в спокойных условиях
            MarketRegime::Transition => 1.0, // Стандартная ширина
        };
        
        self.channel_width = adaptive_width;
        self.upper_channel = center_line + adaptive_width;
        self.lower_channel = center_line - adaptive_width;
        
        // Дополнительные уровни (2x ширина)
        self.upper_channel_2 = center_line + adaptive_width * 2.0;
        self.lower_channel_2 = center_line - adaptive_width * 2.0;
    }
    
    /// Получить основные значения
    pub fn value(&self) -> IndicatorValue {
        let center = match self.center_line_type {
            CenterLineType::AdaptiveLinReg => {
                if let Some(ref regression) = self.regression_ma {
                    regression.value().main()
                } else {
                    0.0
                }
            }
            _ => self.kama.value().main(),  // ✅ Значение готовой KAMA
        };

        IndicatorValue::Channel3 {
            upper: self.upper_channel,
            middle: center,
            lower: self.lower_channel,
        }
    }

    /// Получить основные значения как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        let center = match self.center_line_type {
            CenterLineType::AdaptiveLinReg => {
                if let Some(ref regression) = self.regression_ma {
                    regression.value().main()
                } else {
                    0.0
                }
            }
            _ => self.kama.value().main(),  // ✅ Значение готовой KAMA
        };

        (self.upper_channel, center, self.lower_channel)
    }
    
    /// Получить все уровни каналов
    pub fn all_levels(&self) -> (f64, f64, f64, f64, f64) {
        let center = match self.center_line_type {
            CenterLineType::AdaptiveLinReg => {
                if let Some(ref regression) = self.regression_ma {
                    regression.value().main()
                } else {
                    0.0
                }
            }
            _ => self.kama.value().main(),  // ✅ Значение готовой KAMA
        };
        
        (
            self.upper_channel_2,
            self.upper_channel,
            center,
            self.lower_channel,
            self.lower_channel_2,
        )
    }
    
    /// Получить адаптивную центральную линию
    pub fn adaptive_center(&self) -> f64 {
        match self.center_line_type {
            CenterLineType::AdaptiveLinReg => {
                if let Some(ref regression) = self.regression_ma {
                    regression.value().main()
                } else {
                    0.0
                }
            }
            _ => self.kama.value().main(),  // ✅ Значение готовой KAMA
        }
    }
    
    /// ✅ Получить статистику KAMA
    pub fn kama_statistics(&self) -> (f64, f64, f64, f64, f64) {
        (
            self.kama.efficiency_ratio(),
            self.kama.smoothing_constant(),
            self.kama.adaptive_period(),
            self.kama.average_efficiency(),
            self.kama.trend_consistency(),
        )
    }
    
    /// ✅ Получить тренд-сигнал от KAMA
    pub fn kama_trend_signal(&self) -> &str {
        self.kama.trend_signal().as_str()
    }
    
    /// ✅ Прогноз цены на N периодов от KAMA
    pub fn forecast(&self, periods: usize) -> Vec<f64> {
        if matches!(self.center_line_type, CenterLineType::AdaptiveLinReg) {
            // Для LinearRegression используем прогноз регрессии
            if let Some(ref regression) = self.regression_ma {
                // Используем текущее значение регрессии для всех периодов
                vec![regression.value().main(); periods]
            } else {
                vec![self.adaptive_center(); periods]
            }
        } else {
            // ✅ Используем прогноз KAMA
            self.kama.forecast(periods)
        }
    }
    
    /// Получить ширину канала
    pub fn channel_width(&self) -> f64 {
        self.channel_width
    }
    
    /// Получить уровень адаптации (0.0-1.0)
    pub fn adaptation_level(&self) -> f64 {
        self.adaptation_level
    }
    
    /// Получить рыночный режим
    pub fn market_regime(&self) -> (MarketRegime, f64) {
        (self.current_regime, self.regime_confidence)
    }
    
    /// Получить метрики адаптации
    pub fn adaptation_metrics(&self) -> (f64, f64, f64, f64) {
        (
            self.volatility_factor,
            self.trend_strength,
            self.cycle_factor,
            self.ml_adaptation_factor,
        )
    }
    
    /// Генерировать сигнал с учетом KAMA статистики
    pub fn generate_signal(&self, current_price: f64, previous_price: f64) -> AdaptiveSignal {
        if !self.is_ready() {
            return AdaptiveSignal::ReturnToAdaptiveCenter;
        }
        
        let center = self.adaptive_center();
        
        // ✅ Используем efficiency ratio для определения силы сигнала
        let efficiency_ratio = self.kama.efficiency_ratio();
        
        // Проверяем расширение/сужение каналов
        if self.adaptation_level > 0.7 || efficiency_ratio > 0.8 {
            if current_price > self.upper_channel {
                return AdaptiveSignal::HighAdaptiveBreakout;
            } else if current_price < self.lower_channel {
                return AdaptiveSignal::HighAdaptiveBreakout;
            }
        } else if (self.adaptation_level < 0.3 || efficiency_ratio < 0.2)
            && (current_price > self.upper_channel || current_price < self.lower_channel) {
                return AdaptiveSignal::LowAdaptiveBreakout;
            }
        
        // Проверяем возврат к центру
        let prev_distance = (previous_price - center).abs();
        let current_distance = (current_price - center).abs();
        
        if current_distance < prev_distance && current_distance < self.channel_width * 0.3 {
            return AdaptiveSignal::ReturnToAdaptiveCenter;
        }
        
        // Проверяем отскоки
        if (current_price <= self.upper_channel && previous_price > self.upper_channel) ||
           (current_price >= self.lower_channel && previous_price < self.lower_channel) {
            return AdaptiveSignal::AdaptiveBounce;
        }
        
        // Проверяем режимы волатильности
        match self.current_regime {
            MarketRegime::Volatile => AdaptiveSignal::HighVolatilityRegime,
            MarketRegime::Quiet => AdaptiveSignal::LowVolatilityRegime,
            _ => AdaptiveSignal::ReturnToAdaptiveCenter,
        }
    }
    
    /// Проверить изменение режима волатильности
    pub fn volatility_regime_change(&self, threshold: f64) -> Option<bool> {
        if self.volatility_factor > (1.0 + threshold) {
            Some(true) // Переход к высокой волатильности
        } else if self.volatility_factor < (1.0 - threshold) {
            Some(false) // Переход к низкой волатильности
        } else {
            None // Нет значительного изменения
        }
    }
    
    /// Получить позицию в адаптивном канале
    pub fn position_in_adaptive_channel(&self, price: f64) -> f64 {
        if self.channel_width > 0.0 {
            (price - self.lower_channel) / (self.upper_channel - self.lower_channel)
        } else {
            0.5
        }
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        match self.center_line_type {
            CenterLineType::AdaptiveLinReg => {
                self.regression_ma.as_ref().is_some_and(|r| r.is_ready()) && 
                self.adaptive_atr.is_ready()
            }
            _ => self.kama.is_ready() && self.adaptive_atr.is_ready(),  // ✅ Готовность KAMA
        }
    }
    
    /// Получить параметры
    pub fn get_params(&self) -> (usize, AdaptationMode, CenterLineType, usize) {
        (self.period, self.adaptation_mode, self.center_line_type, self.volatility_lookback)
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.kama.reset();  // ✅ Сброс готовой KAMA
        self.adaptive_atr.reset();
        
        if let Some(ref mut regression) = self.regression_ma {
            regression.reset();
        }
        
        self.price_buffer.clear();
        self.volatility_clusters.clear();
        self.cluster_index = 0;
        self.cluster_filled = false;
        
        self.volatility_factor = 1.0;
        self.trend_strength = 0.0;
        self.cycle_factor = 1.0;
        self.ml_adaptation_factor = 1.0;
        
        self.upper_channel = 0.0;
        self.lower_channel = 0.0;
        self.channel_width = 0.0;
        self.upper_channel_2 = 0.0;
        self.lower_channel_2 = 0.0;
        
        self.current_regime = MarketRegime::Transition;
        self.regime_confidence = 0.0;
        self.adaptation_level = 0.5;
        self.volatility_percentile = 0.5;
        
        self.dominant_cycle = 0.0;
        self.cycle_strength = 0.0;
        self.volatility_mean = 0.0;
        self.volatility_std = 0.0;
        self.current_vol_cluster = 0.0;
        
        self.buffer_index = 0;
        self.buffer_filled = false;
        self.bar_count = 0;
    }
}

impl Default for AdaptiveChannels {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_channels_creation() {
        let ac = AdaptiveChannels::new();
        assert!(!ac.is_ready());
    }

    #[test]
    fn test_adaptive_channels_warmup() {
        let mut ac = AdaptiveChannels::new();
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ac.is_ready());
    }

    #[test]
    fn test_adaptive_channels_values() {
        let mut ac = AdaptiveChannels::new();
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, _middle, lower) = ac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if ac.is_ready() {
                assert!(upper > lower, "Upper should be > lower");
            }
        }
    }

    #[test]
    fn test_adaptive_channels_reset() {
        let mut ac = AdaptiveChannels::new();
        for i in 0..50 {
            ac.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ac.reset();
        assert!(!ac.is_ready());
    }
} 






















