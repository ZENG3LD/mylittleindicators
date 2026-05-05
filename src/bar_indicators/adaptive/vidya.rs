//! Variable Index Dynamic Average (VIDYA)
//! Адаптивная скользящая средняя с динамическим индексом
//! Разработана Тушаром Чанде, адаптируется к волатильности с помощью CMO
//!
//! OPTIMIZED: O(1) running sum for CMO Simple MA mode

use arrayvec::ArrayVec;
use std::collections::VecDeque;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Тип скользящей средней для CMO
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CmoMaType {
    Simple,
    Exponential,
    Linear,
    Triangular,
}

/// Chande Momentum Oscillator для VIDYA
#[derive(Clone)]
pub struct ChandeMomentumOscillator {
    period: usize,
    ma_type: CmoMaType,
    prices: ArrayVec<f64, 500>,

    // VecDeque for O(1) pop_front
    gains: VecDeque<f64>,
    losses: VecDeque<f64>,

    // Running sums for O(1) Simple MA calculation
    sum_gains: f64,
    sum_losses: f64,

    avg_gain: f64,
    avg_loss: f64,
    value: f64,
    is_ready: bool,
}

impl ChandeMomentumOscillator {
    pub fn new(period: usize, ma_type: CmoMaType) -> Self {
        Self {
            period,
            ma_type,
            prices: ArrayVec::new(),
            gains: VecDeque::with_capacity(period),
            losses: VecDeque::with_capacity(period),
            sum_gains: 0.0,
            sum_losses: 0.0,
            avg_gain: 0.0,
            avg_loss: 0.0,
            value: 0.0,
            is_ready: false,
        }
    }

    pub fn update(&mut self, price: f64) {
        // Добавляем новую цену
        if self.prices.len() >= self.period {
            self.prices.remove(0);
        }
        if !self.prices.is_full() {
            self.prices.push(price);
        }

        if self.prices.len() < 2 {
            return;
        }

        // Вычисляем изменение цены
        let change = price - self.prices[self.prices.len() - 2];
        let gain = if change > 0.0 { change } else { 0.0 };
        let loss = if change < 0.0 { -change } else { 0.0 };

        // Добавляем в окна с O(1) running sum tracking
        if self.gains.len() >= self.period {
            let old_gain = self.gains.pop_front().unwrap();
            let old_loss = self.losses.pop_front().unwrap();
            self.sum_gains -= old_gain;
            self.sum_losses -= old_loss;
        }

        self.gains.push_back(gain);
        self.losses.push_back(loss);
        self.sum_gains += gain;
        self.sum_losses += loss;

        if self.gains.len() == self.period {
            // Вычисляем средние значения прибылей и убытков
            match self.ma_type {
                CmoMaType::Simple => {
                    // O(1) calculation using running sums!
                    self.avg_gain = self.sum_gains / self.period as f64;
                    self.avg_loss = self.sum_losses / self.period as f64;
                },
                CmoMaType::Exponential => {
                    let alpha = 2.0 / (self.period as f64 + 1.0);
                    self.avg_gain = alpha * gain + (1.0 - alpha) * self.avg_gain;
                    self.avg_loss = alpha * loss + (1.0 - alpha) * self.avg_loss;
                },
                CmoMaType::Linear => {
                    // Линейно взвешенная средняя
                    let mut weighted_gain = 0.0;
                    let mut weighted_loss = 0.0;
                    let mut weight_sum = 0.0;

                    for (i, &g) in self.gains.iter().enumerate() {
                        let weight = (i + 1) as f64;
                        weighted_gain += g * weight;
                        weight_sum += weight;
                    }

                    for (i, &l) in self.losses.iter().enumerate() {
                        let weight = (i + 1) as f64;
                        weighted_loss += l * weight;
                    }

                    self.avg_gain = weighted_gain / weight_sum;
                    self.avg_loss = weighted_loss / weight_sum;
                },
                CmoMaType::Triangular => {
                    // Треугольная взвешенная средняя
                    let mid = self.period / 2;
                    let mut weighted_gain = 0.0;
                    let mut weighted_loss = 0.0;
                    let mut weight_sum = 0.0;

                    for (i, &g) in self.gains.iter().enumerate() {
                        let weight = if i <= mid {
                            (i + 1) as f64
                        } else {
                            (self.period - i) as f64
                        };
                        weighted_gain += g * weight;
                        weight_sum += weight;
                    }

                    for (i, &l) in self.losses.iter().enumerate() {
                        let weight = if i <= mid {
                            (i + 1) as f64
                        } else {
                            (self.period - i) as f64
                        };
                        weighted_loss += l * weight;
                    }

                    self.avg_gain = weighted_gain / weight_sum;
                    self.avg_loss = weighted_loss / weight_sum;
                },
            }

            // Вычисляем CMO
            let total = self.avg_gain + self.avg_loss;
            if total > 1e-12 {
                self.value = 100.0 * (self.avg_gain - self.avg_loss) / total;
            } else {
                self.value = 0.0;
            }

            if !self.is_ready {
                self.is_ready = true;
            }
        }
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    pub fn reset(&mut self) {
        self.prices.clear();
        self.gains.clear();
        self.losses.clear();
        self.sum_gains = 0.0;
        self.sum_losses = 0.0;
        self.avg_gain = 0.0;
        self.avg_loss = 0.0;
        self.value = 0.0;
        self.is_ready = false;
    }
}

/// Результат VIDYA
#[derive(Debug, Clone)]
pub struct VidyaResult {
    pub value: f64,             // Текущее значение VIDYA
    pub cmo_value: f64,         // Значение CMO
    pub volatility_index: f64,  // Индекс волатильности (0-1)
    pub adaptation_rate: f64,   // Скорость адаптации
    pub efficiency_ratio: f64,  // Коэффициент эффективности
    pub trend_strength: f64,    // Сила тренда
}

impl Default for VidyaResult {
    fn default() -> Self {
        Self::new()
    }
}

impl VidyaResult {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            cmo_value: 0.0,
            volatility_index: 0.0,
            adaptation_rate: 0.0,
            efficiency_ratio: 0.0,
            trend_strength: 0.0,
        }
    }
}

