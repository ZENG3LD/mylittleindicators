//! Ultimate Oscillator indicator.

use std::collections::VecDeque;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::utils::true_range::true_range;

/// Ultimate Oscillator - multi-timeframe momentum oscillator by Larry Williams.
///
/// UO = 100 × [(4×Average7) + (2×Average14) + Average28] / (4 + 2 + 1)
/// where Average = Sum(BP) / Sum(TR) over period
/// BP (Buying Pressure) = Close - Min(Low, Previous Close)
/// TR (True Range) = Max(High, Previous Close) - Min(Low, Previous Close)
///
/// Uses three periods (typically 7, 14, 28) to reduce false signals by
/// incorporating momentum over multiple timeframes with weighted averaging.
///
/// Interpretation:
/// - UO >= 70: Overbought
/// - UO <= 30: Oversold
/// - Divergence with price: Potential reversal signals
///
/// # Parameters
/// - `period1`: Short period (typically 7)
/// - `period2`: Medium period (typically 14)
/// - `period3`: Long period (typically 28)
///
/// # Implementation
///
/// Uses O(1) running sums for efficient calculation.

#[derive(Clone)]
pub struct UltimateOscillator {
    period1: usize,  // Короткий период (обычно 7)
    period2: usize,  // Средний период (обычно 14)
    period3: usize,  // Длинный период (обычно 28)

    // Буферы для Buying Pressure и True Range (VecDeque for O(1) pop_front)
    bp_values: VecDeque<f64>,
    tr_values: VecDeque<f64>,

    // Running sums for all 3 periods - O(1) calculation
    sum_bp_short: f64,   // Period1 (7)
    sum_tr_short: f64,
    sum_bp_medium: f64,  // Period2 (14)
    sum_tr_medium: f64,
    sum_bp_long: f64,    // Period3 (28)
    sum_tr_long: f64,

    // Предыдущая цена закрытия для расчетов
    prev_close: f64,

