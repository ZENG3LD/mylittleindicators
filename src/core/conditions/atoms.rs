//! Condition Primitives - базовые условия для генерации сигналов
//!
//! Условия - это абстрактные примитивы, которые можно применять к любым индикаторам.
//! Они не привязаны к конкретным индикаторам, только к типам значений.

use serde::{Deserialize, Serialize};

// ============================================================================
// THRESHOLD CONDITIONS - Пороговые условия
// ============================================================================

/// Условие порога
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ThresholdCondition {
    /// Значение выше порога
    Above(f64),
    /// Значение ниже порога
    Below(f64),
    /// Значение в диапазоне [min, max]
    InRange(f64, f64),
    /// Значение вне диапазона
    OutOfRange(f64, f64),
    /// Значение примерно равно (с допуском)
    Near(f64, f64), // value, tolerance
}

impl ThresholdCondition {
    /// Проверить условие
    #[inline]
    pub fn check(&self, value: f64) -> bool {
        match self {
            Self::Above(threshold) => value > *threshold,
            Self::Below(threshold) => value < *threshold,
            Self::InRange(min, max) => value >= *min && value <= *max,
            Self::OutOfRange(min, max) => value < *min || value > *max,
            Self::Near(target, tolerance) => (value - target).abs() <= *tolerance,
        }
    }

    /// Проверить переход через порог (было false, стало true)
    #[inline]
    pub fn check_transition(&self, prev: f64, curr: f64) -> bool {
        !self.check(prev) && self.check(curr)
    }

    /// Проверить выход из условия (было true, стало false)
    #[inline]
    pub fn check_exit(&self, prev: f64, curr: f64) -> bool {
        self.check(prev) && !self.check(curr)
    }
}

// ============================================================================
// CROSSOVER CONDITIONS - Условия пересечения
// ============================================================================

/// Тип пересечения
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossoverType {
    /// Пересечение снизу вверх
    CrossUp,
    /// Пересечение сверху вниз
    CrossDown,
    /// Любое пересечение
    CrossAny,
}

impl CrossoverType {
    /// Проверить пересечение двух линий
    #[inline]
    pub fn check(&self, prev_a: f64, curr_a: f64, prev_b: f64, curr_b: f64) -> bool {
        match self {
            Self::CrossUp => prev_a <= prev_b && curr_a > curr_b,
            Self::CrossDown => prev_a >= prev_b && curr_a < curr_b,
            Self::CrossAny => {
                (prev_a <= prev_b && curr_a > curr_b) || (prev_a >= prev_b && curr_a < curr_b)
            }
        }
    }

    /// Проверить пересечение линии с уровнем
    #[inline]
    pub fn check_level(&self, prev: f64, curr: f64, level: f64) -> bool {
        match self {
            Self::CrossUp => prev <= level && curr > level,
            Self::CrossDown => prev >= level && curr < level,
            Self::CrossAny => {
                (prev <= level && curr > level) || (prev >= level && curr < level)
            }
        }
    }
}

// ============================================================================
// COMPARISON CONDITIONS - Условия сравнения
// ============================================================================

/// Условие сравнения двух значений
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompareCondition {
    /// A > B
    Greater,
    /// A >= B
    GreaterOrEqual,
    /// A < B
    Less,
    /// A <= B
    LessOrEqual,
    /// A == B (с допуском)
    Equal,
    /// A != B (с допуском)
    NotEqual,
}

impl CompareCondition {
    /// Проверить условие с допуском
    #[inline]
    pub fn check(&self, a: f64, b: f64, tolerance: f64) -> bool {
        match self {
            Self::Greater => a > b + tolerance,
            Self::GreaterOrEqual => a >= b - tolerance,
            Self::Less => a < b - tolerance,
            Self::LessOrEqual => a <= b + tolerance,
            Self::Equal => (a - b).abs() <= tolerance,
            Self::NotEqual => (a - b).abs() > tolerance,
        }
    }
}

// ============================================================================
// TREND CONDITIONS - Трендовые условия
// ============================================================================

/// Условие тренда
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendCondition {
    /// Восходящий тренд (последовательные более высокие значения)
    Rising,
    /// Нисходящий тренд (последовательные более низкие значения)
    Falling,
    /// Боковик (значения колеблются в диапазоне)
    Sideways,
    /// Ускорение роста
    Accelerating,
    /// Замедление роста
    Decelerating,
}

impl TrendCondition {
    /// Проверить тренд по последним N значениям
    pub fn check(&self, values: &[f64], tolerance: f64) -> bool {
        if values.len() < 2 {
            return false;
        }

        match self {
            Self::Rising => {
                values.windows(2).all(|w| w[1] > w[0] - tolerance)
            }
            Self::Falling => {
                values.windows(2).all(|w| w[1] < w[0] + tolerance)
            }
            Self::Sideways => {
                let first = values[0];
                values.iter().all(|&v| (v - first).abs() <= tolerance)
            }
            Self::Accelerating => {
                if values.len() < 3 {
                    return false;
                }
                let diffs: Vec<f64> = values.windows(2).map(|w| w[1] - w[0]).collect();
                diffs.windows(2).all(|w| w[1] > w[0])
            }
            Self::Decelerating => {
                if values.len() < 3 {
                    return false;
                }
                let diffs: Vec<f64> = values.windows(2).map(|w| w[1] - w[0]).collect();
                diffs.windows(2).all(|w| w[1] < w[0])
            }
        }
    }
}

