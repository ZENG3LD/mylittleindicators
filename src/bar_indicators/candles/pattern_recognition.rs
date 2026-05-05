//! Advanced Pattern Recognition - Enhanced candlestick pattern detection
//!
//! This module provides sophisticated pattern recognition using:
//! - Configurable pattern parameters for flexibility
//! - Multi-timeframe pattern validation
//! - Single, double, and multi-candle patterns
//! - Pattern strength scoring and confidence analysis

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use arrayvec::ArrayVec;
use serde::{Serialize, Deserialize};

/// Конфигурация для гибкой настройки паттернов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternConfig {
    // Общие параметры
    pub min_candle_size_pct: f64,          // Минимальный размер свечи в %
    pub atr_normalization: bool,           // Использовать ATR для нормализации
    
    // Параметры для Doji
    pub doji_body_ratio: f64,              // Максимальное отношение тела к диапазону (0.1 = 10%)
    
    // Параметры для Hammer/Shooting Star
    pub hammer_shadow_ratio: f64,          // Минимальное отношение тени к телу (2.0 = в 2 раза больше)
    pub hammer_opposite_shadow_ratio: f64, // Максимальное отношение противоположной тени к телу (0.5 = половина)
    pub hammer_body_position: f64,         // Позиция тела в диапазоне (0.6 = верхние 40%)
    
    // Параметры для Marubozu
    pub marubozu_body_ratio: f64,          // Минимальное отношение тела к диапазону (0.95 = 95%)
    
    // Параметры для Engulfing
    pub engulfing_min_size_ratio: f64,     // Минимальное отношение размеров тел (1.2 = на 20% больше)
    
    // Параметры для Star паттернов
    pub star_gap_ratio: f64,               // Минимальный гэп для Star паттернов (0.1 = 10% от ATR)
    
    // Параметры для Three White Soldiers/Black Crows
    pub soldiers_min_body_ratio: f64,      // Минимальное отношение тела к диапазону (0.6 = 60%)
    pub soldiers_progression_ratio: f64,   // Минимальное увеличение каждой свечи (1.05 = 5%)
}

impl Default for PatternConfig {
    fn default() -> Self {
        Self {
            min_candle_size_pct: 0.5,
            atr_normalization: true,
            doji_body_ratio: 0.1,
            hammer_shadow_ratio: 2.0,
            hammer_opposite_shadow_ratio: 0.5,
            hammer_body_position: 0.6,
            marubozu_body_ratio: 0.95,
            engulfing_min_size_ratio: 1.2,
            star_gap_ratio: 0.1,
            soldiers_min_body_ratio: 0.6,
            soldiers_progression_ratio: 1.05,
        }
    }
}

/// Тип распознанного паттерна
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PatternType {
    // Одиночные паттерны
    Hammer,
    InvertedHammer,
    ShootingStar,
    HangingMan,
    Doji,
    GravestoneDoji,
    DragonflyDoji,
    LongLeggedDoji,
    Marubozu,
    WhiteMarubozu,
    BlackMarubozu,
    SpinningTop,
    
    // Двойные паттерны
    BullishEngulfing,
    BearishEngulfing,
    BullishHarami,
    BearishHarami,
    PiercingPattern,
    DarkCloudCover,
    TweezerTop,
    TweezerBottom,
    
    // Тройные паттерны
    MorningStar,
    EveningStar,
    MorningDojiStar,
    EveningDojiStar,
    ThreeWhiteSoldiers,
    ThreeBlackCrows,
    ThreeInsideUp,
    ThreeInsideDown,
    ThreeOutsideUp,
    ThreeOutsideDown,
    
    // Продолжение
    RisingThreeMethods,
    FallingThreeMethods,
    UpsideGapTwoCrows,
    DownsideGapThreeMethods,
    
    Unknown,
}

impl PatternType {
    pub fn is_bullish(&self) -> bool {
        matches!(self, 
            PatternType::Hammer | PatternType::InvertedHammer | PatternType::DragonflyDoji |
            PatternType::WhiteMarubozu | PatternType::BullishEngulfing | PatternType::BullishHarami |
            PatternType::PiercingPattern | PatternType::TweezerBottom | PatternType::MorningStar |
            PatternType::MorningDojiStar | PatternType::ThreeWhiteSoldiers | 
            PatternType::ThreeInsideUp | PatternType::ThreeOutsideUp | PatternType::RisingThreeMethods
        )
    }
    
    pub fn is_bearish(&self) -> bool {
        matches!(self, 
            PatternType::ShootingStar | PatternType::HangingMan | PatternType::GravestoneDoji |
            PatternType::BlackMarubozu | PatternType::BearishEngulfing | PatternType::BearishHarami |
            PatternType::DarkCloudCover | PatternType::TweezerTop | PatternType::EveningStar |
            PatternType::EveningDojiStar | PatternType::ThreeBlackCrows | 
            PatternType::ThreeInsideDown | PatternType::ThreeOutsideDown | PatternType::FallingThreeMethods
        )
    }
    
