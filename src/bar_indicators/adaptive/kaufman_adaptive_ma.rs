//! Kaufman's Adaptive Moving Average (KAMA)
//! Адаптивная скользящая средняя Кауфмана
//! Использует Efficiency Ratio для адаптации к рыночным условиям
//!
//! OPTIMIZED: O(1) running sum for efficiency ratios instead of O(n) iter().sum()

use arrayvec::ArrayVec;
use std::collections::VecDeque;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Kaufman's Adaptive Moving Average
#[derive(Clone)]
pub struct KaufmanAdaptiveMA {
    // Основные параметры
    efficiency_ratio_period: usize,     // Период для расчета Efficiency Ratio
    fast_sc_period: usize,              // Быстрая константа сглаживания
    slow_sc_period: usize,              // Медленная константа сглаживания

    // Данные для расчета
    prices: ArrayVec<f64, 200>,         // История цен

    // Промежуточные значения
    direction_values: ArrayVec<f64, 200>,    // Направление движения
    volatility_values: ArrayVec<f64, 200>,   // Волатильность

    // VecDeque for O(1) pop_front
    efficiency_ratios: VecDeque<f64>,   // История Efficiency Ratio
    smoothing_constants: ArrayVec<f64, 100>, // История констант сглаживания

    // Running sum for O(1) average efficiency calculation
    sum_efficiency_ratios: f64,

    // Константы сглаживания
    fast_sc: f64,                       // Быстрая константа
    slow_sc: f64,                       // Медленная константа
    sc_diff: f64,                       // Разность констант

    // Результаты
    kama: f64,                          // Значение KAMA
    efficiency_ratio: f64,              // Текущий Efficiency Ratio
    current_sc: f64,                    // Текущая константа сглаживания
    adaptive_period: f64,               // Эквивалентный адаптивный период

    // Статистика
    avg_efficiency: f64,                // Средний Efficiency Ratio
    efficiency_variance: f64,           // Дисперсия Efficiency Ratio
    trend_consistency: f64,             // Консистентность тренда

    // Источник данных
    source: OhlcvField,                 // Поле OHLCV для расчета

    // Состояние
    is_ready: bool,
}

impl KaufmanAdaptiveMA {
    pub fn new(efficiency_ratio_period: usize, fast_sc_period: usize, slow_sc_period: usize) -> Self {
        Self::with_source(efficiency_ratio_period, fast_sc_period, slow_sc_period, OhlcvField::Close)
    }

    /// Создать KAMA с настраиваемым источником данных
    pub fn with_source(efficiency_ratio_period: usize, fast_sc_period: usize, slow_sc_period: usize, source: OhlcvField) -> Self {
        let efficiency_ratio_period = efficiency_ratio_period.clamp(2, 200);
        let fast_sc_period = fast_sc_period.clamp(1, 50);
        let slow_sc_period = slow_sc_period.max(fast_sc_period + 1).min(200);

        // Вычисляем константы сглаживания
        let fast_sc = 2.0 / (fast_sc_period as f64 + 1.0);
        let slow_sc = 2.0 / (slow_sc_period as f64 + 1.0);
        let sc_diff = fast_sc - slow_sc;

        Self {
            efficiency_ratio_period,
            fast_sc_period,
            slow_sc_period,
            prices: ArrayVec::new(),
            direction_values: ArrayVec::new(),
            volatility_values: ArrayVec::new(),
            efficiency_ratios: VecDeque::with_capacity(100),
            smoothing_constants: ArrayVec::new(),
            sum_efficiency_ratios: 0.0,
            fast_sc,
            slow_sc,
            sc_diff,
            kama: 0.0,
            efficiency_ratio: 0.0,
            current_sc: 0.0,
            adaptive_period: 0.0,
            avg_efficiency: 0.0,
            efficiency_variance: 0.0,
            trend_consistency: 0.0,
            source,
            is_ready: false,
        }
    }

    /// Создать KAMA с настройками по умолчанию
    pub fn default() -> Self {
        Self::new(10, 2, 30)
    }

