//! Adaptive Stochastic - адаптивный стохастик
//! 
//! Улучшенная версия классического стохастика, где период автоматически
//! адаптируется к рыночной волатильности на основе ATR.
//! 
//! Переиспользует существующие компоненты MovingAverage и ATR

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::volatility::atr::Atr;
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Результат Adaptive Stochastic
#[derive(Debug, Clone, Copy)]
pub struct AdaptiveStochasticResult {
    pub k_percent: f64,          // %K значение (0-100)
    pub d_percent: f64,          // %D значение (сглаженный %K) (0-100)
    pub adaptive_period: f64,    // Текущий адаптивный период
    pub volatility_factor: f64,  // Фактор волатильности (0-3+)
    pub momentum_strength: f64,  // Сила momentum (0-1)
    pub overbought_level: f64,   // Адаптивный уровень перекупленности
    pub oversold_level: f64,     // Адаптивный уровень перепроданности
    pub signal_strength: f64,    // Сила сигнала (0-1)
    pub trend_bias: i8,          // Смещение тренда: 1 (вверх), -1 (вниз), 0 (нейтрально)
}

impl AdaptiveStochasticResult {
    pub fn empty() -> Self {
        Self {
            k_percent: 50.0,
            d_percent: 50.0,
            adaptive_period: 14.0,
            volatility_factor: 1.0,
            momentum_strength: 0.5,
            overbought_level: 80.0,
            oversold_level: 20.0,
            signal_strength: 0.0,
            trend_bias: 0,
        }
    }
    
    /// Определить состояние стохастика
    pub fn stochastic_state(&self) -> &'static str {
        if self.k_percent >= self.overbought_level {
            "Перекуплен"
        } else if self.k_percent <= self.oversold_level {
            "Перепродан"
        } else if self.k_percent > 50.0 {
            "Выше середины"
        } else {
            "Ниже середины"
        }
    }
    
    /// Получить описание силы сигнала
    pub fn signal_strength_description(&self) -> &'static str {
        match self.signal_strength {
            x if x < 0.2 => "Очень слабый",
            x if x < 0.4 => "Слабый",
            x if x < 0.6 => "Умеренный",
            x if x < 0.8 => "Сильный",
            _ => "Очень сильный",
        }
    }
    
    /// Получить описание смещения тренда
    pub fn trend_bias_description(&self) -> &'static str {
        match self.trend_bias {
            1 => "Бычье смещение",
            -1 => "Медвежье смещение",
            _ => "Нейтральное",
        }
    }
}

/// Adaptive Stochastic индикатор
#[derive(Clone)]
pub struct AdaptiveStochastic {
    // Переиспользуем существующие компоненты
    atr: Atr,                        // ATR для анализа волатильности
    volatility_ma: MovingAverageProvider,    // MA для сглаживания волатильности
    d_ma: MovingAverageProvider,             // MA для %D линии
    momentum_ma: MovingAverageProvider,      // MA для анализа momentum
    
    // Буферы для расчетов
    highs: ArrayVec<f64, 64>,
    lows: ArrayVec<f64, 64>,
    closes: ArrayVec<f64, 64>,
    k_values: ArrayVec<f64, 32>,     // История %K значений
    periods: ArrayVec<f64, 16>,      // История адаптивных периодов
    
    // Параметры адаптации
    base_period: usize,              // Базовый период
    min_period: usize,               // Минимальный период
    max_period: usize,               // Максимальный период
    d_period: usize,                 // Период для %D сглаживания
    volatility_sensitivity: f64,     // Чувствительность к волатильности
    
    // Текущие значения
    current_period: f64,
    
