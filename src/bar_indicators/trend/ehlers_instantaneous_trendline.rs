//! Ehlers Instantaneous Trendline - мгновенная трендовая линия от Джона Эхлерса
//!
//! Использует Hilbert Transform для создания адаптивной трендовой линии,
//! которая автоматически подстраивается под изменения в данных.
//!
//! Основано на работе John Ehlers "Rocket Science for Traders"
//!
//! Переиспользует существующие MovingAverage компоненты для оптимизации

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;


/// Результат Instantaneous Trendline
#[derive(Debug, Clone, Copy)]
pub struct InstantaneousTrendlineResult {
    pub trendline: f64,          // Мгновенная трендовая линия
    pub trend_direction: i8,     // Направление тренда: 1 (вверх), -1 (вниз), 0 (боковик)
    pub trend_strength: f64,     // Сила тренда (0.0 до 1.0)
    pub cycle_component: f64,    // Циклическая компонента
    pub noise_level: f64,        // Уровень шума (0.0 до 1.0)
    pub phase: f64,              // Фаза сигнала
}

impl InstantaneousTrendlineResult {
    pub fn empty() -> Self {
        Self {
            trendline: 0.0,
            trend_direction: 0,
            trend_strength: 0.0,
            cycle_component: 0.0,
            noise_level: 0.5,
            phase: 0.0,
        }
    }
    
    /// Получить описание направления тренда
    pub fn trend_direction_name(&self) -> &'static str {
        match self.trend_direction {
            1 => "Восходящий",
            -1 => "Нисходящий", 
            _ => "Боковой",
        }
    }
    
    /// Получить описание силы тренда
    pub fn trend_strength_name(&self) -> &'static str {
        match self.trend_strength {
            x if x < 0.2 => "Очень слабый",
            x if x < 0.4 => "Слабый",
            x if x < 0.6 => "Умеренный",
            x if x < 0.8 => "Сильный",
            _ => "Очень сильный",
        }
    }
}

/// Ehlers Instantaneous Trendline индикатор
#[derive(Clone)]
pub struct EhlersInstantaneousTrendline {
    // Переиспользуем существующие MA компоненты
    smoothing_ma: MovingAverageProvider,     // MA для сглаживания
    trend_ma: MovingAverageProvider,         // MA для трендовой компоненты
    noise_ma: MovingAverageProvider,         // MA для анализа шума

    // Буферы для расчетов
    prices: ArrayVec<f64, 64>,
    i_components: ArrayVec<f64, 32>,  // In-Phase компоненты
    q_components: ArrayVec<f64, 32>,  // Quadrature компоненты
    trendlines: ArrayVec<f64, 32>,    // История трендовых линий

    // Параметры
    alpha: f64,                      // Коэффициент сглаживания (0.0 до 1.0)

    // Результат
    current_result: InstantaneousTrendlineResult,