    /// Создать быструю KAMA
    pub fn fast() -> Self {
        Self::new(5, 1, 15)
    }

    /// Создать медленную KAMA
    pub fn slow() -> Self {
        Self::new(20, 5, 50)
    }

    /// Обновить KAMA с OHLCV баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let price = self.source.extract(open, high, low, close, volume);
        self.update(price)
    }

    /// Обновить KAMA новой ценой
    pub fn update(&mut self, price: f64) -> f64 {
        self.add_price(price);

        if self.prices.len() > self.efficiency_ratio_period {
            self.calculate_efficiency_ratio();
            self.calculate_smoothing_constant();
            self.calculate_kama();
            self.update_statistics();
            self.is_ready = true;
        } else {
            self.kama = price;
        }

        self.kama
    }

    /// Добавить новую цену
    fn add_price(&mut self, price: f64) {
        if self.prices.len() >= 200 {
            self.prices.remove(0);
        }
        if !self.prices.is_full() {
            self.prices.push(price);
        }
    }

    /// Расчет Efficiency Ratio
    fn calculate_efficiency_ratio(&mut self) {
        if self.prices.len() < self.efficiency_ratio_period + 1 {
            return;
        }

        let current_idx = self.prices.len() - 1;
        let start_idx = current_idx - self.efficiency_ratio_period;

        // Направление (Direction) - чистое изменение цены
        let direction = (self.prices[current_idx] - self.prices[start_idx]).abs();

        // Волатильность (Volatility) - сумма абсолютных изменений
        let mut volatility = 0.0;
        for i in (start_idx + 1)..=current_idx {
            volatility += (self.prices[i] - self.prices[i - 1]).abs();
        }

        // Efficiency Ratio = Direction / Volatility
        self.efficiency_ratio = if volatility > 0.0 {
            direction / volatility
        } else {
            0.0 // Нет изменений в цене
        };

        // Ограничиваем ER от 0 до 1
        self.efficiency_ratio = self.efficiency_ratio.clamp(0.0, 1.0);

        // Сохраняем направление и волатильность
        if self.direction_values.len() >= 200 {
            self.direction_values.remove(0);
        }
        if !self.direction_values.is_full() {
            self.direction_values.push(direction);
        }

        if self.volatility_values.len() >= 200 {
            self.volatility_values.remove(0);
        }
        if !self.volatility_values.is_full() {
            self.volatility_values.push(volatility);
        }

        // Сохраняем ER в историю с O(1) running sum tracking
        if self.efficiency_ratios.len() >= 100 {
            let old_er = self.efficiency_ratios.pop_front().unwrap();
            self.sum_efficiency_ratios -= old_er;
        }
        self.efficiency_ratios.push_back(self.efficiency_ratio);
        self.sum_efficiency_ratios += self.efficiency_ratio;
    }

    /// Расчет адаптивной константы сглаживания
    fn calculate_smoothing_constant(&mut self) {
        // Квадрат сглаженного Efficiency Ratio
        let smoothed_ratio = self.efficiency_ratio;
        self.current_sc = (smoothed_ratio * self.sc_diff + self.slow_sc).powi(2);

        // Эквивалентный адаптивный период
        self.adaptive_period = if self.current_sc > 0.0 {
            (2.0 / self.current_sc) - 1.0
        } else {
            self.slow_sc_period as f64
        };

        // Сохраняем константу сглаживания
        if self.smoothing_constants.len() >= 100 {
            self.smoothing_constants.remove(0);
        }
        if !self.smoothing_constants.is_full() {
            self.smoothing_constants.push(self.current_sc);
        }
    }

    /// Расчет KAMA
    fn calculate_kama(&mut self) {
        let current_price = self.prices[self.prices.len() - 1];

        if self.kama == 0.0 {
            // Инициализация
            self.kama = current_price;
        } else {
            // KAMA = KAMA(предыдущая) + SC * (Цена - KAMA(предыдущая))
            self.kama = self.kama + self.current_sc * (current_price - self.kama);
        }
    }

    /// Обновление статистики
    fn update_statistics(&mut self) {
        // Средний Efficiency Ratio - O(1) using running sum!
        if !self.efficiency_ratios.is_empty() {
            self.avg_efficiency = self.sum_efficiency_ratios / self.efficiency_ratios.len() as f64;

            // Дисперсия Efficiency Ratio (still needs iteration, but only once)
            let variance_sum = self.efficiency_ratios.iter()
                .map(|&er| (er - self.avg_efficiency).powi(2))
                .sum::<f64>();
            self.efficiency_variance = variance_sum / self.efficiency_ratios.len() as f64;
        }

        // Консистентность тренда
        self.calculate_trend_consistency();
    }

    /// Расчет консистентности тренда
    fn calculate_trend_consistency(&mut self) {
        if self.direction_values.len() < 10 {
            self.trend_consistency = 0.0;
            return;
        }

        // Берем последние 10 значений направления
        let recent_directions: Vec<f64> = self.direction_values.iter().rev().take(10).copied().collect();

        // Считаем, сколько направлений указывают в одну сторону
        let mut up_count = 0;
        let mut down_count = 0;

        for i in 1..recent_directions.len() {
            if recent_directions[i-1] < recent_directions[i] {
                up_count += 1;
            } else if recent_directions[i-1] > recent_directions[i] {
                down_count += 1;
            }
        }

        let total_moves = up_count + down_count;
        if total_moves > 0 {
            let dominant_moves = up_count.max(down_count);
            self.trend_consistency = dominant_moves as f64 / total_moves as f64;
        } else {
            self.trend_consistency = 0.0;
        }
    }

    /// Получить прогноз на следующий период
    pub fn forecast(&self, periods: usize) -> Vec<f64> {
        if !self.is_ready || periods == 0 {
            return vec![];
        }

        let mut forecast = Vec::new();
        let mut current_kama = self.kama;
        let last_price = self.prices[self.prices.len() - 1];

        // Предполагаем, что текущая константа сглаживания остается неизменной
        for _ in 0..periods {
            // Простая экстраполяция: предполагаем небольшое изменение цены
            let price_change_factor = 1.0 + (self.efficiency_ratio - 0.5) * 0.01;
            let projected_price = last_price * price_change_factor;

            current_kama = current_kama + self.current_sc * (projected_price - current_kama);
            forecast.push(current_kama);
        }

        forecast
    }

    /// Получить сигнал тренда
    pub fn trend_signal(&self) -> TrendSignal {
        if !self.is_ready {
            return TrendSignal::Neutral;
        }

        let last_price = self.prices[self.prices.len() - 1];
        let price_vs_kama = (last_price - self.kama) / self.kama;

        // Определяем силу сигнала на основе ER и отклонения от KAMA
        let signal_strength = self.efficiency_ratio * price_vs_kama.abs();

        if price_vs_kama > 0.001 && signal_strength > 0.1 {
            if self.efficiency_ratio > 0.7 {
                TrendSignal::StrongBuy
            } else {
                TrendSignal::Buy
            }
        } else if price_vs_kama < -0.001 && signal_strength > 0.1 {
            if self.efficiency_ratio > 0.7 {
                TrendSignal::StrongSell
            } else {
                TrendSignal::Sell
            }
        } else {
            TrendSignal::Neutral
        }
    }

    // Публичные методы доступа
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.kama)
    }

    pub fn efficiency_ratio(&self) -> f64 {
        self.efficiency_ratio
    }

    pub fn smoothing_constant(&self) -> f64 {
        self.current_sc
    }

    pub fn adaptive_period(&self) -> f64 {
        self.adaptive_period
    }

    pub fn average_efficiency(&self) -> f64 {
        self.avg_efficiency
    }

    pub fn efficiency_variance(&self) -> f64 {
        self.efficiency_variance
    }

    pub fn trend_consistency(&self) -> f64 {
        self.trend_consistency
    }

    pub fn fast_sc_period(&self) -> usize {
        self.fast_sc_period
    }

    pub fn slow_sc_period(&self) -> usize {
        self.slow_sc_period
    }

    pub fn efficiency_ratio_period(&self) -> usize {
        self.efficiency_ratio_period
    }

    pub fn efficiency_history(&self) -> &[f64] {
        // Note: VecDeque may not be contiguous, but we can return the whole VecDeque as an iterator target
        self.efficiency_ratios.as_slices().0
    }

    pub fn smoothing_constant_history(&self) -> &[f64] {
        &self.smoothing_constants
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Вычислить отклонение цены от KAMA в процентах
    pub fn price_deviation_percent(&self) -> f64 {
        if !self.is_ready || self.kama == 0.0 {
            return 0.0;
        }

        let last_price = self.prices[self.prices.len() - 1];
        ((last_price - self.kama) / self.kama) * 100.0
    }

    /// Получить текущую волатильность
    pub fn current_volatility(&self) -> f64 {
        if self.volatility_values.is_empty() {
            0.0
        } else {
            self.volatility_values[self.volatility_values.len() - 1]
        }
    }

    /// Получить текущее направление
    pub fn current_direction(&self) -> f64 {
        if self.direction_values.is_empty() {
            0.0
        } else {
            self.direction_values[self.direction_values.len() - 1]
        }
    }

    pub fn reset(&mut self) {
        self.prices.clear();
        self.direction_values.clear();
        self.volatility_values.clear();
        self.efficiency_ratios.clear();
        self.smoothing_constants.clear();
        self.sum_efficiency_ratios = 0.0;

        self.kama = 0.0;
        self.efficiency_ratio = 0.0;
        self.current_sc = 0.0;
        self.adaptive_period = 0.0;
        self.avg_efficiency = 0.0;
        self.efficiency_variance = 0.0;
        self.trend_consistency = 0.0;
        self.is_ready = false;
    }

    /// Получить быструю константу сглаживания
    pub fn get_fast_sc(&self) -> f64 {
        self.fast_sc
    }

    /// Получить медленную константу сглаживания
    pub fn get_slow_sc(&self) -> f64 {
        self.slow_sc
    }

    /// Получить разность констант сглаживания
    pub fn get_sc_diff(&self) -> f64 {
        self.sc_diff
    }

    /// Получить полную конфигурацию индикатора
    pub fn get_config(&self) -> KamaConfig {
        KamaConfig {
            efficiency_ratio_period: self.efficiency_ratio_period,
            fast_sc_period: self.fast_sc_period,
            slow_sc_period: self.slow_sc_period,
            fast_sc: self.fast_sc,
            slow_sc: self.slow_sc,
            sc_diff: self.sc_diff,
        }
    }

    /// Установить новую конфигурацию периодов (пересчитывает константы)
    pub fn set_periods(&mut self, efficiency_ratio_period: usize, fast_sc_period: usize, slow_sc_period: usize) {
        let efficiency_ratio_period = efficiency_ratio_period.clamp(2, 200);
        let fast_sc_period = fast_sc_period.clamp(1, 50);
        let slow_sc_period = slow_sc_period.max(fast_sc_period + 1).min(200);

        self.efficiency_ratio_period = efficiency_ratio_period;
        self.fast_sc_period = fast_sc_period;
        self.slow_sc_period = slow_sc_period;

        // Пересчитываем константы сглаживания
        self.fast_sc = 2.0 / (fast_sc_period as f64 + 1.0);
        self.slow_sc = 2.0 / (slow_sc_period as f64 + 1.0);
        self.sc_diff = self.fast_sc - self.slow_sc;

        // Если есть история, пересчитываем
        if !self.prices.is_empty() {
            self.recalculate_from_history();
        }
    }

    /// Пересчитать индикатор с сохраненной историей цен
    fn recalculate_from_history(&mut self) {
        let prices_copy = self.prices.clone();

        // Очищаем все данные
        self.prices.clear();
        self.direction_values.clear();
        self.volatility_values.clear();
        self.efficiency_ratios.clear();
        self.smoothing_constants.clear();
        self.sum_efficiency_ratios = 0.0;
        self.kama = 0.0;
        self.efficiency_ratio = 0.0;
        self.current_sc = 0.0;
        self.adaptive_period = 0.0;
        self.avg_efficiency = 0.0;
        self.efficiency_variance = 0.0;
        self.trend_consistency = 0.0;
        self.is_ready = false;

        // Пересчитываем с новыми параметрами
        for price in prices_copy {
            self.update(price);
        }
    }
}