// ============================================================================
// DIVERGENCE CONDITIONS - Условия дивергенции
// ============================================================================

/// Тип дивергенции
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DivergenceType {
    /// Бычья дивергенция: цена падает, индикатор растёт
    Bullish,
    /// Медвежья дивергенция: цена растёт, индикатор падает
    Bearish,
    /// Скрытая бычья: цена растёт, индикатор падает (продолжение тренда)
    HiddenBullish,
    /// Скрытая медвежья: цена падает, индикатор растёт (продолжение тренда)
    HiddenBearish,
}

impl DivergenceType {
    /// Проверить дивергенцию по двум точкам
    /// price1, price2 - цены в точках 1 и 2
    /// ind1, ind2 - значения индикатора в точках 1 и 2
    pub fn check(&self, price1: f64, price2: f64, ind1: f64, ind2: f64) -> bool {
        match self {
            Self::Bullish => {
                // Цена делает новый минимум, индикатор - более высокий минимум
                price2 < price1 && ind2 > ind1
            }
            Self::Bearish => {
                // Цена делает новый максимум, индикатор - более низкий максимум
                price2 > price1 && ind2 < ind1
            }
            Self::HiddenBullish => {
                // Цена делает более высокий минимум, индикатор - новый минимум
                price2 > price1 && ind2 < ind1
            }
            Self::HiddenBearish => {
                // Цена делает более низкий максимум, индикатор - новый максимум
                price2 < price1 && ind2 > ind1
            }
        }
    }
}

// ============================================================================
// CHANNEL CONDITIONS - Канальные условия
// ============================================================================

/// Позиция относительно канала
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelPosition {
    /// Выше верхней границы
    AboveUpper,
    /// Касается верхней границы
    AtUpper,
    /// В верхней половине канала
    UpperHalf,
    /// В середине канала
    Middle,
    /// В нижней половине канала
    LowerHalf,
    /// Касается нижней границы
    AtLower,
    /// Ниже нижней границы
    BelowLower,
}

impl ChannelPosition {
    /// Определить позицию значения в канале
    pub fn determine(value: f64, upper: f64, lower: f64, tolerance: f64) -> Self {
        let mid = (upper + lower) / 2.0;

        if value > upper + tolerance {
            Self::AboveUpper
        } else if (value - upper).abs() <= tolerance {
            Self::AtUpper
        } else if value > mid {
            Self::UpperHalf
        } else if (value - mid).abs() <= tolerance {
            Self::Middle
        } else if value > lower {
            Self::LowerHalf
        } else if (value - lower).abs() <= tolerance {
            Self::AtLower
        } else {
            Self::BelowLower
        }
    }

    /// Является ли позиция экстремальной (у границ или за ними)
    pub fn is_extreme(&self) -> bool {
        matches!(
            self,
            Self::AboveUpper | Self::AtUpper | Self::AtLower | Self::BelowLower
        )
    }
}

// ============================================================================
// PATTERN CONDITIONS - Условия паттернов
// ============================================================================

/// Состояние формирования паттерна
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternState {
    /// Паттерн не обнаружен
    None,
    /// Паттерн формируется
    Forming,
    /// Паттерн завершён
    Complete,
    /// Паттерн подтверждён
    Confirmed,
    /// Паттерн сломан/отменён
    Broken,
}

/// Тип свечного паттерна
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandlePattern {
    // Одиночные свечи
    Doji,
    Hammer,
    InvertedHammer,
    ShootingStar,
    HangingMan,
    Marubozu,
    SpinningTop,

    // Двойные паттерны
    BullishEngulfing,
    BearishEngulfing,
    BullishHarami,
    BearishHarami,
    PiercingLine,
    DarkCloudCover,
    Tweezer,

    // Тройные паттерны
    MorningStar,
    EveningStar,
    ThreeWhiteSoldiers,
    ThreeBlackCrows,
    ThreeInsideUp,
    ThreeInsideDown,
}

// ============================================================================
// VOLATILITY CONDITIONS - Условия волатильности
// ============================================================================

/// Режим волатильности
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolatilityRegime {
    /// Очень низкая волатильность
    VeryLow,
    /// Низкая волатильность
    Low,
    /// Нормальная волатильность
    Normal,
    /// Высокая волатильность
    High,
    /// Очень высокая волатильность
    VeryHigh,
}

impl VolatilityRegime {
    /// Определить режим волатильности по Z-score
    pub fn from_zscore(zscore: f64) -> Self {
        if zscore < -1.5 {
            Self::VeryLow
        } else if zscore < -0.5 {
            Self::Low
        } else if zscore < 0.5 {
            Self::Normal
        } else if zscore < 1.5 {
            Self::High
        } else {
            Self::VeryHigh
        }
    }

