//! Ehlers Rocket RSI - "ракетная" версия RSI от Джона Эхлерса
//! 
//! Rocket RSI использует двойное сглаживание и momentum для получения
//! более быстрых и точных сигналов по сравнению с обычным RSI.
//! 
//! Основано на работе John Ehlers "Rocket Science for Traders"
//! 
//! Переиспользует существующие компоненты MovingAverage для оптимизации

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Результат Rocket RSI
#[derive(Debug, Clone, Copy)]
pub struct RocketRsiResult {
    pub rocket_rsi: f64,         // Rocket RSI (0-100)
    pub regular_rsi: f64,        // Обычный RSI для сравнения (0-100)
    pub momentum_factor: f64,    // Фактор momentum (-1.0 до 1.0)
    pub velocity: f64,           // Скорость изменения RSI
    pub acceleration: f64,       // Ускорение RSI
    pub signal_quality: f64,     // Качество сигнала (0.0-1.0)
    pub rocket_signal: i8,       // Rocket сигнал: 1 (покупка), -1 (продажа), 0 (нет)
}

impl RocketRsiResult {
    pub fn empty() -> Self {
        Self {
            rocket_rsi: 50.0,
            regular_rsi: 50.0,
            momentum_factor: 0.0,
            velocity: 0.0,
            acceleration: 0.0,
            signal_quality: 0.0,
            rocket_signal: 0,
        }
    }
    
    /// Определить состояние рынка с учетом momentum
    pub fn market_condition(&self) -> &'static str {
        let momentum_adjusted_levels = if self.momentum_factor > 0.3 {
            (25.0, 75.0)  // При сильном восходящем momentum более жесткие уровни
        } else if self.momentum_factor < -0.3 {
            (25.0, 75.0)  // При сильном нисходящем momentum тоже жесткие
        } else {
            (30.0, 70.0)  // Стандартные уровни
        };
        
        match self.rocket_rsi {
            x if x <= momentum_adjusted_levels.0 => "Rocket перепродан",
            x if x >= momentum_adjusted_levels.1 => "Rocket перекуплен",
            _ => "Rocket нейтральный",
        }
    }
    
    /// Получить описание качества сигнала
    pub fn signal_quality_description(&self) -> &'static str {
        match self.signal_quality {
            x if x < 0.2 => "Очень низкое",
            x if x < 0.4 => "Низкое",
            x if x < 0.6 => "Среднее",
            x if x < 0.8 => "Высокое",
            _ => "Очень высокое",
        }
    }
}

/// Ehlers Rocket RSI индикатор
#[derive(Clone)]
pub struct EhlersRocketRsi {
    // Переиспользуем существующие компоненты
    price_smoother: MovingAverageProvider,    // Первичное сглаживание цены
    momentum_ma: MovingAverageProvider,       // MA для momentum расчетов
    velocity_ma: MovingAverageProvider,       // MA для velocity сглаживания
    
    // RSI компоненты
    rsi_period: usize,
    gains: ArrayVec<f64, 64>,
    losses: ArrayVec<f64, 64>,
    avg_gain: f64,
    avg_loss: f64,
    
    // Rocket компоненты
    smoothed_prices: ArrayVec<f64, 32>,
    rsi_values: ArrayVec<f64, 32>,
    momentum_values: ArrayVec<f64, 16>,
    
    // Параметры
    smoothing_factor: f64,           // Фактор сглаживания (0.0-1.0)
    momentum_period: usize,          // Период для momentum
    
    // Данные для расчетов
    prev_close: Option<f64>,
    