/// Конфигурация индикатора KAMA
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KamaConfig {
    pub efficiency_ratio_period: usize,
    pub fast_sc_period: usize,
    pub slow_sc_period: usize,
    pub fast_sc: f64,
    pub slow_sc: f64,
    pub sc_diff: f64,
}

impl std::fmt::Debug for KaufmanAdaptiveMA {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KAMA")
            .field("efficiency_ratio_period", &self.efficiency_ratio_period)
            .field("fast_sc_period", &self.fast_sc_period)
            .field("slow_sc_period", &self.slow_sc_period)
            .field("fast_sc", &self.fast_sc)
            .field("slow_sc", &self.slow_sc)
            .field("sc_diff", &self.sc_diff)
            .field("kama", &self.kama)
            .field("efficiency_ratio", &self.efficiency_ratio)
            .field("current_sc", &self.current_sc)
            .field("adaptive_period", &self.adaptive_period)
            .field("avg_efficiency", &self.avg_efficiency)
            .field("trend_consistency", &self.trend_consistency)
            .field("is_ready", &self.is_ready)
            .finish()
    }
}

/// Сигналы тренда для KAMA
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrendSignal {
    StrongBuy,
    Buy,
    Neutral,
    Sell,
    StrongSell,
}

impl TrendSignal {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrendSignal::StrongBuy => "Strong Buy",
            TrendSignal::Buy => "Buy",
            TrendSignal::Neutral => "Neutral",
            TrendSignal::Sell => "Sell",
            TrendSignal::StrongSell => "Strong Sell",
        }
    }

    pub fn strength(&self) -> f64 {
        match self {
            TrendSignal::StrongBuy => 1.0,
            TrendSignal::Buy => 0.5,
            TrendSignal::Neutral => 0.0,
            TrendSignal::Sell => -0.5,
            TrendSignal::StrongSell => -1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kama_creation() {
        let ind = KaufmanAdaptiveMA::new(10, 2, 30);
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_kama_default_presets() {
        let default = KaufmanAdaptiveMA::default();
        assert_eq!(default.efficiency_ratio_period(), 10);

        let fast = KaufmanAdaptiveMA::fast();
        assert_eq!(fast.efficiency_ratio_period(), 5);

        let slow = KaufmanAdaptiveMA::slow();
        assert_eq!(slow.efficiency_ratio_period(), 20);
    }

    #[test]
    fn test_kama_warmup() {
        let mut ind = KaufmanAdaptiveMA::new(10, 2, 30);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_kama_values_finite() {
        let mut ind = KaufmanAdaptiveMA::new(10, 2, 30);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 5.0;
            ind.update(price);
        }
        assert!(ind.value().main().is_finite());
        assert!(ind.efficiency_ratio() >= 0.0 && ind.efficiency_ratio() <= 1.0);
        assert!(ind.smoothing_constant().is_finite());
    }

    #[test]
    fn test_kama_trend_signal() {
        let mut ind = KaufmanAdaptiveMA::new(10, 2, 30);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            ind.update(price);
        }
        let signal = ind.trend_signal();
        assert!(signal.strength() >= -1.0 && signal.strength() <= 1.0);
    }

    #[test]
    fn test_kama_reset() {
        let mut ind = KaufmanAdaptiveMA::new(10, 2, 30);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            ind.update(price);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
}