    pub fn candle_count(&self) -> usize {
        match self {
            // Одиночные
            PatternType::Hammer | PatternType::InvertedHammer | PatternType::ShootingStar |
            PatternType::HangingMan | PatternType::Doji | PatternType::GravestoneDoji |
            PatternType::DragonflyDoji | PatternType::LongLeggedDoji | PatternType::Marubozu |
            PatternType::WhiteMarubozu | PatternType::BlackMarubozu | PatternType::SpinningTop => 1,
            
            // Двойные
            PatternType::BullishEngulfing | PatternType::BearishEngulfing | PatternType::BullishHarami |
            PatternType::BearishHarami | PatternType::PiercingPattern | PatternType::DarkCloudCover |
            PatternType::TweezerTop | PatternType::TweezerBottom => 2,
            
            // Тройные
            PatternType::MorningStar | PatternType::EveningStar | PatternType::MorningDojiStar |
            PatternType::EveningDojiStar | PatternType::ThreeWhiteSoldiers | PatternType::ThreeBlackCrows |
            PatternType::ThreeInsideUp | PatternType::ThreeInsideDown | PatternType::ThreeOutsideUp |
            PatternType::ThreeOutsideDown | PatternType::UpsideGapTwoCrows => 3,
            
            // Пятисвечные
            PatternType::RisingThreeMethods | PatternType::FallingThreeMethods | 
            PatternType::DownsideGapThreeMethods => 5,
            
            PatternType::Unknown => 0,
        }
    }
}

/// Результат распознавания паттерна
#[derive(Debug, Clone, Copy)]
pub struct PatternResult {
    pub pattern_type: PatternType,
    pub confidence: f64,        // Уверенность в паттерне (0.0-1.0)
    pub strength: f64,          // Сила паттерна (0.0-1.0)
    pub bullish_probability: f64, // Вероятность бычьего движения
    pub bearish_probability: f64, // Вероятность медвежьего движения
    pub reliability_score: f64,   // Историческая надежность паттерна (0.0-1.0)
}

impl Default for PatternResult {
    fn default() -> Self {
        Self {
            pattern_type: PatternType::Unknown,
            confidence: 0.0,
            strength: 0.0,
            bullish_probability: 0.5,
            bearish_probability: 0.5,
            reliability_score: 0.5,
        }
    }
}

/// Продвинутый распознаватель паттернов
#[derive(Debug)]
pub struct AdvancedPatternRecognition {
    // Конфигурация
    config: PatternConfig,
    
    // История баров для анализа
    ohlc_history: ArrayVec<[f64; 4], 10>, // [open, high, low, close]
    
    // Вспомогательные индикаторы
    atr_ma: MovingAverageProvider,      // Для нормализации
    volume_ma: MovingAverageProvider,   // Анализ объема
    
    // Результат
    current_result: PatternResult,
    
    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl Clone for AdvancedPatternRecognition {
    fn clone(&self) -> Self {
        Self::new_with_config(self.config.clone())
    }
}

impl Default for AdvancedPatternRecognition {
    fn default() -> Self {
        Self::new()
    }
}

impl AdvancedPatternRecognition {
    pub fn new() -> Self {
        Self::new_with_config(PatternConfig::default())
    }
    
    pub fn new_with_config(config: PatternConfig) -> Self {
        Self {
            config,
            ohlc_history: ArrayVec::new(),
            atr_ma: MovingAverageProvider::new(MovingAverageType::EMA, 14),
            volume_ma: MovingAverageProvider::new(MovingAverageType::SMA, 20),
            current_result: PatternResult::default(),
            is_ready: false,
            update_count: 0,
        }
    }
    
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> PatternResult {
        // Проверяем минимальный размер свечи
        if self.config.min_candle_size_pct > 0.0 {
            let candle_size_pct = ((close - open).abs() / open) * 100.0;
            if candle_size_pct < self.config.min_candle_size_pct {
                self.current_result = PatternResult::default();
                return self.current_result;
            }
        }
        
        // Сохраняем OHLC
        if self.ohlc_history.len() >= 10 {
            self.ohlc_history.remove(0);
        }
        self.ohlc_history.push([open, high, low, close]);
        
        // Обновляем вспомогательные индикаторы
        let range = high - low;
        self.atr_ma.update_bar(0.0, 0.0, 0.0, range, 0.0);
        self.volume_ma.update_bar(0.0, 0.0, 0.0, volume, 0.0);
        
        // Анализируем паттерны
        if !self.ohlc_history.is_empty() {
            self.analyze_patterns();
            self.is_ready = true;
        }
        
        self.update_count += 1;
        self.current_result
    }
    
    fn analyze_patterns(&mut self) {
        let len = self.ohlc_history.len();
        
        // Пытаемся найти паттерны в порядке приоритета (сложные сначала)
        
        // 5-свечные паттерны
        if len >= 5 {
            if let Some(result) = self.analyze_five_candle_patterns() {
                self.current_result = result;
                return;
            }
        }
        
        // 3-свечные паттерны
        if len >= 3 {
            if let Some(result) = self.analyze_three_candle_patterns() {
                self.current_result = result;
                return;
            }
        }
        
        // 2-свечные паттерны
        if len >= 2 {
            if let Some(result) = self.analyze_two_candle_patterns() {
                self.current_result = result;
                return;
            }
        }
        
        // Одиночные паттерны
        if let Some(result) = self.analyze_single_candle_patterns() {
            self.current_result = result;
            return;
        }
        
        self.current_result = PatternResult::default();
    }
    