    // Результат
    current_result: RocketRsiResult,
    
    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl EhlersRocketRsi {
    /// Создать новый Rocket RSI с параметрами по умолчанию
    pub fn new() -> Self {
        Self::with_parameters(14, 0.1, 8)
    }
    
    /// Создать с настраиваемыми параметрами
    pub fn with_parameters(rsi_period: usize, smoothing_factor: f64, momentum_period: usize) -> Self {
        assert!(rsi_period > 0, "RSI period must be greater than 0");
        assert!(smoothing_factor > 0.0 && smoothing_factor <= 1.0, 
                "Smoothing factor must be between 0.0 and 1.0");
        assert!(momentum_period > 0, "Momentum period must be greater than 0");
        
        Self {
            // Переиспользуем MovingAverage для разных целей
            price_smoother: MovingAverageProvider::new(MovingAverageType::EMA, 3),
            momentum_ma: MovingAverageProvider::new(MovingAverageType::EMA, momentum_period),
            velocity_ma: MovingAverageProvider::new(MovingAverageType::SMA, 5),
            
            rsi_period,
            gains: ArrayVec::new(),
            losses: ArrayVec::new(),
            avg_gain: 0.0,
            avg_loss: 0.0,
            
            smoothed_prices: ArrayVec::new(),
            rsi_values: ArrayVec::new(),
            momentum_values: ArrayVec::new(),
            
            smoothing_factor,
            momentum_period,
            
            prev_close: None,
            current_result: RocketRsiResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> RocketRsiResult {
        // 1. Сглаживаем цену (переиспользуем MovingAverage)
        let smoothed_price = self.price_smoother.update_bar(0.0, 0.0, 0.0, close, 0.0);
        
        // Сохраняем сглаженную цену
        if self.smoothed_prices.len() >= 32 {
            self.smoothed_prices.remove(0);
        }
        self.smoothed_prices.push(smoothed_price);
        
        // 2. Рассчитываем обычный RSI на сглаженных данных
        let regular_rsi = self.calculate_rsi(smoothed_price);
        
        // 3. Рассчитываем Rocket RSI с momentum
        let rocket_rsi = self.calculate_rocket_rsi(regular_rsi);
        
        // 4. Рассчитываем momentum и производные
        self.calculate_momentum_and_derivatives();
        
        // 5. Оцениваем качество сигнала
        self.calculate_signal_quality();
        
        // 6. Генерируем Rocket сигналы
        self.generate_rocket_signals();
        
        // Обновляем результат
        self.current_result.regular_rsi = regular_rsi;
        self.current_result.rocket_rsi = rocket_rsi;
        
        // Проверяем готовность
        if self.gains.len() >= self.rsi_period && self.rsi_values.len() >= 3 {
            self.is_ready = true;
        }
        
        self.prev_close = Some(close);
        self.update_count += 1;
        self.current_result
    }
    
    /// Рассчитать обычный RSI на сглаженных данных
    fn calculate_rsi(&mut self, smoothed_price: f64) -> f64 {
        if self.smoothed_prices.len() < 2 {
            return 50.0;
        }
        
        let len = self.smoothed_prices.len();
        let prev_smoothed = self.smoothed_prices[len - 2];
        let change = smoothed_price - prev_smoothed;
        
        let gain = if change > 0.0 { change } else { 0.0 };
        let loss = if change < 0.0 { -change } else { 0.0 };
        
        // Добавляем в буферы
        if self.gains.len() >= self.rsi_period {
            self.gains.remove(0);
        }
        self.gains.push(gain);
        
        if self.losses.len() >= self.rsi_period {
            self.losses.remove(0);
        }
        self.losses.push(loss);
        
        // Рассчитываем средние значения
        if self.gains.len() == self.rsi_period {
            if self.avg_gain == 0.0 && self.avg_loss == 0.0 {
                // Первый расчет - простое среднее
                self.avg_gain = self.gains.iter().sum::<f64>() / self.rsi_period as f64;
                self.avg_loss = self.losses.iter().sum::<f64>() / self.rsi_period as f64;
            } else {
                // Экспоненциальное сглаживание (Wilder's method)
                let alpha = 1.0 / self.rsi_period as f64;
                self.avg_gain = alpha * gain + (1.0 - alpha) * self.avg_gain;
                self.avg_loss = alpha * loss + (1.0 - alpha) * self.avg_loss;
            }
            
            // Рассчитываем RSI
            if self.avg_loss == 0.0 {
                return 100.0;
            }
            
            let rs = self.avg_gain / self.avg_loss;
            return 100.0 - (100.0 / (1.0 + rs));
        }
        
        50.0
    }
    
    /// Рассчитать Rocket RSI с momentum усилением
    fn calculate_rocket_rsi(&mut self, regular_rsi: f64) -> f64 {
        // Сохраняем RSI значения
        if self.rsi_values.len() >= 32 {
            self.rsi_values.remove(0);
        }
        self.rsi_values.push(regular_rsi);
        
        if self.rsi_values.len() < 3 {
            return regular_rsi;
        }
        
        // Рассчитываем momentum RSI
        let len = self.rsi_values.len();
        let current_rsi = self.rsi_values[len - 1];
        let prev_rsi = self.rsi_values[len - 2];
        let momentum = current_rsi - prev_rsi;
        
        // Сглаживаем momentum (переиспользуем MovingAverage)
        let smoothed_momentum = self.momentum_ma.update_bar(0.0, 0.0, 0.0, momentum, 0.0);
        
        // Применяем Rocket формулу: RSI + momentum_factor * smoothed_momentum
        let rocket_rsi = regular_rsi + self.smoothing_factor * smoothed_momentum * 10.0;
        
        // Ограничиваем диапазон 0-100
        rocket_rsi.clamp(0.0, 100.0)
    }
    
    /// Рассчитать momentum и производные
    fn calculate_momentum_and_derivatives(&mut self) {
        if self.rsi_values.len() < 3 {
            return;
        }
        
        let len = self.rsi_values.len();
        let current_rsi = self.rsi_values[len - 1];
        let prev_rsi = self.rsi_values[len - 2];
        let prev2_rsi = self.rsi_values[len - 3];
        
        // Velocity (первая производная)
        let velocity = current_rsi - prev_rsi;
        
        // Сглаживаем velocity
        let smoothed_velocity = self.velocity_ma.update_bar(0.0, 0.0, 0.0, velocity, 0.0);
        self.current_result.velocity = smoothed_velocity;
        
        // Acceleration (вторая производная)
        let prev_velocity = prev_rsi - prev2_rsi;
        self.current_result.acceleration = velocity - prev_velocity;
        
        // Momentum factor (нормализованный momentum)
        let momentum_range = 20.0; // Диапазон для нормализации
        self.current_result.momentum_factor = (smoothed_velocity / momentum_range).clamp(-1.0, 1.0);
    }
    
    /// Рассчитать качество сигнала
    fn calculate_signal_quality(&mut self) {
        if !self.is_ready {
            self.current_result.signal_quality = 0.0;
            return;
        }
        
        let rocket_rsi = self.current_result.rocket_rsi;
        let velocity = self.current_result.velocity.abs();
        let momentum_factor = self.current_result.momentum_factor.abs();
        
        // Качество сигнала зависит от:
        // 1. Экстремальности RSI
        let rsi_extremity = if rocket_rsi <= 30.0 {
            (30.0 - rocket_rsi) / 30.0
        } else if rocket_rsi >= 70.0 {
            (rocket_rsi - 70.0) / 30.0
        } else {
            0.0
        };
        
        // 2. Силы momentum
        let momentum_strength = momentum_factor;
        
        // 3. Скорости изменения
        let velocity_factor = (velocity / 5.0).min(1.0);
        
        // Комбинируем факторы
        self.current_result.signal_quality = (rsi_extremity * 0.4 + 
                                             momentum_strength * 0.4 + 
                                             velocity_factor * 0.2).min(1.0);
    }
    
    /// Генерировать Rocket сигналы
    fn generate_rocket_signals(&mut self) {
        if !self.is_ready {
            self.current_result.rocket_signal = 0;
            return;
        }
        
        let rocket_rsi = self.current_result.rocket_rsi;
        let momentum_factor = self.current_result.momentum_factor;
        let signal_quality = self.current_result.signal_quality;
        
        // Rocket сигналы требуют высокого качества
        if signal_quality < 0.5 {
            self.current_result.rocket_signal = 0;
            return;
        }
        
        // Покупка: перепроданность + положительный momentum
        if rocket_rsi <= 30.0 && momentum_factor > 0.2 {
            self.current_result.rocket_signal = 1;
        }
        // Продажа: перекупленность + отрицательный momentum  
        else if rocket_rsi >= 70.0 && momentum_factor < -0.2 {
            self.current_result.rocket_signal = -1;
        }
        // Экстремальные сигналы
        else if rocket_rsi <= 15.0 && signal_quality > 0.7 {
            self.current_result.rocket_signal = 1; // Сильная покупка
        }
        else if rocket_rsi >= 85.0 && signal_quality > 0.7 {
            self.current_result.rocket_signal = -1; // Сильная продажа
        }
        else {
            self.current_result.rocket_signal = 0;
        }
    }
    
    /// Получить текущее значение Rocket RSI
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.rocket_rsi)
    }
    
    /// Получить полный результат
    pub fn result(&self) -> RocketRsiResult {
        self.current_result
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.price_smoother.reset();
        self.momentum_ma.reset();
        self.velocity_ma.reset();
        
        self.gains.clear();
        self.losses.clear();
        self.avg_gain = 0.0;
        self.avg_loss = 0.0;
        
        self.smoothed_prices.clear();
        self.rsi_values.clear();
        self.momentum_values.clear();
        
        self.prev_close = None;
        self.current_result = RocketRsiResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.rsi_period
    }
    
    /// Генерировать торговый сигнал
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        self.current_result.rocket_signal
    }
    