/// Variable Index Dynamic Average
#[derive(Clone)]
pub struct VariableIndexDynamicAverage {
    // Параметры
    period: usize,
    cmo_ma_type: CmoMaType,

    // Компоненты
    cmo: ChandeMomentumOscillator,

    // Данные
    prices: ArrayVec<f64, 500>,

    // Параметры адаптации
    alpha: f64,                 // Базовый альфа-коэффициент
    min_alpha: f64,             // Минимальный альфа
    max_alpha: f64,             // Максимальный альфа

    // Результаты
    current_result: VidyaResult,

    // История для анализа
    volatility_history: ArrayVec<f64, 100>,
    adaptation_history: ArrayVec<f64, 100>,

    // Статистики
    trend_changes: usize,
    high_volatility_periods: usize,
    low_volatility_periods: usize,

    // Источник данных
    source: OhlcvField,

    // Состояние
    is_initialized: bool,
    last_trend_direction: i8, // -1: down, 0: sideways, 1: up
}

impl VariableIndexDynamicAverage {
    pub fn new(period: usize, cmo_ma_type: CmoMaType) -> Self {
        Self::with_source(period, cmo_ma_type, OhlcvField::Close)
    }

    /// Создать с настраиваемым источником данных
    pub fn with_source(period: usize, cmo_ma_type: CmoMaType, source: OhlcvField) -> Self {
        let alpha = 2.0 / (period as f64 + 1.0);

        Self {
            period,
            cmo_ma_type,
            cmo: ChandeMomentumOscillator::new(period, cmo_ma_type),
            prices: ArrayVec::new(),
            alpha,
            min_alpha: alpha * 0.1,   // 10% от базового альфа
            max_alpha: alpha * 3.0,   // 300% от базового альфа
            current_result: VidyaResult::new(),
            volatility_history: ArrayVec::new(),
            adaptation_history: ArrayVec::new(),
            trend_changes: 0,
            high_volatility_periods: 0,
            low_volatility_periods: 0,
            source,
            is_initialized: false,
            last_trend_direction: 0,
        }
    }

