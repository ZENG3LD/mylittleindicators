//! Adaptive Moving Average
//! Адаптивная скользящая средняя с различными режимами адаптации
//! Автоматически подстраивается под рыночные условия

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Режимы адаптации
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AdaptationMode {
    Volatility,     // Адаптация по волатильности
    Volume,         // Адаптация по объему
    Trend,          // Адаптация по силе тренда
    Momentum,       // Адаптация по моментуму
    Combined,       // Комбинированная адаптация
    Market,         // Адаптация по рыночным условиям
}

/// Метод расчета эффективности
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EfficiencyMethod {
    Kaufman,        // Efficiency Ratio Кауфмана
    Fractal,        // Фрактальная эффективность
    DirectionalMovement, // Направленное движение
    TrendStrength,  // Сила тренда
    NoiseRatio,     // Отношение сигнал/шум
}

/// Параметры адаптации
#[derive(Debug, Clone)]
pub struct AdaptationParameters {
    pub fast_period: f64,           // Быстрый период
    pub slow_period: f64,           // Медленный период
    pub efficiency_threshold: f64,   // Порог эффективности
    pub volatility_scaling: f64,     // Масштабирование по волатильности
    pub volume_scaling: f64,         // Масштабирование по объему
    pub trend_sensitivity: f64,      // Чувствительность к тренду
    pub noise_floor: f64,           // Минимальный уровень шума
}

impl Default for AdaptationParameters {
    fn default() -> Self {
        Self::new()
    }
}

impl AdaptationParameters {
    pub fn new() -> Self {
        Self {
            fast_period: 2.0,
            slow_period: 30.0,
            efficiency_threshold: 0.5,
            volatility_scaling: 1.0,
            volume_scaling: 1.0,
            trend_sensitivity: 1.0,
            noise_floor: 0.1,
        }
    }
    
    pub fn conservative() -> Self {
        Self {
            fast_period: 5.0,
            slow_period: 50.0,
            efficiency_threshold: 0.7,
            volatility_scaling: 0.5,
            volume_scaling: 0.5,
            trend_sensitivity: 0.8,
            noise_floor: 0.2,
        }
    }
    
    pub fn aggressive() -> Self {
        Self {
            fast_period: 1.0,
            slow_period: 20.0,
            efficiency_threshold: 0.3,
            volatility_scaling: 2.0,
            volume_scaling: 2.0,
            trend_sensitivity: 1.5,
            noise_floor: 0.05,
        }
    }
}

/// Результат адаптации
#[derive(Debug, Clone)]
pub struct AdaptationResult {
    pub efficiency_ratio: f64,      // Коэффициент эффективности
    pub adaptive_period: f64,       // Адаптивный период
    pub smoothing_constant: f64,    // Константа сглаживания
    pub volatility_factor: f64,     // Фактор волатильности
    pub volume_factor: f64,         // Фактор объема
    pub trend_factor: f64,          // Фактор тренда
    pub noise_level: f64,           // Уровень шума
    pub market_regime: String,      // Рыночный режим
}

impl Default for AdaptationResult {
    fn default() -> Self {
        Self::new()
    }
}

impl AdaptationResult {
    pub fn new() -> Self {
        Self {
            efficiency_ratio: 0.0,
            adaptive_period: 10.0,
            smoothing_constant: 0.1,
            volatility_factor: 1.0,
            volume_factor: 1.0,
            trend_factor: 1.0,
            noise_level: 0.0,
            market_regime: "Unknown".to_string(),
        }
    }
}

/// Adaptive Moving Average
#[derive(Clone)]
pub struct AdaptiveMovingAverage {
    // Основные параметры
    period: usize,
    adaptation_mode: AdaptationMode,
    efficiency_method: EfficiencyMethod,
    adaptation_params: AdaptationParameters,

    // Данные для расчета
    prices: ArrayVec<f64, 200>,
    highs: ArrayVec<f64, 200>,
    lows: ArrayVec<f64, 200>,
    volumes: ArrayVec<f64, 200>,