    /// Генерировать сигнал дивергенции между Rocket RSI и обычным RSI
    pub fn divergence_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let rocket_rsi = self.current_result.rocket_rsi;
        let regular_rsi = self.current_result.regular_rsi;
        let divergence = rocket_rsi - regular_rsi;
        
        // Значительная дивергенция при высоком качестве сигнала
        if self.current_result.signal_quality > 0.6 {
            if divergence > 10.0 {
                return 1; // Rocket RSI сильно выше - покупка
            } else if divergence < -10.0 {
                return -1; // Rocket RSI сильно ниже - продажа
            }
        }
        
        0
    }
    
    /// Получить информацию о текущем состоянии
    pub fn info(&self) -> String {
        let result = self.current_result;
        let signal = match self.trading_signal() {
            1 => "🚀 Покупка",
            -1 => "🚀 Продажа",
            _ => "Нет сигнала",
        };
        
        format!(
            "Rocket RSI: {:.1}, RSI: {:.1}, Momentum: {:.2}, Velocity: {:.2}, Качество: {} ({:.2}), Сигнал: {}",
            result.rocket_rsi,
            result.regular_rsi,
            result.momentum_factor,
            result.velocity,
            result.signal_quality_description(),
            result.signal_quality,
            signal
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("rocket_rsi".to_string(), self.current_result.rocket_rsi);
        values.insert("regular_rsi".to_string(), self.current_result.regular_rsi);
        values.insert("momentum_factor".to_string(), self.current_result.momentum_factor);
        values.insert("velocity".to_string(), self.current_result.velocity);
        values.insert("acceleration".to_string(), self.current_result.acceleration);
        values.insert("signal_quality".to_string(), self.current_result.signal_quality);
        values.insert("rocket_signal".to_string(), self.current_result.rocket_signal as f64);
        values.insert("rsi_divergence".to_string(), self.current_result.rocket_rsi - self.current_result.regular_rsi);
        values
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    
    /// Получить параметры
    pub fn parameters(&self) -> (usize, f64, usize) {
        (self.rsi_period, self.smoothing_factor, self.momentum_period)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rocket_rsi_creation() {
        let rocket_rsi = EhlersRocketRsi::new();
        assert!(!rocket_rsi.is_ready());
        assert_eq!(rocket_rsi.parameters().0, 14);
    }
    
    #[test]
    fn test_rocket_rsi_with_parameters() {
        let rocket_rsi = EhlersRocketRsi::with_parameters(21, 0.2, 10);
        assert_eq!(rocket_rsi.parameters(), (21, 0.2, 10));
    }
    
    #[test]
    fn test_rocket_rsi_update() {
        let mut rocket_rsi = EhlersRocketRsi::new();
        
        // Добавляем трендовые данные для тестирования momentum
        for i in 0..25 {
            let price = 100.0 + i as f64 * 0.5; // Восходящий тренд
            let result = rocket_rsi.update_bar(price, price + 0.5, price - 0.5, price, 1000.0);
            
            if i > 15 {
                assert!(rocket_rsi.is_ready());
                assert!(result.rocket_rsi >= 0.0 && result.rocket_rsi <= 100.0);
                assert!(result.regular_rsi >= 0.0 && result.regular_rsi <= 100.0);
                assert!(result.signal_quality >= 0.0 && result.signal_quality <= 1.0);
            }
        }
    }
    
    #[test]
    fn test_rocket_signals() {
        let mut rocket_rsi = EhlersRocketRsi::new();
        
        // Создаем условия для перепроданности с momentum
        let mut price = 100.0;
        for i in 0..20 {
            price -= 1.0; // Падающие цены
            let _result = rocket_rsi.update_bar(price, price + 0.1, price - 0.1, price, 1000.0);
            
            if i > 15 && rocket_rsi.is_ready() {
                let signal = rocket_rsi.trading_signal();
                let div_signal = rocket_rsi.divergence_signal();
                
                assert!(signal >= -1 && signal <= 1);
                assert!(div_signal >= -1 && div_signal <= 1);
            }
        }
    }
    
    #[test]
    fn test_momentum_calculations() {
        let mut rocket_rsi = EhlersRocketRsi::new();
        
        // Тестируем различные momentum условия
        let prices = [100.0, 101.0, 103.0, 106.0, 110.0, 115.0]; // Ускоряющийся рост
        for &price in &prices {
            let result = rocket_rsi.update_bar(price, price + 0.5, price - 0.5, price, 1000.0);
            
            if rocket_rsi.is_ready() {
                assert!(result.momentum_factor >= -1.0 && result.momentum_factor <= 1.0);
                assert!(result.velocity.is_finite());
                assert!(result.acceleration.is_finite());
            }
        }
    }
} 






