    /// Обновление VIDYA с OHLCV баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> &VidyaResult {
        let price = self.source.extract(open, high, low, close, volume);
        self.update(price)
    }

    /// Обновление VIDYA
    pub fn update(&mut self, price: f64) -> &VidyaResult {
        // Добавляем цену
        if self.prices.len() >= 500 {
            self.prices.remove(0);
        }
        if !self.prices.is_full() {
            self.prices.push(price);
        }

        // Обновляем CMO
        self.cmo.update(price);
        self.current_result.cmo_value = self.cmo.value().main();

        if !self.is_initialized {
            if self.cmo.is_ready() {
                self.current_result.value = price;
                self.is_initialized = true;
            } else {
                self.current_result.value = price;
                return &self.current_result;
            }
        }

        // Вычисляем индекс волатильности
        let cmo_abs = self.current_result.cmo_value.abs();
        self.current_result.volatility_index = cmo_abs / 100.0;

        // Адаптивный коэффициент
        let adaptive_alpha = self.calculate_adaptive_alpha();
        self.current_result.adaptation_rate = adaptive_alpha;

        // Обновляем VIDYA
        let prev_value = self.current_result.value;
        self.current_result.value = adaptive_alpha * price + (1.0 - adaptive_alpha) * prev_value;

        // Дополнительные аналитические метрики
        self.calculate_additional_metrics(price);

        // Сохраняем историю
        self.save_history();

        &self.current_result
    }

    /// Вычисление адаптивного альфа-коэффициента
    fn calculate_adaptive_alpha(&self) -> f64 {
        // Базовый метод: альфа пропорционален волатильности
        let volatility_factor = self.current_result.volatility_index;
        let adaptive_alpha = self.alpha * volatility_factor;

        // Ограничиваем диапазон
        adaptive_alpha.max(self.min_alpha).min(self.max_alpha)
    }

    /// Вычисление дополнительных метрик
    fn calculate_additional_metrics(&mut self, _current_price: f64) {
        if self.prices.len() < 10 {
            return;
        }

        // Коэффициент эффективности (аналог Kaufman's ER)
        let lookback = 10.min(self.prices.len());
        let start_idx = self.prices.len() - lookback;
        let end_idx = self.prices.len() - 1;

        let direction = (self.prices[end_idx] - self.prices[start_idx]).abs();
        let volatility: f64 = (start_idx..end_idx)
            .map(|i| (self.prices[i + 1] - self.prices[i]).abs())
            .sum();

        self.current_result.efficiency_ratio = if volatility > 1e-12 {
            direction / volatility
        } else {
            0.0
        };

        // Сила тренда на основе CMO и направления
        let cmo_strength = self.current_result.cmo_value.abs() / 100.0;
        let price_momentum = if self.prices.len() >= 5 {
            let recent_change = self.prices[self.prices.len() - 1] - self.prices[self.prices.len() - 5];
            recent_change.abs() / self.prices[self.prices.len() - 5]
        } else {
            0.0
        };

        self.current_result.trend_strength = (cmo_strength + price_momentum) / 2.0;

        // Отслеживание изменений тренда
        let current_trend = if self.current_result.cmo_value > 20.0 {
            1  // Восходящий тренд
        } else if self.current_result.cmo_value < -20.0 {
            -1 // Нисходящий тренд
        } else {
            0  // Боковой тренд
        };

        if current_trend != self.last_trend_direction {
            self.trend_changes += 1;
            self.last_trend_direction = current_trend;
        }

        // Классификация периодов волатильности
        if self.current_result.volatility_index > 0.6 {
            self.high_volatility_periods += 1;
        } else if self.current_result.volatility_index < 0.2 {
            self.low_volatility_periods += 1;
        }
    }

    /// Сохранение истории
    fn save_history(&mut self) {
        // Волатильность
        if self.volatility_history.len() >= 100 {
            self.volatility_history.remove(0);
        }
        if !self.volatility_history.is_full() {
            self.volatility_history.push(self.current_result.volatility_index);
        }

        // Адаптация
        if self.adaptation_history.len() >= 100 {
            self.adaptation_history.remove(0);
        }
        if !self.adaptation_history.is_full() {
            self.adaptation_history.push(self.current_result.adaptation_rate);
        }
    }

    // Публичные методы

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.value)
    }

    pub fn cmo_value(&self) -> f64 {
        self.current_result.cmo_value
    }

    pub fn volatility_index(&self) -> f64 {
        self.current_result.volatility_index
    }

    pub fn adaptation_rate(&self) -> f64 {
        self.current_result.adaptation_rate
    }

    pub fn efficiency_ratio(&self) -> f64 {
        self.current_result.efficiency_ratio
    }

    pub fn trend_strength(&self) -> f64 {
        self.current_result.trend_strength
    }

    pub fn period(&self) -> usize {
        self.period
    }

    pub fn is_ready(&self) -> bool {
        self.is_initialized
    }

    pub fn trend_changes(&self) -> usize {
        self.trend_changes
    }

    pub fn high_volatility_periods(&self) -> usize {
        self.high_volatility_periods
    }

    pub fn low_volatility_periods(&self) -> usize {
        self.low_volatility_periods
    }

    pub fn volatility_history(&self) -> &[f64] {
        &self.volatility_history
    }

    pub fn adaptation_history(&self) -> &[f64] {
        &self.adaptation_history
    }

    /// Установка пользовательских границ адаптации
    pub fn set_adaptation_bounds(&mut self, min_factor: f64, max_factor: f64) {
        self.min_alpha = self.alpha * min_factor.max(0.01);
        self.max_alpha = self.alpha * max_factor.min(10.0);
    }

    /// Получение текущих границ адаптации
    pub fn adaptation_bounds(&self) -> (f64, f64) {
        (self.min_alpha / self.alpha, self.max_alpha / self.alpha)
    }

    pub fn reset(&mut self) {
        self.prices.clear();
        self.cmo.reset();
        self.volatility_history.clear();
        self.adaptation_history.clear();
        self.current_result = VidyaResult::new();
        self.trend_changes = 0;
        self.high_volatility_periods = 0;
        self.low_volatility_periods = 0;
        self.is_initialized = false;
        self.last_trend_direction = 0;
    }

    /// Получить тип скользящей средней для CMO
    pub fn get_cmo_ma_type(&self) -> CmoMaType {
        self.cmo_ma_type
    }

    /// Установить новый тип скользящей средней для CMO (пересоздает CMO)
    pub fn set_cmo_ma_type(&mut self, new_cmo_ma_type: CmoMaType) {
        self.cmo_ma_type = new_cmo_ma_type;
        self.cmo = ChandeMomentumOscillator::new(self.period, self.cmo_ma_type);
        // Пересчитываем результаты если данные уже есть
        if !self.prices.is_empty() {
            self.recalculate_from_history();
        }
    }

    /// Получить полную конфигурацию индикатора
    pub fn get_config(&self) -> VidyaConfig {
        VidyaConfig {
            period: self.period,
            cmo_ma_type: self.cmo_ma_type,
            min_alpha_factor: self.min_alpha / self.alpha,
            max_alpha_factor: self.max_alpha / self.alpha,
        }
    }

    /// Установить новую конфигурацию
    pub fn set_config(&mut self, config: VidyaConfig) {
        let cmo_changed = config.cmo_ma_type != self.cmo_ma_type || config.period != self.period;

        self.period = config.period;
        self.cmo_ma_type = config.cmo_ma_type;
        self.alpha = 2.0 / (self.period as f64 + 1.0);
        self.min_alpha = self.alpha * config.min_alpha_factor;
        self.max_alpha = self.alpha * config.max_alpha_factor;

        if cmo_changed {
            self.cmo = ChandeMomentumOscillator::new(self.period, self.cmo_ma_type);
            if !self.prices.is_empty() {
                self.recalculate_from_history();
            }
        }
    }

    /// Пересчитать индикатор с сохраненной историей цен
    fn recalculate_from_history(&mut self) {
        let prices_copy = self.prices.clone();
        self.prices.clear();
        self.cmo.reset();
        self.volatility_history.clear();
        self.adaptation_history.clear();
        self.current_result = VidyaResult::new();
        self.trend_changes = 0;
        self.high_volatility_periods = 0;
        self.low_volatility_periods = 0;
        self.is_initialized = false;
        self.last_trend_direction = 0;

        for price in prices_copy {
            self.update(price);
        }
    }

    /// Получить текущую настройку границ адаптации
    pub fn get_adaptation_bounds(&self) -> (f64, f64) {
        (self.min_alpha / self.alpha, self.max_alpha / self.alpha)
    }

    /// Прогноз значений VIDYA на несколько периодов вперед
    pub fn forecast(&self, periods: usize) -> Vec<f64> {
        if !self.is_initialized || periods == 0 {
            return vec![];
        }

        let mut forecasts = Vec::with_capacity(periods);
        let mut current_value = self.current_result.value;

        // Используем последний адаптивный коэффициент для прогноза
        let last_alpha = self.current_result.adaptation_rate;

        // Простой экспоненциальный прогноз с текущим альфа
        let last_price = self.prices[self.prices.len() - 1];
        let trend = current_value - last_price;

        for i in 0..periods {
            let expected_price = last_price + trend * (i + 1) as f64 * 0.1; // Простое продолжение тренда
            current_value = current_value + last_alpha * (expected_price - current_value);
            forecasts.push(current_value);
        }

        forecasts
    }
}