    #[allow(dead_code)]
    fn analyze_single_candle(&self, open: f64, high: f64, low: f64, close: f64) -> PatternResult {
        let body_size = (close - open).abs();
        let total_range = high - low;
        let upper_shadow = high - open.max(close);
        let lower_shadow = open.min(close) - low;
        
        // Нормализация относительно ATR
        let atr = self.atr_ma.value().main();
        let _normalized_body = if atr > 0.0 { body_size / atr } else { 0.0 };
        let _normalized_range = if atr > 0.0 { total_range / atr } else { 0.0 };
        
        // Определяем тип паттерна
        if self.is_doji(body_size, total_range) {
            PatternResult {
                pattern_type: PatternType::Doji,
                confidence: 0.8,
                strength: 0.6,
                bullish_probability: 0.5,
                bearish_probability: 0.5,
                reliability_score: 0.7, // Добавляем надежность
            }
        } else if self.is_hammer(open, high, low, close, body_size, lower_shadow, upper_shadow) {
            PatternResult {
                pattern_type: PatternType::Hammer,
                confidence: 0.75,
                strength: 0.7,
                bullish_probability: 0.7,
                bearish_probability: 0.3,
                reliability_score: 0.8, // Добавляем надежность
            }
        } else if self.is_shooting_star(open, high, low, close, body_size, upper_shadow, lower_shadow) {
            PatternResult {
                pattern_type: PatternType::ShootingStar,
                confidence: 0.75,
                strength: 0.7,
                bullish_probability: 0.3,
                bearish_probability: 0.7,
                reliability_score: 0.8, // Добавляем надежность
            }
        } else if self.is_marubozu(body_size, total_range) {
            PatternResult {
                pattern_type: PatternType::Marubozu,
                confidence: 0.8,
                strength: 0.8,
                bullish_probability: if close > open { 0.8 } else { 0.2 },
                bearish_probability: if close > open { 0.2 } else { 0.8 },
                reliability_score: 0.9, // Добавляем надежность
            }
        } else {
            PatternResult {
                pattern_type: PatternType::Unknown,
                confidence: 0.0,
                strength: 0.0,
                bullish_probability: 0.5,
                bearish_probability: 0.5,
                reliability_score: 0.0, // Добавляем надежность
            }
        }
    }
    
    #[allow(dead_code)]
    fn analyze_multi_candle_patterns(&self) -> PatternResult {
        let len = self.ohlc_history.len();
        if len < 2 {
            return self.current_result;
        }
        
        let current = self.ohlc_history[len - 1];
        let previous = self.ohlc_history[len - 2];
        
        // Проверяем Engulfing
        if self.is_engulfing(previous, current) {
            return PatternResult {
                pattern_type: PatternType::BullishEngulfing,
                confidence: 0.85,
                strength: 0.8,
                bullish_probability: if current[3] > current[0] { 0.8 } else { 0.2 },
                bearish_probability: if current[3] > current[0] { 0.2 } else { 0.8 },
                reliability_score: 0.9, // Добавляем надежность
            };
        }
        
        // Проверяем Harami
        if self.is_harami(previous, current) {
            return PatternResult {
                pattern_type: PatternType::BullishHarami,
                confidence: 0.7,
                strength: 0.6,
                bullish_probability: 0.6,
                bearish_probability: 0.4,
                reliability_score: 0.8, // Добавляем надежность
            };
        }
        
        self.current_result
    }
    
    // Паттерн-детекторы
    fn is_doji(&self, body_size: f64, total_range: f64) -> bool {
        total_range > 0.0 && (body_size / total_range) < self.config.doji_body_ratio
    }
    
    #[allow(dead_code)]
    fn is_hammer(&self, _open: f64, high: f64, low: f64, _close: f64,
                 body_size: f64, lower_shadow: f64, upper_shadow: f64) -> bool {
        let total_range = high - low;
        if total_range == 0.0 { return false; }
        
        lower_shadow > body_size * self.config.hammer_shadow_ratio && 
        upper_shadow < body_size * self.config.hammer_opposite_shadow_ratio &&
        (body_size / total_range) > 0.1
    }
    
    #[allow(dead_code)]
    fn is_shooting_star(&self, _open: f64, high: f64, low: f64, _close: f64,
                       body_size: f64, upper_shadow: f64, lower_shadow: f64) -> bool {
        let total_range = high - low;
        if total_range == 0.0 { return false; }
        
        upper_shadow > body_size * self.config.hammer_shadow_ratio && 
        lower_shadow < body_size * self.config.hammer_opposite_shadow_ratio &&
        (body_size / total_range) > 0.1
    }
    