    // Промежуточные расчеты
    price_changes: ArrayVec<f64, 200>,
    absolute_changes: ArrayVec<f64, 200>,
    volatility_values: ArrayVec<f64, 50>,
    efficiency_values: ArrayVec<f64, 50>,

    // Результаты
    adaptive_ma: f64,
    adaptation_result: AdaptationResult,

    // История адаптации
    period_history: ArrayVec<f64, 100>,
    efficiency_history: ArrayVec<f64, 100>,

    // Вспомогательные индикаторы
    atr: f64,                       // Average True Range
    momentum: f64,                  // Momentum
    trend_strength: f64,            // Сила тренда

    // Состояние
    is_ready: bool,
    min_samples: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_moving_average_creation() {
        let ind = AdaptiveMovingAverage::new(14, AdaptationMode::Volatility, EfficiencyMethod::Kaufman);
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_adaptive_moving_average_warmup() {
        let mut ind = AdaptiveMovingAverage::new(10, AdaptationMode::Combined, EfficiencyMethod::Kaufman);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(price, price + 1.0, price - 1.0, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_adaptive_moving_average_values_finite() {
        let mut ind = AdaptiveMovingAverage::new(10, AdaptationMode::Trend, EfficiencyMethod::TrendStrength);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 5.0;
            ind.update(price, price + 1.0, price - 1.0, 1000.0);
        }
        assert!(ind.value().main().is_finite());
        assert!(ind.efficiency_ratio().is_finite());
        assert!(ind.adaptive_period().is_finite());
    }

    #[test]
    fn test_adaptive_moving_average_modes() {
        let modes = [
            AdaptationMode::Volatility,
            AdaptationMode::Volume,
            AdaptationMode::Trend,
            AdaptationMode::Momentum,
            AdaptationMode::Combined,
            AdaptationMode::Market,
        ];
        for mode in modes {
            let mut ind = AdaptiveMovingAverage::new(10, mode, EfficiencyMethod::Kaufman);
            for i in 0..25 {
                let price = 100.0 + i as f64;
                ind.update(price, price + 1.0, price - 1.0, 1000.0);
            }
            assert!(ind.is_ready());
            assert!(ind.value().main().is_finite());
        }
    }

    #[test]
    fn test_adaptive_moving_average_reset() {
        let mut ind = AdaptiveMovingAverage::new(10, AdaptationMode::Volatility, EfficiencyMethod::Kaufman);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            ind.update(price, price + 1.0, price - 1.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
}

impl AdaptiveMovingAverage {
    pub fn new(
        period: usize, 
        adaptation_mode: AdaptationMode, 
        efficiency_method: EfficiencyMethod
    ) -> Self {
        let period = period.clamp(2, 200);
        
        Self {
            period,
            adaptation_mode,
            efficiency_method,
            adaptation_params: AdaptationParameters::new(),
            prices: ArrayVec::new(),
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            volumes: ArrayVec::new(),
            price_changes: ArrayVec::new(),
            absolute_changes: ArrayVec::new(),
            volatility_values: ArrayVec::new(),
            efficiency_values: ArrayVec::new(),
            adaptive_ma: 0.0,
            adaptation_result: AdaptationResult::new(),
            period_history: ArrayVec::new(),
            efficiency_history: ArrayVec::new(),
            atr: 0.0,
            momentum: 0.0,
            trend_strength: 0.0,
            is_ready: false,
            min_samples: period.max(10),
        }
    }
    
    /// Создать адаптивную MA с параметрами
    pub fn with_parameters(
        period: usize,
        adaptation_mode: AdaptationMode,
        efficiency_method: EfficiencyMethod,
        params: AdaptationParameters
    ) -> Self {
        let mut ama = Self::new(period, adaptation_mode, efficiency_method);
        ama.adaptation_params = params;
        ama
    }
    
    /// Обновить индикатор новыми данными
    pub fn update(&mut self, price: f64, high: f64, low: f64, volume: f64) -> f64 {
        self.add_data(price, high, low, volume);
        
        if self.prices.len() >= self.min_samples {
            self.calculate_efficiency();
            self.calculate_adaptation_factors();
            self.calculate_adaptive_period();
            self.calculate_adaptive_ma();
            self.update_statistics();
            self.is_ready = true;
        } else {
            self.adaptive_ma = price;
        }
        
        self.adaptive_ma
    }
    
    /// Добавить новые данные
    fn add_data(&mut self, price: f64, high: f64, low: f64, volume: f64) {
        // Добавляем цены
        if self.prices.len() >= 200 {
            self.prices.remove(0);
        }
        if !self.prices.is_full() {
            self.prices.push(price);
        }
        
        // Добавляем высокие цены
        if self.highs.len() >= 200 {
            self.highs.remove(0);
        }
        if !self.highs.is_full() {
            self.highs.push(high);
        }
        
        // Добавляем низкие цены
        if self.lows.len() >= 200 {
            self.lows.remove(0);
        }
        if !self.lows.is_full() {
            self.lows.push(low);
        }
        
        // Добавляем объемы
        if self.volumes.len() >= 200 {
            self.volumes.remove(0);
        }
        if !self.volumes.is_full() {
            self.volumes.push(volume);
        }
        
        // Вычисляем изменения цен
        if self.prices.len() >= 2 {
            let change = price - self.prices[self.prices.len() - 2];
            
            if self.price_changes.len() >= 200 {
                self.price_changes.remove(0);
            }
            if !self.price_changes.is_full() {
                self.price_changes.push(change);
            }
            
            if self.absolute_changes.len() >= 200 {
                self.absolute_changes.remove(0);
            }
            if !self.absolute_changes.is_full() {
                self.absolute_changes.push(change.abs());
            }
        }
    }
    
    /// Расчет коэффициента эффективности
    fn calculate_efficiency(&mut self) {
        if self.prices.len() < self.period {
            return;
        }
        
        let efficiency = match self.efficiency_method {
            EfficiencyMethod::Kaufman => self.calculate_kaufman_efficiency(),
            EfficiencyMethod::Fractal => self.calculate_fractal_efficiency(),
            EfficiencyMethod::DirectionalMovement => self.calculate_dm_efficiency(),
            EfficiencyMethod::TrendStrength => self.calculate_trend_strength(),
            EfficiencyMethod::NoiseRatio => self.calculate_noise_ratio(),
        };
        
        self.adaptation_result.efficiency_ratio = efficiency;
        
        // Сохраняем в историю
        if self.efficiency_history.len() >= 100 {
            self.efficiency_history.remove(0);
        }
        if !self.efficiency_history.is_full() {
            self.efficiency_history.push(efficiency);
        }
    }
    
    /// Efficiency Ratio Кауфмана
    fn calculate_kaufman_efficiency(&self) -> f64 {
        let start_idx = self.prices.len() - self.period;
        let direction = (self.prices[self.prices.len() - 1] - self.prices[start_idx]).abs();
        
        let mut volatility = 0.0;
        for i in (start_idx + 1)..self.prices.len() {
            volatility += (self.prices[i] - self.prices[i - 1]).abs();
        }
        
        if volatility > 0.0 {
            direction / volatility
        } else {
            0.0
        }
    }
    
    /// Фрактальная эффективность
    fn calculate_fractal_efficiency(&self) -> f64 {
        if self.prices.len() < self.period {
            return 0.0;
        }
        
        let start_idx = self.prices.len() - self.period;
        let direct_distance = (self.prices[self.prices.len() - 1] - self.prices[start_idx]).abs();
        
        let mut path_length = 0.0;
        for i in (start_idx + 1)..self.prices.len() {
            path_length += (self.prices[i] - self.prices[i - 1]).abs();
        }
        
        if path_length > 0.0 {
            direct_distance / path_length
        } else {
            0.0
        }
    }
    
    /// Эффективность направленного движения
    fn calculate_dm_efficiency(&self) -> f64 {
        if self.highs.len() < self.period || self.lows.len() < self.period {
            return 0.0;
        }
        
        let mut plus_dm = 0.0;
        let mut minus_dm = 0.0;
        let mut true_range = 0.0;
        
        let start_idx = self.highs.len() - self.period;
        for i in (start_idx + 1)..self.highs.len() {
            let high_diff = self.highs[i] - self.highs[i - 1];
            let low_diff = self.lows[i - 1] - self.lows[i];
            
            if high_diff > low_diff && high_diff > 0.0 {
                plus_dm += high_diff;
            }
            if low_diff > high_diff && low_diff > 0.0 {
                minus_dm += low_diff;
            }
            
            let tr = (self.highs[i] - self.lows[i])
                .max((self.highs[i] - self.prices[i - 1]).abs())
                .max((self.lows[i] - self.prices[i - 1]).abs());
            true_range += tr;
        }
        
        if true_range > 0.0 {
            (plus_dm - minus_dm).abs() / true_range
        } else {
            0.0
        }
    }
    
    /// Сила тренда
    fn calculate_trend_strength(&self) -> f64 {
        if self.prices.len() < self.period {
            return 0.0;
        }
        
        let start_idx = self.prices.len() - self.period;
        let mut trend_sum = 0.0;
        let mut counter_trend_sum = 0.0;
        
        // Определяем общее направление тренда
        let overall_trend = self.prices[self.prices.len() - 1] - self.prices[start_idx];
        let trend_direction = if overall_trend > 0.0 { 1.0 } else { -1.0 };
        
        for i in (start_idx + 1)..self.prices.len() {
            let change = self.prices[i] - self.prices[i - 1];
            if change * trend_direction > 0.0 {
                trend_sum += change.abs();
            } else {
                counter_trend_sum += change.abs();
            }
        }
        
        let total_movement = trend_sum + counter_trend_sum;
        if total_movement > 0.0 {
            trend_sum / total_movement
        } else {
            0.0
        }
    }
    
    /// Отношение сигнал/шум
    fn calculate_noise_ratio(&self) -> f64 {
        if self.price_changes.len() < self.period {
            return 0.0;
        }
        
        let start_idx = self.price_changes.len() - self.period;
        let signal = self.price_changes[start_idx..].iter().sum::<f64>().abs();
        let noise = self.price_changes[start_idx..].iter().map(|&x| x.abs()).sum::<f64>();
        
        if noise > 0.0 {
            signal / noise
        } else {
            0.0
        }
    }
    
    /// Расчет факторов адаптации
    fn calculate_adaptation_factors(&mut self) {
        // Волатильность
        self.calculate_volatility_factor();
        
        // Объем
        self.calculate_volume_factor();
        
        // Тренд
        self.calculate_trend_factor();
        
        // Уровень шума
        self.calculate_noise_level();
        
        // Определение рыночного режима
        self.determine_market_regime();
    }
    
    /// Фактор волатильности
    fn calculate_volatility_factor(&mut self) {
        if self.highs.len() < 14 || self.lows.len() < 14 {
            self.adaptation_result.volatility_factor = 1.0;
            return;
        }
        
        // Вычисляем ATR
        let mut atr = 0.0;
        let start_idx = self.highs.len() - 14;
        
        for i in (start_idx + 1)..self.highs.len() {
            let tr = (self.highs[i] - self.lows[i])
                .max((self.highs[i] - self.prices[i - 1]).abs())
                .max((self.lows[i] - self.prices[i - 1]).abs());
            atr += tr;
        }
        atr /= 13.0;
        self.atr = atr;
        
        // Нормализуем волатильность
        let current_price = self.prices[self.prices.len() - 1];
        let normalized_volatility = if current_price > 0.0 {
            atr / current_price
        } else {
            0.0
        };
        
        // Масштабируем фактор
        self.adaptation_result.volatility_factor = (1.0 + normalized_volatility * self.adaptation_params.volatility_scaling).clamp(0.1, 3.0);
    }
    
    /// Фактор объема
    fn calculate_volume_factor(&mut self) {
        if self.volumes.len() < 20 {
            self.adaptation_result.volume_factor = 1.0;
            return;
        }
        
        let recent_volume = self.volumes[self.volumes.len() - 1];
        let avg_volume = self.volumes.iter().rev().take(20).sum::<f64>() / 20.0;
        
        let volume_ratio = if avg_volume > 0.0 {
            recent_volume / avg_volume
        } else {
            1.0
        };
        
        self.adaptation_result.volume_factor = (1.0 + (volume_ratio - 1.0) * self.adaptation_params.volume_scaling).clamp(0.1, 3.0);
    }
    
    /// Фактор тренда
    fn calculate_trend_factor(&mut self) {
        if self.prices.len() < 20 {
            self.adaptation_result.trend_factor = 1.0;
            return;
        }
        
        // Простая регрессия для определения силы тренда
        let n = 20.min(self.prices.len());
        let start_idx = self.prices.len() - n;
        
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_x2 = 0.0;
        
        for (i, &price) in self.prices[start_idx..].iter().enumerate() {
            let x = i as f64;
            sum_x += x;
            sum_y += price;
            sum_xy += x * price;
            sum_x2 += x * x;
        }
        
        let n_f = n as f64;
        let slope = (n_f * sum_xy - sum_x * sum_y) / (n_f * sum_x2 - sum_x * sum_x);
        
        // Нормализуем наклон
        let avg_price = sum_y / n_f;
        let normalized_slope = if avg_price > 0.0 {
            slope / avg_price
        } else {
            0.0
        };
        
        self.trend_strength = normalized_slope.abs();
        self.adaptation_result.trend_factor = (1.0 + self.trend_strength * self.adaptation_params.trend_sensitivity).clamp(0.1, 2.0);
    }
    
    /// Уровень шума
    fn calculate_noise_level(&mut self) {
        if self.absolute_changes.len() < 10 {
            self.adaptation_result.noise_level = 0.0;
            return;
        }
        
        let recent_changes: Vec<f64> = self.absolute_changes.iter().rev().take(10).copied().collect();
        let avg_change = recent_changes.iter().sum::<f64>() / recent_changes.len() as f64;
        
        let variance = recent_changes.iter()
            .map(|&x| (x - avg_change).powi(2))
            .sum::<f64>() / (recent_changes.len() - 1) as f64;
        
        self.adaptation_result.noise_level = variance.sqrt().max(self.adaptation_params.noise_floor);
    }
    
    /// Определение рыночного режима
    fn determine_market_regime(&mut self) {
        let efficiency = self.adaptation_result.efficiency_ratio;
        let volatility = self.adaptation_result.volatility_factor;
        let trend = self.adaptation_result.trend_factor;
        
        self.adaptation_result.market_regime = if efficiency > 0.7 && trend > 1.3 {
            "Strong Trend".to_string()
        } else if efficiency > 0.5 && trend > 1.1 {
            "Trend".to_string()
        } else if volatility > 1.5 {
            "High Volatility".to_string()
        } else if efficiency < 0.3 {
            "Consolidation".to_string()
        } else {
            "Mixed".to_string()
        };
    }
    
    /// Расчет адаптивного периода
    fn calculate_adaptive_period(&mut self) {
        let base_efficiency = self.adaptation_result.efficiency_ratio;
        
        // Применяем факторы адаптации в зависимости от режима
        let efficiency = match self.adaptation_mode {
            AdaptationMode::Volatility => {
                base_efficiency * self.adaptation_result.volatility_factor
            },
            AdaptationMode::Volume => {
                base_efficiency * self.adaptation_result.volume_factor
            },
            AdaptationMode::Trend => {
                base_efficiency * self.adaptation_result.trend_factor
            },
            AdaptationMode::Momentum => {
                base_efficiency * (1.0 + self.momentum.abs())
            },
            AdaptationMode::Combined => {
                base_efficiency * 
                (self.adaptation_result.volatility_factor + 
                 self.adaptation_result.volume_factor + 
                 self.adaptation_result.trend_factor) / 3.0
            },
            AdaptationMode::Market => {
                // Адаптация по рыночному режиму
                match self.adaptation_result.market_regime.as_str() {
                    "Strong Trend" => base_efficiency * 1.5,
                    "Trend" => base_efficiency * 1.2,
                    "High Volatility" => base_efficiency * 0.8,
                    "Consolidation" => base_efficiency * 0.6,
                    _ => base_efficiency,
                }
            },
        };
        
        // Ограничиваем эффективность
        let bounded_efficiency = efficiency.clamp(0.01, 1.0);
        
        // Вычисляем адаптивный период
        let fast_alpha = 2.0 / (self.adaptation_params.fast_period + 1.0);
        let slow_alpha = 2.0 / (self.adaptation_params.slow_period + 1.0);
        let adaptive_alpha = slow_alpha + bounded_efficiency * (fast_alpha - slow_alpha);
        
        self.adaptation_result.smoothing_constant = adaptive_alpha;
        self.adaptation_result.adaptive_period = (2.0 / adaptive_alpha) - 1.0;
        
        // Сохраняем в историю
        if self.period_history.len() >= 100 {
            self.period_history.remove(0);
        }
        if !self.period_history.is_full() {
            self.period_history.push(self.adaptation_result.adaptive_period);
        }
    }
    
    /// Расчет адаптивной скользящей средней
    fn calculate_adaptive_ma(&mut self) {
        let current_price = self.prices[self.prices.len() - 1];
        let alpha = self.adaptation_result.smoothing_constant;
        
        if self.adaptive_ma == 0.0 {
            self.adaptive_ma = current_price;
        } else {
            self.adaptive_ma = alpha * current_price + (1.0 - alpha) * self.adaptive_ma;
        }
    }
    
    /// Обновление статистики
    fn update_statistics(&mut self) {
        // Вычисляем momentum
        if self.prices.len() >= 10 {
            let momentum_period = 10.min(self.prices.len());
            let start_idx = self.prices.len() - momentum_period;
            self.momentum = self.prices[self.prices.len() - 1] - self.prices[start_idx];
        }
        
        // Обновляем волатильность в истории
        if self.volatility_values.len() >= 50 {
            self.volatility_values.remove(0);
        }
        if !self.volatility_values.is_full() {
            self.volatility_values.push(self.atr);
        }
        
        // Обновляем эффективность в истории
        if self.efficiency_values.len() >= 50 {
            self.efficiency_values.remove(0);
        }
        if !self.efficiency_values.is_full() {
            self.efficiency_values.push(self.adaptation_result.efficiency_ratio);
        }
    }
    
    // Публичные методы доступа
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.adaptive_ma)
    }
    
    pub fn efficiency_ratio(&self) -> f64 {
        self.adaptation_result.efficiency_ratio
    }
    
    pub fn adaptive_period(&self) -> f64 {
        self.adaptation_result.adaptive_period
    }
    
    pub fn smoothing_constant(&self) -> f64 {
        self.adaptation_result.smoothing_constant
    }
    
    pub fn market_regime(&self) -> &str {
        &self.adaptation_result.market_regime
    }
    
    pub fn adaptation_result(&self) -> &AdaptationResult {
        &self.adaptation_result
    }
    
    pub fn volatility_factor(&self) -> f64 {
        self.adaptation_result.volatility_factor
    }
    
    pub fn volume_factor(&self) -> f64 {
        self.adaptation_result.volume_factor
    }
    
    pub fn trend_factor(&self) -> f64 {
        self.adaptation_result.trend_factor
    }
    
    pub fn atr(&self) -> f64 {
        self.atr
    }
    
    pub fn momentum(&self) -> f64 {
        self.momentum
    }
    
    pub fn trend_strength(&self) -> f64 {
        self.trend_strength
    }
    
    pub fn period_history(&self) -> &[f64] {
        &self.period_history
    }
    
    pub fn efficiency_history(&self) -> &[f64] {
        &self.efficiency_history
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    pub fn set_adaptation_parameters(&mut self, params: AdaptationParameters) {
        self.adaptation_params = params;
    }
    
    pub fn reset(&mut self) {
        self.prices.clear();
        self.highs.clear();
        self.lows.clear();
        self.volumes.clear();
        self.price_changes.clear();
        self.absolute_changes.clear();
        self.volatility_values.clear();
        self.efficiency_values.clear();
        self.adaptive_ma = 0.0;
        self.adaptation_result = AdaptationResult::new();
        self.period_history.clear();
        self.efficiency_history.clear();
        self.atr = 0.0;
        self.momentum = 0.0;
        self.trend_strength = 0.0;
        self.is_ready = false;
    }
} 






