/// Конфигурация индикатора VIDYA
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VidyaConfig {
    pub period: usize,
    pub cmo_ma_type: CmoMaType,
    pub min_alpha_factor: f64,  // Множитель для минимального альфа (относительно базового)
    pub max_alpha_factor: f64,  // Множитель для максимального альфа (относительно базового)
}

impl std::fmt::Debug for VariableIndexDynamicAverage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VIDYA")
            .field("period", &self.period)
            .field("cmo_ma_type", &self.cmo_ma_type)
            .field("value", &self.current_result.value)
            .field("cmo_value", &self.current_result.cmo_value)
            .field("volatility_index", &self.current_result.volatility_index)
            .field("adaptation_rate", &self.current_result.adaptation_rate)
            .field("efficiency_ratio", &self.current_result.efficiency_ratio)
            .field("trend_strength", &self.current_result.trend_strength)
            .field("is_initialized", &self.is_initialized)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmo_creation() {
        let ind = ChandeMomentumOscillator::new(14, CmoMaType::Simple);
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_cmo_warmup() {
        let mut ind = ChandeMomentumOscillator::new(10, CmoMaType::Simple);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_cmo_ma_types() {
        let types = [CmoMaType::Simple, CmoMaType::Exponential, CmoMaType::Linear, CmoMaType::Triangular];
        for ma_type in types {
            let mut ind = ChandeMomentumOscillator::new(10, ma_type);
            for i in 0..20 {
                let price = 100.0 + i as f64;
                ind.update(price);
            }
            assert!(ind.is_ready());
            assert!(ind.value().main().is_finite());
        }
    }

    #[test]
    fn test_vidya_creation() {
        let ind = VariableIndexDynamicAverage::new(14, CmoMaType::Simple);
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_vidya_warmup() {
        let mut ind = VariableIndexDynamicAverage::new(10, CmoMaType::Simple);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_vidya_values_finite() {
        let mut ind = VariableIndexDynamicAverage::new(10, CmoMaType::Exponential);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 5.0;
            ind.update(price);
        }
        assert!(ind.value().main().is_finite());
        assert!(ind.cmo_value().is_finite());
        assert!(ind.volatility_index() >= 0.0 && ind.volatility_index() <= 1.0);
    }

    #[test]
    fn test_vidya_reset() {
        let mut ind = VariableIndexDynamicAverage::new(10, CmoMaType::Simple);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            ind.update(price);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
}