    fn is_marubozu(&self, body_size: f64, total_range: f64) -> bool {
        total_range > 0.0 && (body_size / total_range) > self.config.marubozu_body_ratio
    }
    
    #[allow(dead_code)]
    fn is_engulfing(&self, prev: [f64; 4], curr: [f64; 4]) -> bool {
        let prev_body_size = (prev[3] - prev[0]).abs();
        let curr_body_size = (curr[3] - curr[0]).abs();
        
        // Текущее тело больше предыдущего
        curr_body_size > prev_body_size &&
        // Направления противоположные
        (prev[3] > prev[0]) != (curr[3] > curr[0]) &&
        // Текущее тело полностью поглощает предыдущее
        curr[0].min(curr[3]) < prev[0].min(prev[3]) &&
        curr[0].max(curr[3]) > prev[0].max(prev[3])
    }
    
    #[allow(dead_code)]
    fn is_harami(&self, prev: [f64; 4], curr: [f64; 4]) -> bool {
        let prev_body_size = (prev[3] - prev[0]).abs();
        let curr_body_size = (curr[3] - curr[0]).abs();
        
        // Предыдущее тело больше текущего
        prev_body_size > curr_body_size &&
        // Текущее тело внутри предыдущего
        curr[0].min(curr[3]) > prev[0].min(prev[3]) &&
        curr[0].max(curr[3]) < prev[0].max(prev[3])
    }
    