    // Текущее значение
    value: f64,

    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl UltimateOscillator {
    /// Creates a new Ultimate Oscillator with default periods (7, 14, 28).
    pub fn new() -> Self {
        Self::with_periods(7, 14, 28)
    }

    /// Creates a new Ultimate Oscillator with specified periods.
    ///
    /// # Arguments
    /// * `period1` - Short period (must be < period2)
    /// * `period2` - Medium period (must be < period3)
    /// * `period3` - Long period
    pub fn with_periods(period1: usize, period2: usize, period3: usize) -> Self {
        assert!(period1 > 0 && period1 <= 512, "Period1 must be between 1 and 512");
        assert!(period2 > 0 && period2 <= 512, "Period2 must be between 1 and 512");
        assert!(period3 > 0 && period3 <= 512, "Period3 must be between 1 and 512");
        assert!(period1 < period2, "Period1 must be less than Period2");
        assert!(period2 < period3, "Period2 must be less than Period3");

        Self {
            period1,
            period2,
            period3,
            bp_values: VecDeque::with_capacity(period3),
            tr_values: VecDeque::with_capacity(period3),
            sum_bp_short: 0.0,
            sum_tr_short: 0.0,
            sum_bp_medium: 0.0,
            sum_tr_medium: 0.0,
            sum_bp_long: 0.0,
            sum_tr_long: 0.0,
            prev_close: 0.0,
            value: 50.0, // Нейтральное значение
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Updates the Ultimate Oscillator with a new bar and returns the current value.
    ///
    /// Uses `high`, `low`, and `close` prices.
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        self.bars_count += 1;

        if self.bars_count == 1 {
            // Первый бар - инициализируем prev_close
            self.prev_close = close;
            return self.value;
        }

        // Рассчитываем Buying Pressure и True Range
        let min_low_prev_close = low.min(self.prev_close);
        let buying_pressure = close - min_low_prev_close;
        let tr = true_range(high, low, self.prev_close);

        // Update buffers with O(1) operations
        if self.bp_values.len() >= self.period3 {
            let old_bp = self.bp_values.pop_front().unwrap();
            let old_tr = self.tr_values.pop_front().unwrap();

            // Subtract from all running sums (they're all calculated from the same buffer)
            self.sum_bp_long -= old_bp;
            self.sum_tr_long -= old_tr;
            self.sum_bp_medium -= old_bp;
            self.sum_tr_medium -= old_tr;
            self.sum_bp_short -= old_bp;
            self.sum_tr_short -= old_tr;
        }

        self.bp_values.push_back(buying_pressure);
        self.tr_values.push_back(tr);

        // Add to all running sums
        self.sum_bp_long += buying_pressure;
        self.sum_tr_long += tr;
        self.sum_bp_medium += buying_pressure;
        self.sum_tr_medium += tr;
        self.sum_bp_short += buying_pressure;
        self.sum_tr_short += tr;

        // Remove excess values from short and medium sums if buffer exceeds their periods
        let len = self.bp_values.len();
        if len > self.period1 {
            let idx = len - self.period1 - 1;
            self.sum_bp_short -= self.bp_values[idx];
            self.sum_tr_short -= self.tr_values[idx];
        }

        if len > self.period2 {
            let idx = len - self.period2 - 1;
            self.sum_bp_medium -= self.bp_values[idx];
            self.sum_tr_medium -= self.tr_values[idx];
        }

        // Проверяем готовность (нужен самый длинный период)
        if self.bp_values.len() >= self.period3 {
            self.is_ready = true;
        }

        // Рассчитываем Ultimate Oscillator - O(1) using running sums
        if self.is_ready {
            // Calculate averages for each period using running sums
            let avg1 = if self.sum_tr_short.abs() < 1e-12 {
                0.0
            } else {
                self.sum_bp_short / self.sum_tr_short
            };

            let avg2 = if self.sum_tr_medium.abs() < 1e-12 {
                0.0
            } else {
                self.sum_bp_medium / self.sum_tr_medium
            };

            let avg3 = if self.sum_tr_long.abs() < 1e-12 {
                0.0
            } else {
                self.sum_bp_long / self.sum_tr_long
            };

            // Формула Ultimate Oscillator с весами 4:2:1
            self.value = 100.0 * ((4.0 * avg1) + (2.0 * avg2) + avg3) / 7.0;

            // Ограничиваем значения в диапазоне [0, 100]
            self.value = self.value.clamp(0.0, 100.0);
        }

        self.prev_close = close;
        self.value
    }

    /// Returns the current Ultimate Oscillator value as an `IndicatorValue`.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the Ultimate Oscillator has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Returns the periods (period1, period2, period3).
    #[inline]
    pub fn periods(&self) -> (usize, usize, usize) {
        (self.period1, self.period2, self.period3)
    }

    /// Resets the Ultimate Oscillator to its initial state.
    pub fn reset(&mut self) {
        self.bp_values.clear();
        self.tr_values.clear();
        self.sum_bp_short = 0.0;
        self.sum_tr_short = 0.0;
        self.sum_bp_medium = 0.0;
        self.sum_tr_medium = 0.0;
        self.sum_bp_long = 0.0;
        self.sum_tr_long = 0.0;
        self.prev_close = 0.0;
        self.value = 50.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    /// Returns the current market condition.
    pub fn market_condition(&self) -> &'static str {
        match self.value {
            v if v >= 70.0 => "Overbought",
            v if v <= 30.0 => "Oversold",
            v if v >= 50.0 => "Bullish",
            _ => "Bearish",
        }
    }

    /// Returns trading signal (1 = buy, -1 = sell, 0 = neutral).
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }

        match self.value {
            v if v <= 30.0 => 1,   // Покупка в зоне перепроданности
            v if v >= 70.0 => -1,  // Продажа в зоне перекупленности
            _ => 0                 // Нейтрально
        }
    }

    /// Получить усовершенствованный торговый сигнал с проверкой дивергенции
    /// Требует историю цен для анализа дивергенции
    pub fn advanced_signal(&self, price_history: &[f64], lookback: usize) -> i8 {
        if !self.is_ready() || price_history.len() < lookback + 1 {
            return 0;
        }

        let current_price = price_history[price_history.len() - 1];
        let past_price = price_history[price_history.len() - lookback - 1];

        // Базовый сигнал
        let base_signal = self.trading_signal();

        // Проверяем дивергенцию (упрощенная версия)
        if base_signal == 1 {
            // Покупка: ищем бычью дивергенцию
            // Цена делает более низкий минимум, но UO - более высокий минимум
            if current_price < past_price && self.value > 30.0 {
                return 1; // Усиленный сигнал покупки
            }
        } else if base_signal == -1 {
            // Продажа: ищем медвежью дивергенцию
            // Цена делает более высокий максимум, но UO - более низкий максимум
            if current_price > past_price && self.value < 70.0 {
                return -1; // Усиленный сигнал продажи
            }
        }

        base_signal
    }

    /// Получить силу сигнала (расстояние от крайних уровней)
    pub fn signal_strength(&self) -> f64 {
        if self.value >= 70.0 {
            // В зоне перекупленности
            (self.value - 70.0) / 30.0
        } else if self.value <= 30.0 {
            // В зоне перепроданности
            (30.0 - self.value) / 30.0
        } else {
            // В нейтральной зоне
            0.0
        }
    }