    /// Определить режим по процентилю (0-100)
    pub fn from_percentile(percentile: f64) -> Self {
        if percentile < 10.0 {
            Self::VeryLow
        } else if percentile < 30.0 {
            Self::Low
        } else if percentile < 70.0 {
            Self::Normal
        } else if percentile < 90.0 {
            Self::High
        } else {
            Self::VeryHigh
        }
    }
}

// ============================================================================
// VOLUME CONDITIONS - Условия объёма
// ============================================================================

/// Характер объёма
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolumeCharacter {
    /// Очень низкий объём
    VeryLow,
    /// Низкий объём
    Low,
    /// Нормальный объём
    Normal,
    /// Выше среднего
    AboveAverage,
    /// Высокий объём
    High,
    /// Всплеск объёма
    Spike,
    /// Кульминация
    Climax,
}

impl VolumeCharacter {
    /// Определить характер объёма по отношению к среднему
    pub fn from_ratio(volume_ratio: f64) -> Self {
        if volume_ratio < 0.3 {
            Self::VeryLow
        } else if volume_ratio < 0.7 {
            Self::Low
        } else if volume_ratio < 1.3 {
            Self::Normal
        } else if volume_ratio < 2.0 {
            Self::AboveAverage
        } else if volume_ratio < 3.0 {
            Self::High
        } else if volume_ratio < 5.0 {
            Self::Spike
        } else {
            Self::Climax
        }
    }
}

// ============================================================================
// COMPOSITE CONDITIONS - Комбинированные условия
// ============================================================================

/// Логический оператор для комбинирования условий
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogicOp {
    And,
    Or,
    Not,
    Xor,
}

impl LogicOp {
    pub fn apply(&self, a: bool, b: bool) -> bool {
        match self {
            Self::And => a && b,
            Self::Or => a || b,
            Self::Not => !a,
            Self::Xor => a ^ b,
        }
    }
}

/// Требование подтверждения
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfirmationRequirement {
    /// Без подтверждения (сигнал на том же баре)
    Immediate,
    /// Подтверждение на следующем баре
    NextBar,
    /// Подтверждение в течение N баров
    WithinBars(usize),
    /// Подтверждение закрытием выше/ниже уровня
    CloseConfirmation,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_condition() {
        let above = ThresholdCondition::Above(70.0);
        assert!(above.check(75.0));
        assert!(!above.check(65.0));

        let in_range = ThresholdCondition::InRange(30.0, 70.0);
        assert!(in_range.check(50.0));
        assert!(!in_range.check(80.0));
    }

    #[test]
    fn test_threshold_transition() {
        let above = ThresholdCondition::Above(70.0);
        assert!(above.check_transition(65.0, 75.0));
        assert!(!above.check_transition(75.0, 80.0));
    }

    #[test]
    fn test_crossover() {
        let cross_up = CrossoverType::CrossUp;
        // Line A crosses above line B
        assert!(cross_up.check(45.0, 55.0, 50.0, 50.0));
        // Line A stays below
        assert!(!cross_up.check(45.0, 48.0, 50.0, 50.0));
    }

    #[test]
    fn test_crossover_level() {
        let cross_up = CrossoverType::CrossUp;
        assert!(cross_up.check_level(48.0, 52.0, 50.0));
        assert!(!cross_up.check_level(52.0, 55.0, 50.0));
    }

    #[test]
    fn test_trend_condition() {
        let rising = TrendCondition::Rising;
        assert!(rising.check(&[10.0, 20.0, 30.0, 40.0], 0.0));
        assert!(!rising.check(&[40.0, 30.0, 20.0, 10.0], 0.0));
    }

    #[test]
    fn test_divergence() {
        let bullish = DivergenceType::Bullish;
        // Price makes lower low, indicator makes higher low
        assert!(bullish.check(100.0, 95.0, 30.0, 35.0));
        assert!(!bullish.check(100.0, 105.0, 30.0, 35.0));
    }

    #[test]
    fn test_channel_position() {
        let pos = ChannelPosition::determine(75.0, 80.0, 60.0, 1.0);
        assert_eq!(pos, ChannelPosition::UpperHalf);

        let pos = ChannelPosition::determine(85.0, 80.0, 60.0, 1.0);
        assert_eq!(pos, ChannelPosition::AboveUpper);
    }

    #[test]
    fn test_volatility_regime() {
        assert_eq!(VolatilityRegime::from_zscore(-2.0), VolatilityRegime::VeryLow);
        assert_eq!(VolatilityRegime::from_zscore(0.0), VolatilityRegime::Normal);
        assert_eq!(VolatilityRegime::from_zscore(2.0), VolatilityRegime::VeryHigh);
    }

    #[test]
    fn test_volume_character() {
        assert_eq!(VolumeCharacter::from_ratio(0.2), VolumeCharacter::VeryLow);
        assert_eq!(VolumeCharacter::from_ratio(1.0), VolumeCharacter::Normal);
        assert_eq!(VolumeCharacter::from_ratio(4.0), VolumeCharacter::Spike);
    }
}
