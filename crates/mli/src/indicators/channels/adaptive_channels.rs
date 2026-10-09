//! adaptive_channels.rs: High-Performance Adaptive Channels
//! Адаптивные каналы - автоматическая адаптация к рыночным условиям
//! 
//! Особенности:
//! - Использует готовые компоненты: KaufmanAdaptiveMA, LinearRegressionMA, Atr
//! - Adaptive ATR для динамической ширины каналов
//! - Machine Learning inspired volatility clustering detection
//! - Автоматическая адаптация к рыночным режимам

use crate::indicators::average::kaufman_adaptive_ma::KaufmanAdaptiveMA;
use crate::indicators::average::lr::LinearRegressionMA;
use crate::indicators::volatility::atr::Atr;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{
    Cost, Port, Family, Indicator, Output, SourceAxis, Store, StoreKind,
    UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;

/// Режимы адаптации каналов
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(mli_contract_macros::ParamScalar)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(mli_contract_macros::ParamScalar)]
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
    price_buffer: Vec<f64>,
    volatility_clusters: Vec<f64>,
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
        Self::new_custom_with_atr(
            period,
            adaptation_mode,
            center_line_type,
            volatility_lookback,
            Atr::new_wilder(period),
        )
    }

    /// Создать адаптивные каналы с кастомными параметрами + готовым внутренним ATR.
    ///
    /// `adaptive_atr` — внутренний width-ATR; его смузер + период задаёт вызывающий.
    /// Контракт-путь строит его из demand-fill order (ALMA shape сохраняется); legacy —
    /// period-only.
    pub fn new_custom_with_atr(
        period: usize,
        adaptation_mode: AdaptationMode,
        center_line_type: CenterLineType,
        volatility_lookback: usize,
        adaptive_atr: Atr,
    ) -> Self {
        assert!(period > 0, "Period must be > 0");
        assert!(volatility_lookback > 0, "Volatility lookback must be > 0");

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
            adaptive_atr,
            regression_ma,
            price_buffer: Vec::with_capacity(period),
            volatility_clusters: Vec::with_capacity(100),
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
    
    /// Feed the resolved `[high, low, close]` lanes (in `SOURCE` order). The channel is
    /// built on KAMA(close) for the centre, ATR(h/l/c) for width, and range-based (h-l)
    /// adaptation; open/volume are unused.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        self.bar_count += 1;

        // Обновляем буферы
        self.update_buffers(close);

        // ✅ Обновляем центральную линию через готовые компоненты
        let center_line = self.update_center_line(close);
        
        // ✅ Обновляем adaptive ATR через готовый компонент
        let atr_value = self.adaptive_atr.feed(&[high, low, close]);
        
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
    fn update_buffers(&mut self, close: f64) {
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
    fn update_center_line(&mut self, close: f64) -> f64 {
        match self.center_line_type {
            CenterLineType::AdaptiveLinReg => {
                // Используем LinearRegressionMA
                if let Some(ref mut regression) = self.regression_ma {
                    regression.feed(close)
                } else {
                    close
                }
            }
            _ => {
                // ✅ Используем готовую мощную KAMA!
                self.kama.feed(close)
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
        let avg_range = self.adaptive_atr.value();
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
        let avg_range = self.adaptive_atr.value();
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
    

    /// Получить основные значения как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        let center = match self.center_line_type {
            CenterLineType::AdaptiveLinReg => {
                if let Some(ref regression) = self.regression_ma {
                    regression.line()
                } else {
                    0.0
                }
            }
            _ => self.kama.line(),  // ✅ Значение готовой KAMA
        };

        (self.upper_channel, center, self.lower_channel)
    }
    
    /// Получить все уровни каналов
    pub fn all_levels(&self) -> (f64, f64, f64, f64, f64) {
        let center = match self.center_line_type {
            CenterLineType::AdaptiveLinReg => {
                if let Some(ref regression) = self.regression_ma {
                    regression.line()
                } else {
                    0.0
                }
            }
            _ => self.kama.line(),  // ✅ Значение готовой KAMA
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
                    regression.line()
                } else {
                    0.0
                }
            }
            _ => self.kama.line(),  // ✅ Значение готовой KAMA
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
                vec![regression.line(); periods]
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

    pub fn upper(&self) -> f64 { self.upper_channel }
    pub fn middle(&self) -> f64 { self.adaptive_center() }
    pub fn lower(&self) -> f64 { self.lower_channel }

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
    fn test_new_custom_with_atr_non_default() {
        let mut ac = AdaptiveChannels::new_custom_with_atr(
            20,
            AdaptationMode::Volatility,
            CenterLineType::KAMA,
            40,
            Atr::new_ema(20),
        );
        assert!(!ac.is_ready());
        for i in 0..50 {
            let p = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            let (u, _m, l) = ac.feed(&[p + 1.0, p - 1.0, p]);
            assert!(u.is_finite());
            assert!(l.is_finite());
        }
        assert!(ac.is_ready());
    }

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
            ac.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ac.is_ready());
    }

    #[test]
    fn test_adaptive_channels_values() {
        let mut ac = AdaptiveChannels::new();
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, _middle, lower) = ac.feed(&[price + 1.0, price - 1.0, price]);
            if ac.is_ready() {
                assert!(upper > lower, "Upper should be > lower");
            }
        }
    }

    #[test]
    fn test_adaptive_channels_reset() {
        let mut ac = AdaptiveChannels::new();
        for i in 0..50 {
            ac.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        ac.reset();
        assert!(!ac.is_ready());
    }
}

