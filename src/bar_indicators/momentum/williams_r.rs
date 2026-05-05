//! Williams %R indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Williams %R - momentum oscillator showing close position relative to high-low range.
///
/// %R = (Highest High - Close) / (Highest High - Lowest Low) × -100
///
/// Range: -100 to 0
/// - Above -20: Overbought
/// - Below -80: Oversold
///
/// Similar to Stochastic %K but inverted (0 at top, -100 at bottom).
///
/// # Parameters
/// - `period`: Lookback period (typically 14)
///
/// # Implementation
///
/// Uses ring buffer for high/low tracking. O(period) per update.
/// Maximum period is 512.
#[derive(Clone)]
pub struct WilliamsR {
    period: usize,
    highs: ArrayVec<f64, 512>,
    lows: ArrayVec<f64, 512>,
    closes: ArrayVec<f64, 512>,
    index: usize,
    filled: bool,
    value: f64,
}

impl WilliamsR {
    /// Создать новый Williams %R с заданным периодом
    pub fn new(period: usize) -> Self {
        assert!(period > 0 && period <= 512, "Period must be between 1 and 512");

        Self {
            period,
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            closes: ArrayVec::new(),
            index: 0,
            filled: false,
            value: -50.0, // Нейтральное значение
        }
    }

    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        // Добавляем новые значения
        if self.highs.len() >= self.period {
            self.highs.remove(0);
        }
        self.highs.push(high);

        if self.lows.len() >= self.period {
            self.lows.remove(0);
        }
        self.lows.push(low);

        if self.closes.len() >= self.period {
            self.closes.remove(0);
        }
        self.closes.push(close);

        self.index += 1;

        // Проверяем, заполнены ли буферы
        if self.highs.len() >= self.period {
            self.filled = true;
        }

        // Вычисляем Williams %R
        if self.filled {
            let (lowest_low, highest_high) = self.highs.iter()
                .zip(self.lows.iter())
                .fold((f64::INFINITY, f64::NEG_INFINITY),
                      |(min, max), (&h, &l)| (min.min(l), max.max(h)));
            let current_close = close;

            let range = highest_high - lowest_low;

            if range.abs() < 1e-12 {
                // Если диапазон близок к нулю, возвращаем нейтральное значение
                self.value = -50.0;
            } else {
                // Формула Williams %R
                self.value = ((highest_high - current_close) / range) * -100.0;

                // Ограничиваем значения в диапазоне [-100, 0]
                self.value = self.value.clamp(-100.0, 0.0);
            }
        }