    // Источник данных
    source: OhlcvField,

    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl EhlersInstantaneousTrendline {
    /// Создать новый индикатор с параметрами по умолчанию
    pub fn new() -> Self {
        Self::with_alpha(0.07)
    }

    /// Создать новый индикатор с настраиваемым коэффициентом сглаживания
    pub fn with_alpha(alpha: f64) -> Self {
        Self::with_source(alpha, OhlcvField::Close)
    }

    /// Создать с настраиваемым источником данных
    pub fn with_source(alpha: f64, source: OhlcvField) -> Self {
        assert!(alpha > 0.0 && alpha <= 1.0, "Alpha must be between 0.0 and 1.0");

        Self {
            // Переиспользуем MovingAverage для разных целей
            smoothing_ma: MovingAverageProvider::new(MovingAverageType::EMA, 5),
            trend_ma: MovingAverageProvider::new(MovingAverageType::EMA, 10),
            noise_ma: MovingAverageProvider::new(MovingAverageType::SMA, 20),

            prices: ArrayVec::new(),
            i_components: ArrayVec::new(),
            q_components: ArrayVec::new(),
            trendlines: ArrayVec::new(),

            alpha,
            current_result: InstantaneousTrendlineResult::empty(),
            source,
            is_ready: false,
            update_count: 0,
        }
    }

    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> InstantaneousTrendlineResult {
        let price = self.source.extract(open, high, low, close, volume);
        self.update_price(price)
    }
    
    /// Обновить индикатор новой ценой
    pub fn update_price(&mut self, price: f64) -> InstantaneousTrendlineResult {
        // Добавляем цену в буфер
        if self.prices.len() >= 64 {
            self.prices.remove(0);
        }
        self.prices.push(price);
        
        // Нужно минимум данных для расчетов
        if self.prices.len() >= 7 {
            // 1. Рассчитываем сглаженную цену
            let smoothed_price = self.calculate_smoothed_price();
            
            // 2. Рассчитываем Hilbert Transform компоненты
            self.calculate_hilbert_components(smoothed_price);
            
            // 3. Рассчитываем мгновенную трендовую линию
            self.calculate_instantaneous_trendline();
            
            // 4. Анализируем тренд и шум
            self.analyze_trend_and_noise();
            
            self.is_ready = true;
        }
        
        self.update_count += 1;
        self.current_result
    }
    
    /// Рассчитать сглаженную цену используя существующую MA
    fn calculate_smoothed_price(&mut self) -> f64 {
        let current_price = self.prices[self.prices.len() - 1];
        self.smoothing_ma.update_bar(0.0, 0.0, 0.0, current_price, 0.0)
    }
    
    /// Рассчитать компоненты Hilbert Transform
    fn calculate_hilbert_components(&mut self, smoothed_price: f64) {
        let len = self.prices.len();
        if len < 7 {
            return;
        }
        
        // I компонент (In-Phase) - текущая сглаженная цена
        let i_component = smoothed_price;
        
        // Q компонент (Quadrature) - используем сдвиг на 90 градусов
        let q_component = if len >= 4 {
            // Приближение Hilbert Transform через задержку и сглаживание
            let delayed_price = self.prices[len - 4];
            let smoothed_delayed = self.trend_ma.update_bar(0.0, 0.0, 0.0, delayed_price, 0.0);
            
            // Квадратурная компонента
            (smoothed_price - smoothed_delayed) * 0.707 // sin(45°) ≈ 0.707
        } else {
            0.0
        };
        
        // Сохраняем компоненты
        if self.i_components.len() >= 32 {
            self.i_components.remove(0);
        }
        self.i_components.push(i_component);
        
        if self.q_components.len() >= 32 {
            self.q_components.remove(0);
        }
        self.q_components.push(q_component);
        
        // Рассчитываем фазу
        if i_component != 0.0 {
            self.current_result.phase = (q_component / i_component).atan();
        }
    }
    
    /// Рассчитать мгновенную трендовую линию
    fn calculate_instantaneous_trendline(&mut self) {
        if self.i_components.len() < 2 || self.q_components.len() < 2 {
            return;
        }
        
        let i_len = self.i_components.len();
        let q_len = self.q_components.len();
        
        let i_curr = self.i_components[i_len - 1];
        let _i_prev = if i_len >= 2 { self.i_components[i_len - 2] } else { i_curr };
        let q_curr = self.q_components[q_len - 1];
        let _q_prev = if q_len >= 2 { self.q_components[q_len - 2] } else { q_curr };
        
        // Рассчитываем мгновенную трендовую линию по формуле Эхлерса
        // ITrend = (a - a²/4) * Price + (a²/2) * Price[1] - (a - 3a²/4) * ITrend[1]
        let a = self.alpha;
        let a2 = a * a;
        
        let current_price = self.prices[self.prices.len() - 1];
        let prev_price = if self.prices.len() >= 2 { 
            self.prices[self.prices.len() - 2] 
        } else { 
            current_price 
        };
        
        let prev_trendline = if self.trendlines.is_empty() { 
            current_price 
        } else { 
            self.trendlines[self.trendlines.len() - 1] 
        };
        
        let trendline = (a - a2 / 4.0) * current_price 
                       + (a2 / 2.0) * prev_price 
                       - (a - 3.0 * a2 / 4.0) * prev_trendline;
        
        // Сохраняем трендовую линию
        if self.trendlines.len() >= 32 {
            self.trendlines.remove(0);
        }
        self.trendlines.push(trendline);
        
        self.current_result.trendline = trendline;
        
        // Рассчитываем циклическую компоненту
        self.current_result.cycle_component = current_price - trendline;
    }
    
    /// Анализировать тренд и шум
    fn analyze_trend_and_noise(&mut self) {
        if self.trendlines.len() < 3 {
            return;
        }
        
        let len = self.trendlines.len();
        let current_trendline = self.trendlines[len - 1];
        let prev_trendline = self.trendlines[len - 2];
        let prev2_trendline = self.trendlines[len - 3];
        
        // Определяем направление тренда
        let trend_change = current_trendline - prev_trendline;
        let prev_trend_change = prev_trendline - prev2_trendline;
        
        self.current_result.trend_direction = if trend_change > 0.0 && prev_trend_change > 0.0 {
            1  // Восходящий тренд
        } else if trend_change < 0.0 && prev_trend_change < 0.0 {
            -1 // Нисходящий тренд
        } else {
            0  // Боковой тренд
        };
        
        // Рассчитываем силу тренда
        let trend_changes: Vec<f64> = self.trendlines.windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .collect();
        
        if !trend_changes.is_empty() {
            let avg_change: f64 = trend_changes.iter().sum::<f64>() / trend_changes.len() as f64;
            let current_change = trend_change.abs();
            
            // Нормализуем силу тренда
            self.current_result.trend_strength = if avg_change > 0.0 {
                (current_change / avg_change).min(1.0)
            } else {
                0.0
            };
        }
        
        // Анализируем уровень шума
        let current_price = self.prices[self.prices.len() - 1];
        let noise = (current_price - current_trendline).abs();
        
        // Обновляем MA для анализа шума
        let avg_noise = self.noise_ma.update_bar(0.0, 0.0, 0.0, noise, 0.0);
        
        // Нормализуем уровень шума
        self.current_result.noise_level = if current_trendline != 0.0 {
            (avg_noise / current_trendline.abs()).min(1.0)
        } else {
            0.5
        };
    }
    
    /// Получить текущее значение трендовой линии
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.trendline)
    }
    
    /// Получить полный результат
    pub fn result(&self) -> InstantaneousTrendlineResult {
        self.current_result
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.smoothing_ma.reset();
        self.trend_ma.reset();
        self.noise_ma.reset();
        
        self.prices.clear();
        self.i_components.clear();
        self.q_components.clear();
        self.trendlines.clear();
        
        self.current_result = InstantaneousTrendlineResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }
    
    /// Получить период (условный)
    pub fn period(&self) -> usize {
        (2.0 / self.alpha) as usize
    }
    
    /// Генерировать торговый сигнал
    pub fn trading_signal(&self, current_price: f64) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let result = self.current_result;
        
        // Сигналы только при низком уровне шума и достаточной силе тренда
        if result.noise_level > 0.3 || result.trend_strength < 0.4 {
            return 0;
        }
        
        // Пересечение цены и трендовой линии
        if current_price > result.trendline && result.trend_direction == 1 {
            return 1; // Покупка при восходящем тренде
        } else if current_price < result.trendline && result.trend_direction == -1 {
            return -1; // Продажа при нисходящем тренде
        }
        
        0
    }
    
    /// Генерировать сигнал дивергенции
    pub fn divergence_signal(&self, current_price: f64, prev_price: f64) -> i8 {
        if !self.is_ready || self.trendlines.len() < 2 {
            return 0;
        }
        
        let current_trendline = self.current_result.trendline;
        let prev_trendline = self.trendlines[self.trendlines.len() - 2];
        
        let price_direction = (current_price - prev_price).signum() as i8;
        let trend_direction = (current_trendline - prev_trendline).signum() as i8;
        
        // Дивергенция: цена и трендовая линия движутся в разных направлениях
        if price_direction != 0 && trend_direction != 0 && price_direction != trend_direction {
            return -price_direction; // Сигнал противоположный направлению цены
        }
        
        0
    }
    
    /// Получить информацию о текущем состоянии
    pub fn info(&self, current_price: f64) -> String {
        let result = self.current_result;
        let position = if current_price > result.trendline { "Выше" } else { "Ниже" };
        
        format!(
            "Ehlers ITrend: {:.4}, Цена {} тренда, Направление: {}, Сила: {} ({:.2}), Шум: {:.2}",
            result.trendline,
            position,
            result.trend_direction_name(),
            result.trend_strength_name(),
            result.trend_strength,
            result.noise_level
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("trendline".to_string(), self.current_result.trendline);
        values.insert("trend_direction".to_string(), self.current_result.trend_direction as f64);
        values.insert("trend_strength".to_string(), self.current_result.trend_strength);
        values.insert("cycle_component".to_string(), self.current_result.cycle_component);
        values.insert("noise_level".to_string(), self.current_result.noise_level);
        values.insert("phase".to_string(), self.current_result.phase);
        values
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    
    /// Получить параметры
    pub fn alpha(&self) -> f64 {
        self.alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ehlers_instantaneous_trendline_creation() {
        let itl = EhlersInstantaneousTrendline::new();
        assert!(!itl.is_ready());
        assert_eq!(itl.alpha(), 0.07);
    }
    
    #[test]
    fn test_ehlers_with_alpha() {
        let itl = EhlersInstantaneousTrendline::with_alpha(0.1);
        assert_eq!(itl.alpha(), 0.1);
    }
    
    #[test]
    fn test_ehlers_update() {
        let mut itl = EhlersInstantaneousTrendline::new();
        
        // Добавляем трендовые данные
        for i in 0..20 {
            let price = 100.0 + i as f64 * 0.5; // Восходящий тренд
            let result = itl.update_price(price);
            
            if i > 10 {
                assert!(itl.is_ready());
                assert!(result.trendline > 0.0);
                assert!(result.trend_strength >= 0.0 && result.trend_strength <= 1.0);
                assert!(result.noise_level >= 0.0 && result.noise_level <= 1.0);
            }
        }
        
        // При восходящем тренде направление должно быть положительным
        assert_eq!(itl.result().trend_direction, 1);
    }
    
    #[test]
    fn test_trading_signals() {
        let mut itl = EhlersInstantaneousTrendline::new();
        
        // Добавляем данные
        for i in 0..15 {
            let price = 100.0 + i as f64;
            let _result = itl.update_price(price);
        }
        
        if itl.is_ready() {
            let signal = itl.trading_signal(115.0);
            assert!(signal >= -1 && signal <= 1);
        }
    }
    
    #[test]
    fn test_divergence_signals() {
        let mut itl = EhlersInstantaneousTrendline::new();
        
        // Добавляем данные
        for i in 0..15 {
            let price = 100.0 + i as f64;
            let _result = itl.update_price(price);
        }
        
        if itl.is_ready() {
            let signal = itl.divergence_signal(115.0, 114.0);
            assert!(signal >= -1 && signal <= 1);
        }
    }
} 






