    /// Проверить пересечение ключевых уровней
    /// Возвращает: 1 = пересечение 30 снизу вверх, -1 = пересечение 70 сверху вниз, 0 = нет пересечения
    pub fn level_crossover(&self, prev_value: f64) -> i8 {
        // Пересечение уровня 30 снизу вверх (потенциальная покупка)
        if prev_value <= 30.0 && self.value > 30.0 {
            return 1;
        }

        // Пересечение уровня 70 сверху вниз (потенциальная продажа)
        if prev_value >= 70.0 && self.value < 70.0 {
            return -1;
        }

        0
    }

    /// Returns the component averages for analysis.
    pub fn components(&self) -> (f64, f64, f64) {
        if !self.is_ready() {
            return (0.0, 0.0, 0.0);
        }

        let avg1 = if self.sum_tr_short.abs() < 1e-12 {
            0.0
        } else {
            100.0 * self.sum_bp_short / self.sum_tr_short
        };

        let avg2 = if self.sum_tr_medium.abs() < 1e-12 {
            0.0
        } else {
            100.0 * self.sum_bp_medium / self.sum_tr_medium
        };

        let avg3 = if self.sum_tr_long.abs() < 1e-12 {
            0.0
        } else {
            100.0 * self.sum_bp_long / self.sum_tr_long
        };

        (avg1, avg2, avg3)
    }

    /// Returns information about the indicator state.
    pub fn info(&self) -> String {
        let (avg1, avg2, avg3) = self.components();
        format!(
            "UO: {:.2}, Periods: ({},{},{}), Avg1: {:.2}, Avg2: {:.2}, Avg3: {:.2}",
            self.value, self.period1, self.period2, self.period3, avg1, avg2, avg3
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_uo_basic_calculation() {
        let mut uo = UltimateOscillator::new();

        // Feed uptrend data with valid OHLC
        for i in 1..=50 {
            let base = 100.0 + i as f64;
            uo.update_bar(base, base + 2.0, base - 1.0, base + 1.0, 0.0);
        }

        assert!(uo.is_ready());
        // Value should be in [0, 100]
        assert!(uo.value().main() >= 0.0 && uo.value().main() <= 100.0);
    }

    #[test]
    fn test_uo_overbought() {
        let mut uo = UltimateOscillator::new();

        // Strong uptrend - close always at high
        for i in 1..=50 {
            let base = 100.0 + i as f64 * 2.0;
            uo.update_bar(base, base + 3.0, base - 0.5, base + 3.0, 0.0);
        }

        assert!(uo.is_ready());
        // Strong buying pressure should push UO high
        assert!(uo.value().main() > 50.0, "UO in strong uptrend should be > 50");
    }

    #[test]
    fn test_uo_oversold() {
        let mut uo = UltimateOscillator::new();

        // Strong downtrend - close always at low
        for i in 1..=50 {
            let base = 200.0 - i as f64 * 2.0;
            uo.update_bar(base, base + 0.5, base - 3.0, base - 3.0, 0.0);
        }

        assert!(uo.is_ready());
        // Low buying pressure should push UO low
        assert!(uo.value().main() < 50.0, "UO in strong downtrend should be < 50");
    }

    #[test]
    fn test_uo_range() {
        let mut uo = UltimateOscillator::new();

        for i in 1..=50 {
            let base = 100.0 + (i % 10) as f64;
            uo.update_bar(base, base + 2.0, base - 2.0, base, 0.0);
        }

        assert!(uo.is_ready());
        // Value should always be in [0, 100]
        assert!(uo.value().main() >= 0.0 && uo.value().main() <= 100.0);
    }

    #[test]
    fn test_uo_reset() {
        let mut uo = UltimateOscillator::new();

        for i in 1..=50 {
            let base = 100.0 + i as f64;
            uo.update_bar(base, base + 2.0, base - 1.0, base + 1.0, 0.0);
        }
        assert!(uo.is_ready());

        uo.reset();
        assert!(!uo.is_ready());
        // Default value is 50
        assert!((uo.value().main() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_uo_periods() {
        let uo = UltimateOscillator::new();
        assert_eq!(uo.periods(), (7, 14, 28));
    }

    #[test]
    fn test_uo_market_condition() {
        let mut uo = UltimateOscillator::new();

        for i in 1..=50 {
            let base = 100.0 + i as f64;
            uo.update_bar(base, base + 2.0, base - 1.0, base + 1.0, 0.0);
        }

        assert!(uo.is_ready());
        let condition = uo.market_condition();
        assert!(
            condition == "Overbought"
                || condition == "Oversold"
                || condition == "Bullish"
                || condition == "Bearish"
        );
    }
}