        self.value
    }

    /// Получить текущее значение Williams %R
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Проверить, готов ли индикатор (заполнен период)
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Получить период индикатора
    pub fn period(&self) -> usize {
        self.period
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.closes.clear();
        self.index = 0;
        self.filled = false;
        self.value = -50.0;
    }

    /// Определить состояние рынка на основе Williams %R
    pub fn market_condition(&self) -> &'static str {
        match self.value {
            v if v >= -20.0 => "Overbought",     // Перекупленность
            v if v <= -80.0 => "Oversold",      // Перепроданность
            _ => "Neutral"                      // Нейтральная зона
        }
    }

    /// Получить торговый сигнал
    /// 1 = покупка (выход из перепроданности)
    /// -1 = продажа (выход из перекупленности)
    /// 0 = нейтрально
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() || self.closes.len() < 2 {
            return 0;
        }

        let prev_close = self.closes[self.closes.len() - 2];
        let _current_close = self.closes[self.closes.len() - 1];

        // Вычисляем предыдущее значение %R для определения пересечений
        let prev_highs = &self.highs[..self.highs.len()-1];
        let prev_lows = &self.lows[..self.lows.len()-1];

        if prev_highs.len() >= self.period - 1 && prev_lows.len() >= self.period - 1 {
            let (prev_lowest, prev_highest) = prev_highs.iter()
                .zip(prev_lows.iter())
                .fold((f64::INFINITY, f64::NEG_INFINITY),
                      |(min, max), (&h, &l)| (min.min(l), max.max(h)));
            let prev_range = prev_highest - prev_lowest;

            let prev_williams_r = if prev_range.abs() < 1e-12 {
                -50.0
            } else {
                ((prev_highest - prev_close) / prev_range) * -100.0
            };

            // Сигнал покупки: пересечение уровня -80 снизу вверх (выход из перепроданности)
            if prev_williams_r <= -80.0 && self.value > -80.0 {
                return 1;
            }

            // Сигнал продажи: пересечение уровня -20 сверху вниз (выход из перекупленности)
            if prev_williams_r >= -20.0 && self.value < -20.0 {
                return -1;
            }
        }

        0
    }

    /// Получить силу сигнала (расстояние от крайних уровней)
    pub fn signal_strength(&self) -> f64 {
        if self.value >= -20.0 {
            // В зоне перекупленности - сила = расстояние от -20
            (self.value + 20.0) / 20.0
        } else if self.value <= -80.0 {
            // В зоне перепроданности - сила = расстояние от -80
            (-80.0 - self.value) / 20.0
        } else {
            // В нейтральной зоне
            0.0
        }
    }

    /// Проверить дивергенцию с ценой
    /// Возвращает: 1 = бычья дивергенция, -1 = медвежья дивергенция, 0 = нет дивергенции
    pub fn check_divergence(&self, lookback: usize) -> i8 {
        if !self.is_ready() || self.closes.len() < lookback + 1 {
            return 0;
        }

        let len = self.closes.len();
        if len < lookback + 1 {
            return 0;
        }

        // Сравниваем последние два максимума/минимума
        let recent_price = self.closes[len - 1];
        let past_price = self.closes[len - lookback - 1];

        // Вычисляем прошлое значение Williams %R
        let start_idx = if len >= lookback + self.period { len - lookback - self.period } else { 0 };
        let end_idx = len - lookback;

        if end_idx > start_idx {
            let past_highs = &self.highs[start_idx..end_idx];
            let past_lows = &self.lows[start_idx..end_idx];

            if past_highs.len() >= self.period && past_lows.len() >= self.period {
                let past_highest = past_highs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let past_lowest = past_lows.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                let past_range = past_highest - past_lowest;

                let past_williams_r = if past_range.abs() < 1e-12 {
                    -50.0
                } else {
                    ((past_highest - past_price) / past_range) * -100.0
                };

                // Бычья дивергенция: цена делает новый минимум, а Williams %R - более высокий минимум
                if recent_price < past_price && self.value > past_williams_r && self.value <= -60.0 {
                    return 1;
                }

                // Медвежья дивергенция: цена делает новый максимум, а Williams %R - более низкий максимум
                if recent_price > past_price && self.value < past_williams_r && self.value >= -40.0 {
                    return -1;
                }
            }
        }

        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_williams_r_basic_calculation() {
        let mut wr = WilliamsR::new(14);

        // Feed uptrend data closing near highs
        for i in 1..=20 {
            let base = 100.0 + i as f64;
            wr.update_bar(0.0, base + 2.0, base - 2.0, base + 1.5, 0.0);
        }

        assert!(wr.is_ready());
        // Close near high = %R near 0 (overbought zone)
        assert!(wr.value().main() > -30.0, "Williams %R near high should be > -30, got {}", wr.value().main());
    }

    #[test]
    fn test_williams_r_at_low() {
        let mut wr = WilliamsR::new(14);

        // Feed data with SAME range, closing near lows
        for _ in 1..=20 {
            wr.update_bar(0.0, 110.0, 90.0, 91.0, 0.0);
        }

        assert!(wr.is_ready());
        // Close near low = %R near -100 (oversold zone)
        assert!(wr.value().main() < -80.0, "Williams %R near low should be < -80, got {}", wr.value().main());
    }

    #[test]
    fn test_williams_r_range() {
        let mut wr = WilliamsR::new(14);

        // Feed various data
        for i in 1..=20 {
            let base = 100.0 + (i % 5) as f64;
            wr.update_bar(0.0, base + 3.0, base - 3.0, base, 0.0);
        }

        assert!(wr.is_ready());
        let val = wr.value().main();
        assert!(val >= -100.0 && val <= 0.0, "Williams %R should be in [-100, 0], got {}", val);
    }

    #[test]
    fn test_williams_r_market_condition() {
        let mut wr = WilliamsR::new(14);

        // Set up for overbought - same range, close near high
        for _ in 1..=20 {
            wr.update_bar(0.0, 110.0, 90.0, 109.0, 0.0);
        }
        assert_eq!(wr.market_condition(), "Overbought");

        // Reset and set up for oversold - same range, close near low
        wr.reset();
        for _ in 1..=20 {
            wr.update_bar(0.0, 110.0, 90.0, 91.0, 0.0);
        }
        assert_eq!(wr.market_condition(), "Oversold");
    }

    #[test]
    fn test_williams_r_reset() {
        let mut wr = WilliamsR::new(14);

        for i in 1..=20 {
            let base = 100.0 + i as f64;
            wr.update_bar(0.0, base + 2.0, base - 2.0, base, 0.0);
        }
        assert!(wr.is_ready());

        wr.reset();
        assert!(!wr.is_ready());
        assert!((wr.value().main() - (-50.0)).abs() < 0.01);
    }

    #[test]
    fn test_williams_r_period_getter() {
        let wr = WilliamsR::new(14);
        assert_eq!(wr.period(), 14);
    }

    #[test]
    fn test_williams_r_at_exact_high() {
        let mut wr = WilliamsR::new(5);

        // Fill buffer with same range
        for _ in 0..5 {
            wr.update_bar(0.0, 110.0, 90.0, 100.0, 0.0);
        }

        // Close at exact high
        wr.update_bar(0.0, 110.0, 90.0, 110.0, 0.0);
        assert!(wr.value().main() > -1.0, "Williams %R at exact high should be ~0, got {}", wr.value().main());
    }

    #[test]
    fn test_williams_r_at_exact_low() {
        let mut wr = WilliamsR::new(5);

        // Fill buffer with same range
        for _ in 0..5 {
            wr.update_bar(0.0, 110.0, 90.0, 100.0, 0.0);
        }

        // Close at exact low
        wr.update_bar(0.0, 110.0, 90.0, 90.0, 0.0);
        assert!(wr.value().main() < -99.0, "Williams %R at exact low should be ~-100, got {}", wr.value().main());
    }
}