    pub fn result(&self) -> PatternResult {
        self.current_result
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    pub fn reset(&mut self) {
        self.ohlc_history.clear();
        self.atr_ma.reset();
        self.volume_ma.reset();
        self.current_result = PatternResult::default();
        self.is_ready = false;
        self.update_count = 0;
    }

    pub fn value(&self) -> IndicatorValue {
        let pattern = &self.current_result;
        if pattern.pattern_type == PatternType::Unknown {
            IndicatorValue::Signal(0)
        } else if pattern.pattern_type.is_bullish() {
            IndicatorValue::Signal(1)
        } else if pattern.pattern_type.is_bearish() {
            IndicatorValue::Signal(-1)
        } else {
            IndicatorValue::Signal(0)
        }
    }

    // Новые методы для анализа различных типов паттернов
    
    fn analyze_single_candle_patterns(&self) -> Option<PatternResult> {
        let len = self.ohlc_history.len();
        if len < 1 { return None; }
        
        let current = self.ohlc_history[len - 1];
        let [open, high, low, close] = current;
        
        let body_size = (close - open).abs();
        let total_range = high - low;
        let lower_shadow = open.min(close) - low;
        let upper_shadow = high - open.max(close);
        
        if total_range == 0.0 { return None; }
        
        // Анализируем различные одиночные паттерны
        
        // Doji variants
        if self.is_doji(body_size, total_range) {
            let pattern_type = if upper_shadow > 3.0 * body_size && lower_shadow < body_size {
                PatternType::GravestoneDoji
            } else if lower_shadow > 3.0 * body_size && upper_shadow < body_size {
                PatternType::DragonflyDoji
            } else if upper_shadow > 2.0 * body_size && lower_shadow > 2.0 * body_size {
                PatternType::LongLeggedDoji
            } else {
                PatternType::Doji
            };
            
            return Some(PatternResult {
                pattern_type,
                confidence: 0.8,
                strength: 0.7,
                bullish_probability: match pattern_type {
                    PatternType::DragonflyDoji => 0.7,
                    PatternType::GravestoneDoji => 0.3,
                    _ => 0.5,
                },
                bearish_probability: match pattern_type {
                    PatternType::DragonflyDoji => 0.3,
                    PatternType::GravestoneDoji => 0.7,
                    _ => 0.5,
                },
                reliability_score: 0.75,
            });
        }
        
        // Hammer/Hanging Man
        if self.is_hammer_shape(open, high, low, close, body_size, lower_shadow, upper_shadow) {
            let pattern_type = if close > open {
                PatternType::Hammer
            } else {
                PatternType::HangingMan
            };
            
            return Some(PatternResult {
                pattern_type,
                confidence: 0.85,
                strength: 0.8,
                bullish_probability: if pattern_type == PatternType::Hammer { 0.75 } else { 0.25 },
                bearish_probability: if pattern_type == PatternType::Hammer { 0.25 } else { 0.75 },
                reliability_score: 0.8,
            });
        }
        
        // Inverted Hammer/Shooting Star
        if self.is_inverted_hammer_shape(open, high, low, close, body_size, upper_shadow, lower_shadow) {
            let pattern_type = if close > open {
                PatternType::InvertedHammer
            } else {
                PatternType::ShootingStar
            };
            
            return Some(PatternResult {
                pattern_type,
                confidence: 0.85,
                strength: 0.8,
                bullish_probability: if pattern_type == PatternType::InvertedHammer { 0.65 } else { 0.25 },
                bearish_probability: if pattern_type == PatternType::InvertedHammer { 0.35 } else { 0.75 },
                reliability_score: 0.8,
            });
        }
        
        // Marubozu
        if self.is_marubozu(body_size, total_range) {
            let pattern_type = if close > open {
                PatternType::WhiteMarubozu
            } else {
                PatternType::BlackMarubozu
            };
            
            return Some(PatternResult {
                pattern_type,
                confidence: 0.9,
                strength: 0.9,
                bullish_probability: if pattern_type == PatternType::WhiteMarubozu { 0.85 } else { 0.15 },
                bearish_probability: if pattern_type == PatternType::WhiteMarubozu { 0.15 } else { 0.85 },
                reliability_score: 0.9,
            });
        }
        
        // Spinning Top
        if self.is_spinning_top(body_size, total_range, upper_shadow, lower_shadow) {
            return Some(PatternResult {
                pattern_type: PatternType::SpinningTop,
                confidence: 0.7,
                strength: 0.5,
                bullish_probability: 0.5,
                bearish_probability: 0.5,
                reliability_score: 0.6,
            });
        }
        
        None
    }
    
    fn analyze_two_candle_patterns(&self) -> Option<PatternResult> {
        let len = self.ohlc_history.len();
        if len < 2 { return None; }
        
        let prev = self.ohlc_history[len - 2];
        let curr = self.ohlc_history[len - 1];
        
        // Engulfing patterns
        if let Some(result) = self.check_engulfing_patterns(prev, curr) {
            return Some(result);
        }
        
        // Harami patterns
        if let Some(result) = self.check_harami_patterns(prev, curr) {
            return Some(result);
        }
        
        // Piercing Pattern / Dark Cloud Cover
        if let Some(result) = self.check_piercing_dark_cloud(prev, curr) {
            return Some(result);
        }
        
        // Tweezer patterns
        if let Some(result) = self.check_tweezer_patterns(prev, curr) {
            return Some(result);
        }
        
        None
    }
    
    fn analyze_three_candle_patterns(&self) -> Option<PatternResult> {
        let len = self.ohlc_history.len();
        if len < 3 { return None; }
        
        let first = self.ohlc_history[len - 3];
        let second = self.ohlc_history[len - 2];
        let third = self.ohlc_history[len - 1];
        
        // Morning/Evening Star patterns
        if let Some(result) = self.check_star_patterns(first, second, third) {
            return Some(result);
        }
        
        // Three White Soldiers / Three Black Crows
        if let Some(result) = self.check_three_soldiers_crows(first, second, third) {
            return Some(result);
        }
        
        // Three Inside Up/Down
        if let Some(result) = self.check_three_inside_patterns(first, second, third) {
            return Some(result);
        }
        
        // Three Outside Up/Down
        if let Some(result) = self.check_three_outside_patterns(first, second, third) {
            return Some(result);
        }
        
        None
    }
    
    fn analyze_five_candle_patterns(&self) -> Option<PatternResult> {
        let len = self.ohlc_history.len();
        if len < 5 { return None; }
        
        let candles: Vec<[f64; 4]> = self.ohlc_history.iter().rev().take(5).cloned().collect();
        
        // Rising/Falling Three Methods
        if let Some(result) = self.check_three_methods_patterns(&candles) {
            return Some(result);
        }
        
        None
    }
    
    // Helper methods for pattern detection
    
    fn is_hammer_shape(&self, open: f64, high: f64, low: f64, close: f64, 
                      body_size: f64, lower_shadow: f64, upper_shadow: f64) -> bool {
        let total_range = high - low;
        if total_range == 0.0 { return false; }
        
        lower_shadow >= body_size * self.config.hammer_shadow_ratio && 
        upper_shadow <= body_size * self.config.hammer_opposite_shadow_ratio &&
        (close.min(open) - low) / total_range >= self.config.hammer_body_position
    }
    
    fn is_inverted_hammer_shape(&self, open: f64, high: f64, low: f64, close: f64,
                               body_size: f64, upper_shadow: f64, lower_shadow: f64) -> bool {
        let total_range = high - low;
        if total_range == 0.0 { return false; }
        
        upper_shadow >= body_size * self.config.hammer_shadow_ratio && 
        lower_shadow <= body_size * self.config.hammer_opposite_shadow_ratio &&
        (high - close.max(open)) / total_range >= self.config.hammer_body_position
    }
    
    fn is_spinning_top(&self, body_size: f64, total_range: f64, upper_shadow: f64, lower_shadow: f64) -> bool {
        if total_range == 0.0 { return false; }
        
        let body_ratio = body_size / total_range;
        let shadow_ratio = (upper_shadow + lower_shadow) / total_range;
        
        body_ratio < 0.3 && shadow_ratio > 0.6 && 
        upper_shadow > body_size && lower_shadow > body_size
    }
    
    fn check_engulfing_patterns(&self, prev: [f64; 4], curr: [f64; 4]) -> Option<PatternResult> {
        let [prev_o, _prev_h, _prev_l, prev_c] = prev;
        let [curr_o, _curr_h, _curr_l, curr_c] = curr;
        
        let prev_body = (prev_c - prev_o).abs();
        let curr_body = (curr_c - curr_o).abs();
        
        // Проверяем размер поглощения
        if curr_body < prev_body * self.config.engulfing_min_size_ratio {
            return None;
        }
        
        // Bullish Engulfing
        if prev_c < prev_o && curr_c > curr_o && 
           curr_o <= prev_c && curr_c >= prev_o {
            return Some(PatternResult {
                pattern_type: PatternType::BullishEngulfing,
                confidence: 0.9,
                strength: 0.85,
                bullish_probability: 0.8,
                bearish_probability: 0.2,
                reliability_score: 0.85,
            });
        }
        
        // Bearish Engulfing
        if prev_c > prev_o && curr_c < curr_o && 
           curr_o >= prev_c && curr_c <= prev_o {
            return Some(PatternResult {
                pattern_type: PatternType::BearishEngulfing,
                confidence: 0.9,
                strength: 0.85,
                bullish_probability: 0.2,
                bearish_probability: 0.8,
                reliability_score: 0.85,
            });
        }
        
        None
    }
    
    fn check_harami_patterns(&self, prev: [f64; 4], curr: [f64; 4]) -> Option<PatternResult> {
        let [prev_o, _prev_h, _prev_l, prev_c] = prev;
        let [curr_o, _curr_h, _curr_l, curr_c] = curr;
        
        let prev_body_top = prev_o.max(prev_c);
        let prev_body_bottom = prev_o.min(prev_c);
        let curr_body_top = curr_o.max(curr_c);
        let curr_body_bottom = curr_o.min(curr_c);
        
        // Текущее тело должно быть внутри предыдущего
        if curr_body_top <= prev_body_top && curr_body_bottom >= prev_body_bottom {
            let pattern_type = if prev_c < prev_o {
                PatternType::BullishHarami
            } else {
                PatternType::BearishHarami
            };
            
            return Some(PatternResult {
                pattern_type,
                confidence: 0.75,
                strength: 0.7,
                bullish_probability: if pattern_type == PatternType::BullishHarami { 0.65 } else { 0.35 },
                bearish_probability: if pattern_type == PatternType::BullishHarami { 0.35 } else { 0.65 },
                reliability_score: 0.7,
            });
        }
        
        None
    }
    
    fn check_piercing_dark_cloud(&self, prev: [f64; 4], curr: [f64; 4]) -> Option<PatternResult> {
        let [prev_o, _prev_h, _prev_l, prev_c] = prev;
        let [curr_o, _curr_h, _curr_l, curr_c] = curr;
        
        let prev_body_mid = (prev_o + prev_c) / 2.0;
        
        // Piercing Pattern
        if prev_c < prev_o && curr_c > curr_o && 
           curr_o < prev_c && curr_c > prev_body_mid && curr_c < prev_o {
            return Some(PatternResult {
                pattern_type: PatternType::PiercingPattern,
                confidence: 0.8,
                strength: 0.75,
                bullish_probability: 0.7,
                bearish_probability: 0.3,
                reliability_score: 0.75,
            });
        }
        
        // Dark Cloud Cover
        if prev_c > prev_o && curr_c < curr_o && 
           curr_o > prev_c && curr_c < prev_body_mid && curr_c > prev_o {
            return Some(PatternResult {
                pattern_type: PatternType::DarkCloudCover,
                confidence: 0.8,
                strength: 0.75,
                bullish_probability: 0.3,
                bearish_probability: 0.7,
                reliability_score: 0.75,
            });
        }
        
        None
    }
    
    fn check_tweezer_patterns(&self, prev: [f64; 4], curr: [f64; 4]) -> Option<PatternResult> {
        let [prev_o, prev_h, prev_l, prev_c] = prev;
        let [curr_o, curr_h, curr_l, curr_c] = curr;
        
        let high_diff = (prev_h - curr_h).abs();
        let low_diff = (prev_l - curr_l).abs();
        let avg_range = ((prev_h - prev_l) + (curr_h - curr_l)) / 2.0;
        
        if avg_range == 0.0 { return None; }
        
        // Tweezer Top
        if high_diff / avg_range < 0.02 && prev_c > prev_o && curr_c < curr_o {
            return Some(PatternResult {
                pattern_type: PatternType::TweezerTop,
                confidence: 0.75,
                strength: 0.7,
                bullish_probability: 0.3,
                bearish_probability: 0.7,
                reliability_score: 0.7,
            });
        }
        
        // Tweezer Bottom
        if low_diff / avg_range < 0.02 && prev_c < prev_o && curr_c > curr_o {
            return Some(PatternResult {
                pattern_type: PatternType::TweezerBottom,
                confidence: 0.75,
                strength: 0.7,
                bullish_probability: 0.7,
                bearish_probability: 0.3,
                reliability_score: 0.7,
            });
        }
        
        None
    }
    
    fn check_star_patterns(&self, first: [f64; 4], second: [f64; 4], third: [f64; 4]) -> Option<PatternResult> {
        let [f_o, f_h, f_l, f_c] = first;
        let [s_o, s_h, s_l, s_c] = second;
        let [t_o, t_h, t_l, t_c] = third;
        
        let _first_body = (f_c - f_o).abs();
        let second_body = (s_c - s_o).abs();
        let _third_body = (t_c - t_o).abs();
        let avg_range = ((f_h - f_l) + (s_h - s_l) + (t_h - t_l)) / 3.0;
        
        // Проверяем, что средняя свеча маленькая (star)
        if second_body / avg_range > 0.3 { return None; }
        
        // Morning Star
        if f_c < f_o && t_c > t_o && // Первая медвежья, третья бычья
           s_h < f_c.min(f_o) && // Гэп вниз
           t_c > (f_o + f_c) / 2.0 { // Третья закрывается выше середины первой
            
            let is_doji_star = self.is_doji(second_body, s_h - s_l);
            let pattern_type = if is_doji_star {
                PatternType::MorningDojiStar
            } else {
                PatternType::MorningStar
            };
            
            return Some(PatternResult {
                pattern_type,
                confidence: 0.85,
                strength: 0.8,
                bullish_probability: 0.8,
                bearish_probability: 0.2,
                reliability_score: if is_doji_star { 0.9 } else { 0.8 },
            });
        }
        
        // Evening Star
        if f_c > f_o && t_c < t_o && // Первая бычья, третья медвежья
           s_l > f_c.max(f_o) && // Гэп вверх
           t_c < (f_o + f_c) / 2.0 { // Третья закрывается ниже середины первой
            
            let is_doji_star = self.is_doji(second_body, s_h - s_l);
            let pattern_type = if is_doji_star {
                PatternType::EveningDojiStar
            } else {
                PatternType::EveningStar
            };
            
            return Some(PatternResult {
                pattern_type,
                confidence: 0.85,
                strength: 0.8,
                bullish_probability: 0.2,
                bearish_probability: 0.8,
                reliability_score: if is_doji_star { 0.9 } else { 0.8 },
            });
        }
        
        None
    }
    
    fn check_three_soldiers_crows(&self, first: [f64; 4], second: [f64; 4], third: [f64; 4]) -> Option<PatternResult> {
        let candles = [first, second, third];
        
        // Проверяем Three White Soldiers
        let mut is_white_soldiers = true;
        for i in 0..3 {
            let [o, h, l, c] = candles[i];
            let body_ratio = (c - o).abs() / (h - l);
            
            if c <= o || body_ratio < self.config.soldiers_min_body_ratio {
                is_white_soldiers = false;
                break;
            }
            
            if i > 0 {
                let prev_close = candles[i-1][3];
                if o <= prev_close || c <= candles[i-1][3] * self.config.soldiers_progression_ratio {
                    is_white_soldiers = false;
                    break;
                }
            }
        }
        
        if is_white_soldiers {
            return Some(PatternResult {
                pattern_type: PatternType::ThreeWhiteSoldiers,
                confidence: 0.9,
                strength: 0.85,
                bullish_probability: 0.85,
                bearish_probability: 0.15,
                reliability_score: 0.85,
            });
        }
        
        // Проверяем Three Black Crows
        let mut is_black_crows = true;
        for i in 0..3 {
            let [o, h, l, c] = candles[i];
            let body_ratio = (c - o).abs() / (h - l);
            
            if c >= o || body_ratio < self.config.soldiers_min_body_ratio {
                is_black_crows = false;
                break;
            }
            
            if i > 0 {
                let prev_close = candles[i-1][3];
                if o >= prev_close || c >= candles[i-1][3] / self.config.soldiers_progression_ratio {
                    is_black_crows = false;
                    break;
                }
            }
        }
        
        if is_black_crows {
            return Some(PatternResult {
                pattern_type: PatternType::ThreeBlackCrows,
                confidence: 0.9,
                strength: 0.85,
                bullish_probability: 0.15,
                bearish_probability: 0.85,
                reliability_score: 0.85,
            });
        }
        
        None
    }
    
    fn check_three_inside_patterns(&self, first: [f64; 4], second: [f64; 4], third: [f64; 4]) -> Option<PatternResult> {
        // Проверяем, что первые две свечи образуют Harami
        if let Some(harami_result) = self.check_harami_patterns(first, second) {
            let [_t_o, _t_h, _t_l, t_c] = third;
            let [f_o, _f_h, _f_l, _f_c] = first;
            
            // Three Inside Up
            if harami_result.pattern_type == PatternType::BullishHarami && t_c > f_o {
                return Some(PatternResult {
                    pattern_type: PatternType::ThreeInsideUp,
                    confidence: 0.8,
                    strength: 0.75,
                    bullish_probability: 0.75,
                    bearish_probability: 0.25,
                    reliability_score: 0.75,
                });
            }
            
            // Three Inside Down
            if harami_result.pattern_type == PatternType::BearishHarami && t_c < f_o {
                return Some(PatternResult {
                    pattern_type: PatternType::ThreeInsideDown,
                    confidence: 0.8,
                    strength: 0.75,
                    bullish_probability: 0.25,
                    bearish_probability: 0.75,
                    reliability_score: 0.75,
                });
            }
        }
        
        None
    }
    
    fn check_three_outside_patterns(&self, first: [f64; 4], second: [f64; 4], third: [f64; 4]) -> Option<PatternResult> {
        // Проверяем, что первые две свечи образуют Engulfing
        if let Some(engulfing_result) = self.check_engulfing_patterns(first, second) {
            let [_t_o, _t_h, _t_l, t_c] = third;
            let [_s_o, _s_h, _s_l, s_c] = second;
            
            // Three Outside Up
            if engulfing_result.pattern_type == PatternType::BullishEngulfing && t_c > s_c {
                return Some(PatternResult {
                    pattern_type: PatternType::ThreeOutsideUp,
                    confidence: 0.85,
                    strength: 0.8,
                    bullish_probability: 0.8,
                    bearish_probability: 0.2,
                    reliability_score: 0.8,
                });
            }
            
            // Three Outside Down
            if engulfing_result.pattern_type == PatternType::BearishEngulfing && t_c < s_c {
                return Some(PatternResult {
                    pattern_type: PatternType::ThreeOutsideDown,
                    confidence: 0.85,
                    strength: 0.8,
                    bullish_probability: 0.2,
                    bearish_probability: 0.8,
                    reliability_score: 0.8,
                });
            }
        }
        
        None
    }
    
    fn check_three_methods_patterns(&self, candles: &[[f64; 4]]) -> Option<PatternResult> {
        if candles.len() != 5 { return None; }

        let first = candles[4]; // Самая старая
        let last = candles[0];  // Самая новая
        let middle = &candles[1..4]; // Средние три

        let [f_o, f_h, f_l, f_c] = first;
        let [l_o, l_h, l_l, l_c] = last;

        let first_body = (f_c - f_o).abs();
        let last_body = (l_c - l_o).abs();
        let first_range = f_h - f_l;
        let last_range = l_h - l_l;

        // Rising Three Methods
        if f_c > f_o && l_c > l_o && // Первая и последняя бычьи
           l_c > f_c && // Последняя закрывается выше первой
           first_body / first_range > 0.6 && last_body / last_range > 0.6 {

            // Проверяем средние свечи
            let mut valid_middle = true;
            for &[_m_o, m_h, m_l, _m_c] in middle {
                if m_h > f_h || m_l < f_l { // Средние не выходят за пределы первой
                    valid_middle = false;
                    break;
                }
            }

            if valid_middle {
                return Some(PatternResult {
                    pattern_type: PatternType::RisingThreeMethods,
                    confidence: 0.8,
                    strength: 0.75,
                    bullish_probability: 0.75,
                    bearish_probability: 0.25,
                    reliability_score: 0.75,
                });
            }
        }

        // Falling Three Methods
        if f_c < f_o && l_c < l_o && // Первая и последняя медвежьи
           l_c < f_c && // Последняя закрывается ниже первой
           first_body / first_range > 0.6 && last_body / last_range > 0.6 {

            // Проверяем средние свечи
            let mut valid_middle = true;
            for &[_m_o, m_h, m_l, _m_c] in middle {
                if m_h > f_h || m_l < f_l { // Средние не выходят за пределы первой
                    valid_middle = false;
                    break;
                }
            }

            if valid_middle {
                return Some(PatternResult {
                    pattern_type: PatternType::FallingThreeMethods,
                    confidence: 0.8,
                    strength: 0.75,
                    bullish_probability: 0.25,
                    bearish_probability: 0.75,
                    reliability_score: 0.75,
                });
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_recognition_creation() {
        let ind = AdvancedPatternRecognition::new();
        assert!(!ind.is_ready());
    }

    #[test]
    fn test_pattern_recognition_warmup() {
        let mut ind = AdvancedPatternRecognition::new();
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_bar(price, price + 2.0, price - 2.0, price + 1.0, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_pattern_result_values() {
        let mut ind = AdvancedPatternRecognition::new();
        for i in 0..10 {
            let price = 100.0 + i as f64;
            ind.update_bar(price, price + 2.0, price - 2.0, price + 1.0, 1000.0);
        }
        let result = ind.result();
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
        assert!(result.strength >= 0.0 && result.strength <= 1.0);
        assert!(result.bullish_probability >= 0.0 && result.bullish_probability <= 1.0);
        assert!(result.bearish_probability >= 0.0 && result.bearish_probability <= 1.0);
    }

    #[test]
    fn test_pattern_type_properties() {
        assert!(PatternType::Hammer.is_bullish());
        assert!(PatternType::ShootingStar.is_bearish());
        assert_eq!(PatternType::Hammer.candle_count(), 1);
        assert_eq!(PatternType::BullishEngulfing.candle_count(), 2);
        assert_eq!(PatternType::MorningStar.candle_count(), 3);
    }

    #[test]
    fn test_pattern_recognition_reset() {
        let mut ind = AdvancedPatternRecognition::new();
        for i in 0..10 {
            let price = 100.0 + i as f64;
            ind.update_bar(price, price + 2.0, price - 2.0, price + 1.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
    }
} 























