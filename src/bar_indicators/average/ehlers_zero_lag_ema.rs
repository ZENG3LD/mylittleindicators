//! Ehlers Zero Lag EMA - EMA без задержки от Джона Эхлерса
//!
//! Zero Lag EMA компенсирует естественную задержку обычной EMA,
//! обеспечивая более быстрые сигналы при сохранении сглаживания.
//!
//! Формула: ZLEMA = EMA + (EMA - EMA[lag])
//! где lag рассчитывается как (period - 1) / 2
//!
//! Переиспользует существующие компоненты EMA

use crate::bar_indicators::average::ema::Ema;
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;
use super::super::ohlcv_field::OhlcvField;

/// Результат Zero Lag EMA
#[derive(Debug, Clone, Copy)]
pub struct ZeroLagEmaResult {
    pub zlema: f64,              // Zero Lag EMA значение
    pub regular_ema: f64,        // Обычная EMA для сравнения
    pub lag_compensation: f64,   // Величина компенсации задержки
    pub trend_direction: i8,     // Направление тренда: 1 (вверх), -1 (вниз), 0 (боковик)
    pub trend_strength: f64,     // Сила тренда (0.0-1.0)
    pub responsiveness: f64,     // Отзывчивость по сравнению с EMA (1.0+)
}

impl ZeroLagEmaResult {
    pub fn empty() -> Self {
        Self {
            zlema: 0.0,
            regular_ema: 0.0,
            lag_compensation: 0.0,
            trend_direction: 0,
            trend_strength: 0.0,
            responsiveness: 1.0,
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

/// Ehlers Zero Lag EMA индикатор
#[derive(Debug, Clone)]
pub struct EhlersZeroLagEma {
    // Переиспользуем существующие компоненты
    main_ema: Ema,                   // Основная EMA
    lag_ema: Ema,                    // EMA для расчета задержки
    trend_ema: Ema,                  // EMA для анализа тренда

    // Буферы для расчетов
    prices: ArrayVec<f64, 32>,       // Буфер цен
    zlema_values: ArrayVec<f64, 16>, // Буфер ZLEMA значений
    ema_values: ArrayVec<f64, 16>,   // Буфер EMA значений

    // Параметры
    period: usize,
    source: OhlcvField,              // Источник данных
    lag_period: usize,               // Период задержки

    // Результат
    current_result: ZeroLagEmaResult,

    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl EhlersZeroLagEma {
    /// Создать новый Zero Lag EMA с периодом по умолчанию
    pub fn new() -> Self {
        Self::with_period(21)
    }

    /// Создать новый Zero Lag EMA с заданным периодом
    pub fn with_period(period: usize) -> Self {
        Self::with_source(period, OhlcvField::Close)
    }

    /// Создать новый Zero Lag EMA с заданным периодом и источником
    pub fn with_source(period: usize, source: OhlcvField) -> Self {
        assert!(period > 0, "Period must be greater than 0");

        let lag_period = ((period - 1) / 2).max(1);

        Self {
            // Переиспользуем MovingAverage для разных целей
            main_ema: Ema::with_source(period, source),
            lag_ema: Ema::new(period),
            trend_ema: Ema::new(5),

            prices: ArrayVec::new(),
            zlema_values: ArrayVec::new(),
            ema_values: ArrayVec::new(),

            period,
            source,
            lag_period,

            current_result: ZeroLagEmaResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }
    
    /// Создать Zero Lag EMA с настраиваемым lag периодом
    pub fn with_custom_lag(period: usize, lag_period: usize) -> Self {
        Self::with_custom_lag_and_source(period, lag_period, OhlcvField::Close)
    }

    /// Создать Zero Lag EMA с настраиваемым lag периодом и источником
    pub fn with_custom_lag_and_source(period: usize, lag_period: usize, source: OhlcvField) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        assert!(lag_period > 0 && lag_period < period,
                "Lag period must be positive and less than main period");

        let mut zlema = Self::with_source(period, source);
        zlema.lag_period = lag_period;
        zlema
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> ZeroLagEmaResult {
        let price = self.source.extract(open, high, low, close, volume);
        self.update_price(price)
    }
    
    /// Обновить индикатор новой ценой
    pub fn update_price(&mut self, price: f64) -> ZeroLagEmaResult {
        // Добавляем цену в буфер
        if self.prices.len() >= 32 {
            self.prices.remove(0);
        }
        self.prices.push(price);
        
        // 1. Рассчитываем основную EMA
        let current_ema = self.main_ema.update_bar(0.0, 0.0, 0.0, price, 0.0);
        
        // Сохраняем EMA значения
        if self.ema_values.len() >= 16 {
            self.ema_values.remove(0);
        }
        self.ema_values.push(current_ema);
        
        // 2. Получаем задержанную EMA
        let lagged_ema = if self.ema_values.len() > self.lag_period {
            self.ema_values[self.ema_values.len() - 1 - self.lag_period]
        } else {
            current_ema
        };
        
        // 3. Рассчитываем Zero Lag EMA
        let lag_compensation = current_ema - lagged_ema;
        let zlema = current_ema + lag_compensation;
        
        // Сохраняем ZLEMA значения
        if self.zlema_values.len() >= 16 {
            self.zlema_values.remove(0);
        }
        self.zlema_values.push(zlema);
        
        // 4. Анализируем тренд и силу
        self.analyze_trend_and_strength(price, zlema, current_ema);
        
        // 5. Рассчитываем отзывчивость
        self.calculate_responsiveness(price, zlema, current_ema);
        
        // Обновляем результат
        self.current_result.zlema = zlema;
        self.current_result.regular_ema = current_ema;
        self.current_result.lag_compensation = lag_compensation;
        
        // Готов после накопления достаточных данных
        if self.main_ema.is_ready() && self.ema_values.len() > self.lag_period {
            self.is_ready = true;
        }
        
        self.update_count += 1;
        self.current_result
    }
    
    /// Анализировать тренд и силу
    fn analyze_trend_and_strength(&mut self, current_price: f64, zlema: f64, ema: f64) {
        if self.zlema_values.len() < 3 {
            return;
        }
        
        let len = self.zlema_values.len();
        let current_zlema = self.zlema_values[len - 1];
        let prev_zlema = self.zlema_values[len - 2];
        let prev2_zlema = self.zlema_values[len - 3];
        
        // Определяем направление тренда
        let trend_change = current_zlema - prev_zlema;
        let prev_trend_change = prev_zlema - prev2_zlema;
        
        self.current_result.trend_direction = if trend_change > 0.0 && prev_trend_change > 0.0 {
            1  // Устойчивый восходящий тренд
        } else if trend_change < 0.0 && prev_trend_change < 0.0 {
            -1 // Устойчивый нисходящий тренд
        } else {
            0  // Боковой тренд или смена направления
        };
        
        // Рассчитываем силу тренда
        let price_distance = (current_price - zlema).abs();
        let ema_distance = (zlema - ema).abs();
        
        // Сглаживаем силу тренда
        let trend_strength = if zlema != 0.0 {
            ((price_distance + ema_distance) / zlema.abs()).min(1.0)
        } else {
            0.0
        };
        
        self.current_result.trend_strength = self.trend_ema.update_bar(0.0, 0.0, 0.0, trend_strength, 0.0);
    }
    
    /// Рассчитать отзывчивость по сравнению с обычной EMA
    fn calculate_responsiveness(&mut self, current_price: f64, zlema: f64, ema: f64) {
        if self.prices.len() < 2 {
            self.current_result.responsiveness = 1.0;
            return;
        }
        
        let len = self.prices.len();
        let prev_price = self.prices[len - 2];
        let price_change = (current_price - prev_price).abs();
        
        if price_change > 0.0 {
            let zlema_response = (zlema - prev_price).abs();
            let ema_response = (ema - prev_price).abs();
            
            if ema_response > 0.0 {
                self.current_result.responsiveness = (zlema_response / ema_response).min(3.0);
            } else {
                self.current_result.responsiveness = 1.0;
            }
        } else {
            self.current_result.responsiveness = 1.0;
        }
    }
    
    /// Получить текущее значение Zero Lag EMA
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.zlema)
    }
    
    /// Получить полный результат
    pub fn result(&self) -> ZeroLagEmaResult {
        self.current_result
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.main_ema.reset();
        self.lag_ema.reset();
        self.trend_ema.reset();
        
        self.prices.clear();
        self.zlema_values.clear();
        self.ema_values.clear();
        
        self.current_result = ZeroLagEmaResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Генерировать торговый сигнал на основе пересечения цены и ZLEMA
    pub fn trading_signal(&self, current_price: f64, prev_price: f64) -> i8 {
        if !self.is_ready || self.zlema_values.len() < 2 {
            return 0;
        }
        
        let current_zlema = self.current_result.zlema;
        let prev_zlema = self.zlema_values[self.zlema_values.len() - 2];
        
        // Пересечение цены и ZLEMA
        if prev_price <= prev_zlema && current_price > current_zlema {
            return 1; // Пересечение вверх
        } else if prev_price >= prev_zlema && current_price < current_zlema {
            return -1; // Пересечение вниз
        }
        
        0
    }
    
    /// Генерировать сигнал на основе направления ZLEMA
    pub fn trend_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let result = self.current_result;
        
        // Сигналы только при достаточной силе тренда
        if result.trend_strength > 0.3 {
            return result.trend_direction;
        }
        
        0
    }
    
    /// Генерировать сигнал дивергенции между ZLEMA и EMA
    pub fn divergence_signal(&self) -> i8 {
        if !self.is_ready || self.zlema_values.len() < 2 || self.ema_values.len() < 2 {
            return 0;
        }
        
        let zlema_len = self.zlema_values.len();
        let ema_len = self.ema_values.len();
        
        let current_zlema = self.zlema_values[zlema_len - 1];
        let prev_zlema = self.zlema_values[zlema_len - 2];
        let current_ema = self.ema_values[ema_len - 1];
        let prev_ema = self.ema_values[ema_len - 2];
        
        let zlema_direction = (current_zlema - prev_zlema).signum() as i8;
        let ema_direction = (current_ema - prev_ema).signum() as i8;
        
        // Дивергенция: ZLEMA и EMA движутся в разных направлениях
        if zlema_direction != 0 && ema_direction != 0 && zlema_direction != ema_direction {
            return zlema_direction; // Следуем направлению более быстрой ZLEMA
        }
        
        0
    }
    
    /// Получить информацию о текущем состоянии
    pub fn info(&self, current_price: f64) -> String {
        let result = self.current_result;
        let position = if current_price > result.zlema { "Выше" } else { "Ниже" };
        
        format!(
            "Zero Lag EMA: {:.4}, EMA: {:.4}, Цена {} ZLEMA, Тренд: {} ({}), Отзывчивость: {:.1}x",
            result.zlema,
            result.regular_ema,
            position,
            result.trend_direction_name(),
            result.trend_strength_name(),
            result.responsiveness
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("zlema".to_string(), self.current_result.zlema);
        values.insert("regular_ema".to_string(), self.current_result.regular_ema);
        values.insert("lag_compensation".to_string(), self.current_result.lag_compensation);
        values.insert("trend_direction".to_string(), self.current_result.trend_direction as f64);
        values.insert("trend_strength".to_string(), self.current_result.trend_strength);
        values.insert("responsiveness".to_string(), self.current_result.responsiveness);
        values
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    
    /// Получить параметры
    pub fn parameters(&self) -> (usize, usize) {
        (self.period, self.lag_period)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_lag_ema_creation() {
        let zlema = EhlersZeroLagEma::new();
        assert!(!zlema.is_ready());
        assert_eq!(zlema.period(), 21);
    }

    #[test]
    fn test_zero_lag_ema_with_period() {
        let zlema = EhlersZeroLagEma::with_period(14);
        assert_eq!(zlema.period(), 14);
        assert_eq!(zlema.parameters().1, 6);
    }

    #[test]
    fn test_zero_lag_ema_with_custom_lag() {
        let zlema = EhlersZeroLagEma::with_custom_lag(20, 5);
        assert_eq!(zlema.parameters(), (20, 5));
    }

    #[test]
    fn test_zero_lag_ema_update() {
        let mut zlema = EhlersZeroLagEma::new();
        for i in 0..30 {
            let price = 100.0 + i as f64 * 0.5;
            let result = zlema.update_price(price);
            if i > 25 {
                assert!(zlema.is_ready());
                assert!(result.zlema.is_finite());
            }
        }
        assert_eq!(zlema.result().trend_direction, 1);
    }

    #[test]
    fn test_responsiveness() {
        let mut zlema = EhlersZeroLagEma::with_period(5);
        let prices = [100.0, 100.0, 100.0, 105.0, 110.0];
        for &price in &prices {
            let result = zlema.update_price(price);
            if zlema.is_ready() {
                assert!(result.responsiveness >= 1.0);
            }
        }
    }

    #[test]
    fn test_trading_signals() {
        let mut zlema = EhlersZeroLagEma::new();
        for i in 0..25 {
            let price = 100.0 + i as f64;
            let _result = zlema.update_price(price);
        }
        if zlema.is_ready() {
            let signal = zlema.trading_signal(125.0, 124.0);
            assert!(signal >= -1 && signal <= 1);
        }
    }
} 






