    // Результат
    current_result: AdaptiveStochasticResult,
    
    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl AdaptiveStochastic {
    /// Создать новый Adaptive Stochastic с параметрами по умолчанию
    pub fn new() -> Self {
        Self::with_parameters(14, 7, 28, 3, 1.5)
    }
    
    /// Создать с настраиваемыми параметрами
    pub fn with_parameters(
        base_period: usize,
        min_period: usize,
        max_period: usize,
        d_period: usize,
        volatility_sensitivity: f64
    ) -> Self {
        assert!(base_period > 0, "Base period must be greater than 0");
        assert!(min_period > 0 && min_period <= base_period, "Invalid min period");
        assert!(max_period >= base_period, "Invalid max period");
        assert!(d_period > 0, "D period must be greater than 0");
        assert!(volatility_sensitivity > 0.0, "Volatility sensitivity must be positive");
        
        Self {
            // Переиспользуем существующие компоненты
            atr: Atr::new_wilder(14),
            volatility_ma: MovingAverageProvider::new(MovingAverageType::EMA, 10),
            d_ma: MovingAverageProvider::new(MovingAverageType::SMA, d_period),
            momentum_ma: MovingAverageProvider::new(MovingAverageType::EMA, 5),
            
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            closes: ArrayVec::new(),
            k_values: ArrayVec::new(),
            periods: ArrayVec::new(),
            
            base_period,
            min_period,
            max_period,
            d_period,
            volatility_sensitivity,
            
            current_period: base_period as f64,
            
            current_result: AdaptiveStochasticResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> AdaptiveStochasticResult {
        // Добавляем данные в буферы
        if self.highs.len() >= 64 {
            self.highs.remove(0);
        }
        self.highs.push(high);
        
        if self.lows.len() >= 64 {
            self.lows.remove(0);
        }
        self.lows.push(low);
        
        if self.closes.len() >= 64 {
            self.closes.remove(0);
        }
        self.closes.push(close);
        
        // 1. Обновляем ATR (переиспользуем существующий компонент)
        let atr_value = self.atr.update_bar(open, high, low, close, volume);
        
        // 2. Адаптируем период на основе волатильности
        self.adapt_period(atr_value, close);
        
        // 3. Рассчитываем %K с адаптивным периодом
        let k_percent = self.calculate_adaptive_k_percent(high, low, close);
        
        // 4. Рассчитываем %D (сглаженный %K)
        let d_percent = self.d_ma.update_bar(0.0, 0.0, 0.0, k_percent, 0.0);
        
        // 5. Анализируем momentum и силу сигнала
        self.analyze_momentum_and_signals(k_percent, d_percent);
        
        // 6. Адаптируем уровни перекупленности/перепроданности
        self.adapt_levels();
        
        // Обновляем результат
        self.current_result.k_percent = k_percent;
        self.current_result.d_percent = d_percent;
        self.current_result.adaptive_period = self.current_period;
        
        // Готов после накопления достаточных данных
        if self.atr.is_ready() && self.highs.len() >= self.base_period {
            self.is_ready = true;
        }
        
        self.update_count += 1;
        self.current_result
    }
    
    /// Адаптировать период на основе волатильности
    fn adapt_period(&mut self, atr_value: f64, current_price: f64) {
        if self.update_count < 15 {
            return; // Недостаточно данных для адаптации
        }
        
        // Сглаживаем волатильность
        let smoothed_volatility = self.volatility_ma.update_bar(0.0, 0.0, 0.0, atr_value, 0.0);
        
        // Нормализуем волатильность относительно цены
        let normalized_volatility = if current_price > 0.0 {
            smoothed_volatility / current_price
        } else {
            0.01
        };
        
        // Фактор волатильности
        let volatility_factor = (normalized_volatility * 100.0 * self.volatility_sensitivity).max(0.1);
        self.current_result.volatility_factor = volatility_factor;
        
        // Адаптируем период: высокая волатильность = короткий период
        let period_adjustment = 1.0 / (1.0 + volatility_factor);
        self.current_period = (self.base_period as f64 * period_adjustment)
            .max(self.min_period as f64)
            .min(self.max_period as f64);
        
        // Сохраняем период для анализа
        if self.periods.len() >= 16 {
            self.periods.remove(0);
        }
        self.periods.push(self.current_period);
    }
    
    /// Рассчитать адаптивный %K
    fn calculate_adaptive_k_percent(&mut self, current_high: f64, current_low: f64, current_close: f64) -> f64 {
        let period = self.current_period as usize;
        let available_data = self.highs.len().min(period);
        
        if available_data < 2 {
            return 50.0; // Нейтральное значение
        }
        
        let start_idx = self.highs.len() - available_data;
        
        // Находим максимум и минимум за адаптивный период
        let highest_high = self.highs[start_idx..].iter()
            .fold(current_high, |acc, &h| acc.max(h));
        let lowest_low = self.lows[start_idx..].iter()
            .fold(current_low, |acc, &l| acc.min(l));
        
        // Рассчитываем %K
        let k_percent = if highest_high != lowest_low {
            ((current_close - lowest_low) / (highest_high - lowest_low)) * 100.0
        } else {
            50.0
        };
        
        // Сохраняем %K значения
        if self.k_values.len() >= 32 {
            self.k_values.remove(0);
        }
        self.k_values.push(k_percent);
        
        k_percent
    }
    
    /// Анализировать momentum и силу сигналов
    fn analyze_momentum_and_signals(&mut self, k_percent: f64, d_percent: f64) {
        if self.k_values.len() < 3 {
            return;
        }
        
        let len = self.k_values.len();
        let current_k = self.k_values[len - 1];
        let prev_k = self.k_values[len - 2];
        let prev2_k = self.k_values[len - 3];
        
        // Momentum анализ
        let momentum = current_k - prev_k;
        let smoothed_momentum = self.momentum_ma.update_bar(0.0, 0.0, 0.0, momentum, 0.0);
        
        // Сила momentum
        let momentum_strength = (smoothed_momentum.abs() / 10.0).min(1.0);
        self.current_result.momentum_strength = momentum_strength;
        
        // Определяем смещение тренда
        let trend_momentum = current_k - prev2_k;
        self.current_result.trend_bias = if trend_momentum > 2.0 {
            1  // Бычье смещение
        } else if trend_momentum < -2.0 {
            -1 // Медвежье смещение
        } else {
            0  // Нейтральное
        };
        
        // Сила сигнала
        self.calculate_signal_strength(k_percent, d_percent, momentum_strength);
    }
    
    /// Рассчитать силу сигнала
    fn calculate_signal_strength(&mut self, k_percent: f64, d_percent: f64, momentum_strength: f64) {
        // Сила сигнала зависит от:
        // 1. Экстремальности значений
        let extremity = if k_percent <= 20.0 || k_percent >= 80.0 {
            1.0
        } else if k_percent <= 30.0 || k_percent >= 70.0 {
            0.7
        } else {
            0.3
        };
        
        // 2. Дивергенции между %K и %D
        let divergence = (k_percent - d_percent).abs() / 20.0;
        let divergence_factor = divergence.min(1.0);
        
        // 3. Силы momentum
        let momentum_factor = momentum_strength;
        
        // 4. Стабильности адаптивного периода
        let period_stability = if self.periods.len() >= 3 {
            let recent_periods = &self.periods[self.periods.len() - 3..];
            let period_variance = recent_periods.iter()
                .map(|&p| (p - self.current_period).abs())
                .sum::<f64>() / recent_periods.len() as f64;
            (1.0 - (period_variance / self.current_period).min(1.0)).max(0.0)
        } else {
            0.5
        };
        
        // Комбинируем факторы
        self.current_result.signal_strength = (extremity * 0.3 + 
                                              divergence_factor * 0.2 + 
                                              momentum_factor * 0.3 + 
                                              period_stability * 0.2).min(1.0);
    }
    
    /// Адаптировать уровни перекупленности/перепроданности
    fn adapt_levels(&mut self) {
        let volatility_factor = self.current_result.volatility_factor;
        
        // При высокой волатильности уровни становятся более экстремальными
        if volatility_factor > 2.0 {
            self.current_result.overbought_level = 85.0;
            self.current_result.oversold_level = 15.0;
        } else if volatility_factor > 1.5 {
            self.current_result.overbought_level = 82.0;
            self.current_result.oversold_level = 18.0;
        } else if volatility_factor < 0.5 {
            self.current_result.overbought_level = 75.0;
            self.current_result.oversold_level = 25.0;
        } else {
            self.current_result.overbought_level = 80.0;
            self.current_result.oversold_level = 20.0;
        }
    }
    
    /// Получить текущее значение %K и %D
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.current_result.k_percent, self.current_result.d_percent)
    }
    
    /// Получить полный результат
    pub fn result(&self) -> AdaptiveStochasticResult {
        self.current_result
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.atr.reset();
        self.volatility_ma.reset();
        self.d_ma.reset();
        self.momentum_ma.reset();
        
        self.highs.clear();
        self.lows.clear();
        self.closes.clear();
        self.k_values.clear();
        self.periods.clear();
        
        self.current_period = self.base_period as f64;
        
        self.current_result = AdaptiveStochasticResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.current_period as usize
    }
    
    /// Генерировать торговый сигнал
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let result = self.current_result;
        
        // Сигналы только при достаточной силе
        if result.signal_strength < 0.5 {
            return 0;
        }
        
        // Пересечения с адаптивными уровнями
        if result.k_percent <= result.oversold_level && result.d_percent <= result.oversold_level {
            if result.trend_bias >= 0 {
                return 1; // Покупка в перепроданности
            }
        } else if result.k_percent >= result.overbought_level && result.d_percent >= result.overbought_level
            && result.trend_bias <= 0 {
                return -1; // Продажа в перекупленности
            }
        
        0
    }
    