use crate::engine::contract_engine::SmootherSlotOrder;
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::Param;

/// Typed contract config for [`AdaptiveChannels`].
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct AdaptiveChanConfig {
    pub period: Param<usize>,
    pub adaptation_mode: Param<AdaptationMode>,
    pub center_line_type: Param<CenterLineType>,
    pub volatility_lookback: Param<usize>,
    /// The width-ATR's smoother slot (member + own period). Default Rma (Wilder) at the
    /// host period (20).
    #[slot]
    pub atr_smoother: Param<SmootherSlotOrder>,
}

impl Indicator for AdaptiveChannels {
    const ID: IndicatorId = IndicatorId::Adaptivechan;
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to the raw range slices — the channel is built on True Range (h/l/c).
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// A channel NODE. Own state = price / volatility-cluster buffers + ML adaptation
    /// pass. TWO fixed edges: the centre `Kama` (its rich analytics surface drives
    /// adaptation) and the width `Atr` (its `value`). ATR is a REAL node carrying its
    /// OWN MovingAverage smoother slot — the recursion charges ATR's TR base + its
    /// slot member THROUGH this edge (Model B: channel → ATR → MA), instead of the
    /// channel re-declaring ATR's smoothing as its own flat slot (the old Model A).
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[
            Store::window(StoreKind::Vec),     // price_buffer (period deep)
            Store::fixed(StoreKind::Vec, 100), // volatility_clusters
        ],
        inner: &[
            Port::new(
                IndicatorId::Kama,
                &[
                    IndicatorOutputId::KamaLine,
                    IndicatorOutputId::KamaEfficiencyRatio,
                    IndicatorOutputId::KamaAdaptivePeriod,
                    IndicatorOutputId::KamaEfficiencyVariance,
                    IndicatorOutputId::KamaTrendConsistency,
                ],
            ),
            Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr]),
        ],
    };
    /// No own slots: the width smoother belongs to the ATR node (reached via the
    /// `Atr` edge), not the channel. The channel's `atr_ma` order configures THAT
    /// nested ATR's slot — it flows through the edge into ATR's slot when the order
    /// nests the ATR config (`inner_for`).
    const SLOTS: &'static [crate::contract::Slot] = &[];
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::AdaptivechanUpper),
        Output::price(IndicatorOutputId::AdaptivechanMiddle),
        Output::price(IndicatorOutputId::AdaptivechanLower),
        Output::magnitude(IndicatorOutputId::AdaptivechanChannelWidth),
        Output::percent(IndicatorOutputId::AdaptivechanAdaptationLevel),
    ];
    type Config = AdaptiveChanConfig;
    type Runtime = AdaptiveChannels;

    fn create(cfg: AdaptiveChanConfig) -> AdaptiveChannels {
        let period = cfg.period.resolved();
        let atr_order = cfg.atr_smoother.resolved();
        let adaptive_atr = Atr::from_smoother(atr_order.period(), atr_order.id());
        AdaptiveChannels::new_custom_with_atr(
            period,
            cfg.adaptation_mode.resolved(),
            cfg.center_line_type.resolved(),
            cfg.volatility_lookback.resolved(),
            adaptive_atr,
        )
    }
}

impl crate::contract::Config for AdaptiveChanConfig {
    /// Period 20 (granularity default), Combined adaptation, KAMA centre, 50-bar volatility
    /// lookback, RMA-smoothed ATR at the host period (20).
    fn defaults() -> Self {
        AdaptiveChanConfig {
            period: Param::Solo(20),
            adaptation_mode: Param::Solo(AdaptationMode::Combined),
            center_line_type: Param::Solo(CenterLineType::KAMA),
            volatility_lookback: Param::Solo(50),
            atr_smoother: Param::Solo(SmootherSlotOrder::Rma(PeriodConfig { period: 20 })),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1), volatility_lookback→range(2,4048,1)
        s.adaptation_mode = Param::many(vec![
            AdaptationMode::Volatility,
            AdaptationMode::Trend,
            AdaptationMode::Cycle,
            AdaptationMode::Combined,
            AdaptationMode::MachineLearning,
        ]); // Class Q enum — all variants
        s.center_line_type = Param::many(vec![
            CenterLineType::KAMA,
            CenterLineType::FastKAMA,
            CenterLineType::SlowKAMA,
            CenterLineType::AdaptiveLinReg,
        ]); // Class Q enum — all variants
        s
    }
}


impl Render for AdaptiveChannels {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::AdaptivechanUpper, "Upper", Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::AdaptivechanMiddle, "Middle", Color::hex(0x9E9E9E), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::AdaptivechanLower, "Lower", Color::hex(0x4CAF50), 1.0))
            .precision(4)
            .build()
    }
}






