    /// Генерировать сигнал пересечения %K и %D
    pub fn crossover_signal(&self) -> i8 {
        if !self.is_ready || self.k_values.len() < 2 {
            return 0;
        }
        
        let current_k = self.current_result.k_percent;
        let current_d = self.current_result.d_percent;
        let len = self.k_values.len();
        let prev_k = self.k_values[len - 2];
        
        // Приблизительное предыдущее %D (так как у нас нет истории %D)
        let prev_d = current_d; // Упрощение
        
        // Сигналы только при высокой силе
        if self.current_result.signal_strength > 0.6 {
            if prev_k <= prev_d && current_k > current_d {
                return 1; // Пересечение %K выше %D
            } else if prev_k >= prev_d && current_k < current_d {
                return -1; // Пересечение %K ниже %D
            }
        }
        
        0
    }
    
    /// Генерировать сигнал дивергенции
    pub fn divergence_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let k_percent = self.current_result.k_percent;
        let d_percent = self.current_result.d_percent;
        let divergence = k_percent - d_percent;
        
        // Значительная дивергенция при высокой силе сигнала
        if self.current_result.signal_strength > 0.7 {
            if divergence > 15.0 {
                return 1; // %K значительно выше %D
            } else if divergence < -15.0 {
                return -1; // %K значительно ниже %D
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
            "Adaptive Stochastic: %K: {:.1}, %D: {:.1}, Период: {:.0}, Состояние: {}, {}, Сила: {} ({:.2}), Сигнал: {}",
            result.k_percent,
            result.d_percent,
            result.adaptive_period,
            result.stochastic_state(),
            result.trend_bias_description(),
            result.signal_strength_description(),
            result.signal_strength,
            signal
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("k_percent".to_string(), self.current_result.k_percent);
        values.insert("d_percent".to_string(), self.current_result.d_percent);
        values.insert("adaptive_period".to_string(), self.current_result.adaptive_period);
        values.insert("volatility_factor".to_string(), self.current_result.volatility_factor);
        values.insert("momentum_strength".to_string(), self.current_result.momentum_strength);
        values.insert("overbought_level".to_string(), self.current_result.overbought_level);
        values.insert("oversold_level".to_string(), self.current_result.oversold_level);
        values.insert("signal_strength".to_string(), self.current_result.signal_strength);
        values.insert("trend_bias".to_string(), self.current_result.trend_bias as f64);
        values
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    
    /// Получить параметры
    pub fn parameters(&self) -> (usize, usize, usize, usize, f64) {
        (self.base_period, self.min_period, self.max_period, self.d_period, self.volatility_sensitivity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_stochastic_creation() {
        let adaptive_stoch = AdaptiveStochastic::new();
        assert!(!adaptive_stoch.is_ready());
        assert_eq!(adaptive_stoch.parameters().0, 14);
    }
    
    #[test]
    fn test_adaptive_stochastic_with_parameters() {
        let adaptive_stoch = AdaptiveStochastic::with_parameters(21, 10, 35, 5, 2.0);
        assert_eq!(adaptive_stoch.parameters(), (21, 10, 35, 5, 2.0));
    }
    
    #[test]
    fn test_period_adaptation() {
        let mut adaptive_stoch = AdaptiveStochastic::new();
        
        // Период низкой волатильности
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.01);
            let _result = adaptive_stoch.update_bar(price, price + 0.01, price - 0.01, price, 1000.0);
        }
        let low_vol_period = adaptive_stoch.current_period;
        
        // Период высокой волатильности
        for i in 15..30 {
            let base_price = 100.0;
            let high_vol = 5.0;
            let price = base_price + (i as f64 * 0.1);
            let _result = adaptive_stoch.update_bar(
                price, 
                price + high_vol, 
                price - high_vol, 
                price, 
                1000.0
            );
        }
        let high_vol_period = adaptive_stoch.current_period;
        
        if adaptive_stoch.is_ready() {
            // При высокой волатильности период должен уменьшиться
            assert!(high_vol_period <= low_vol_period);
        }
    }
    
    #[test]
    fn test_adaptive_stochastic_calculation() {
        let mut adaptive_stoch = AdaptiveStochastic::new();
        
        // Добавляем данные с четким паттерном
        for i in 0..20 {
            let base = 100.0;
            let amplitude = 10.0;
            let phase = i as f64 * 0.3;
            
            let price = base + amplitude * phase.sin();
            let high = price + 1.0;
            let low = price - 1.0;
            
            let result = adaptive_stoch.update_bar(price, high, low, price, 1000.0);
            
            if i > 15 {
                assert!(adaptive_stoch.is_ready());
                assert!(result.k_percent >= 0.0 && result.k_percent <= 100.0);
                assert!(result.d_percent >= 0.0 && result.d_percent <= 100.0);
                assert!(result.signal_strength >= 0.0 && result.signal_strength <= 1.0);
                assert!(result.adaptive_period >= adaptive_stoch.min_period as f64);
                assert!(result.adaptive_period <= adaptive_stoch.max_period as f64);
            }
        }
    }
    
    #[test]
    fn test_trading_signals() {
        let mut adaptive_stoch = AdaptiveStochastic::new();
        
        // Создаем условия для перепроданности
        for i in 0..20 {
            let price = 100.0 - i as f64 * 2.0; // Падающие цены
            let _result = adaptive_stoch.update_bar(price, price + 0.5, price - 0.5, price, 1000.0);

            if i > 15 && adaptive_stoch.is_ready() {
                let signal = adaptive_stoch.trading_signal();
                let crossover = adaptive_stoch.crossover_signal();
                let divergence = adaptive_stoch.divergence_signal();
                
                assert!(signal >= -1 && signal <= 1);
                assert!(crossover >= -1 && crossover <= 1);
                assert!(divergence >= -1 && divergence <= 1);
            }
        }
    }
    
    #[test]
    fn test_adaptive_levels() {
        let mut adaptive_stoch = AdaptiveStochastic::new();
        
        // Высокая волатильность должна адаптировать уровни
        for i in 0..20 {
            let base_price = 100.0;
            let volatility = 10.0; // Высокая волатильность
            let price = base_price + (i as f64 * 0.5).sin() * volatility;
            
            let result = adaptive_stoch.update_bar(
                price, 
                price + volatility, 
                price - volatility, 
                price, 
                1000.0
            );
            
            if i > 15 && adaptive_stoch.is_ready() {
                // При высокой волатильности уровни должны быть более экстремальными
                assert!(result.overbought_level >= 80.0);
                assert!(result.oversold_level <= 20.0);
            }
        }
    }
} 






















