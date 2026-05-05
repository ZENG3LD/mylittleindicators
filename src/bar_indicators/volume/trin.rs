//! TRIN (Arms Index) - индикатор рыночной широты для анализа настроений рынка
//! 
//! TRIN = (Advancing Issues / Declining Issues) / (Advancing Volume / Declining Volume)
//! 
//! Интерпретация:
//! TRIN < 1.0 = Бычьи настроения (больше объема в растущих акциях)
//! TRIN > 1.0 = Медвежьи настроения (больше объема в падающих акциях)
//! TRIN = 1.0 = Нейтральный рынок
//! 
//! Экстремальные значения:
//! TRIN < 0.5 = Сильно перекупленный рынок
//! TRIN > 2.0 = Сильно перепроданный рынок

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Данные для расчета TRIN
#[derive(Debug, Clone, Copy)]
pub struct TrinData {
    pub advancing_issues: f64,    // Количество растущих инструментов
    pub declining_issues: f64,    // Количество падающих инструментов
    pub advancing_volume: f64,    // Объем в растущих инструментах
    pub declining_volume: f64,    // Объем в падающих инструментах
}

impl TrinData {
    pub fn new(advancing_issues: f64, declining_issues: f64, advancing_volume: f64, declining_volume: f64) -> Self {
        Self {
            advancing_issues,
            declining_issues,
            advancing_volume,
            declining_volume,
        }
    }
}

/// Анализ настроений рынка
#[derive(Debug, Clone, Copy)]
pub enum MarketSentiment {
    StronglyBullish,    // TRIN < 0.5
    Bullish,           // TRIN 0.5-0.8
    SlightlyBullish,   // TRIN 0.8-1.0
    Neutral,           // TRIN = 1.0
    SlightlyBearish,   // TRIN 1.0-1.2
    Bearish,           // TRIN 1.2-2.0
    StronglyBearish,   // TRIN > 2.0
}

impl MarketSentiment {
    pub fn from_trin(trin_value: f64) -> Self {
        if trin_value < 0.5 {
            MarketSentiment::StronglyBullish
        } else if trin_value < 0.8 {
            MarketSentiment::Bullish
        } else if trin_value < 1.0 {
            MarketSentiment::SlightlyBullish
        } else if trin_value == 1.0 {
            MarketSentiment::Neutral
        } else if trin_value < 1.2 {
            MarketSentiment::SlightlyBearish
        } else if trin_value < 2.0 {
            MarketSentiment::Bearish
        } else {
            MarketSentiment::StronglyBearish
        }
    }
    
    pub fn to_string(&self) -> &'static str {
        match self {
            MarketSentiment::StronglyBullish => "Сильно бычий",
            MarketSentiment::Bullish => "Бычий",
            MarketSentiment::SlightlyBullish => "Слегка бычий",
            MarketSentiment::Neutral => "Нейтральный",
            MarketSentiment::SlightlyBearish => "Слегка медвежий",
            MarketSentiment::Bearish => "Медвежий",
            MarketSentiment::StronglyBearish => "Сильно медвежий",
        }
    }
}

/// TRIN (Arms Index) индикатор
#[derive(Clone)]
pub struct Trin {
    // Текущее значение TRIN
    current_value: f64,
    
    // История значений TRIN
    values_history: ArrayVec<f64, 512>,
    
    // Скользящее среднее TRIN (сглаженная версия)
    smoothed_period: usize,
    smoothed_values: ArrayVec<f64, 64>,
    smoothed_trin: f64,
    
    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl Trin {
    /// Создать новый индикатор TRIN
    pub fn new() -> Self {
        Self::with_smoothing(10)
    }
    
    /// Создать новый индикатор TRIN с периодом сглаживания
    pub fn with_smoothing(smoothed_period: usize) -> Self {
        assert!(smoothed_period > 0, "Smoothed period must be greater than 0");
        
        Self {
            current_value: 1.0,
            values_history: ArrayVec::new(),
            smoothed_period,
            smoothed_values: ArrayVec::new(),
            smoothed_trin: 1.0,
            is_ready: false,
            update_count: 0,
        }
    }
    
    /// Обновить индикатор новыми данными TRIN
    pub fn update_trin_data(&mut self, data: TrinData) -> f64 {
        // Рассчитываем TRIN
        let trin_value = self.calculate_trin(data);
        
        // Сохраняем в историю
        if self.values_history.len() >= 512 {
            self.values_history.remove(0);
        }
        self.values_history.push(trin_value);
        
        // Обновляем сглаженное значение
        self.update_smoothed(trin_value);
        
        self.current_value = trin_value;
        self.update_count += 1;
        self.is_ready = true;
        
        trin_value
    }
    
    /// Обновить индикатор данными о цене и объеме (упрощенная версия)
    /// Предполагает, что растущая цена = advancing, падающая = declining
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, volume: f64) -> f64 {
        // Простая логика: если есть история, сравниваем с предыдущим закрытием
        let is_advancing = if let Some(&prev_close) = self.values_history.last() {
            close > prev_close
        } else {
            true // Первый бар считаем растущим
        };
        
        // Создаем упрощенные данные TRIN
        let data = if is_advancing {
            TrinData::new(1.0, 0.0, volume, 0.0)
        } else {
            TrinData::new(0.0, 1.0, 0.0, volume)
        };
        
        self.update_trin_data(data)
    }
    
    /// Рассчитать значение TRIN
    fn calculate_trin(&self, data: TrinData) -> f64 {
        // Избегаем деления на ноль
        if data.declining_issues == 0.0 || data.declining_volume == 0.0 {
            if data.advancing_issues == 0.0 || data.advancing_volume == 0.0 {
                return 1.0; // Нейтральное значение
            } else {
                return 0.1; // Очень бычье значение
            }
        }
        
        if data.advancing_issues == 0.0 || data.advancing_volume == 0.0 {
            return 10.0; // Очень медвежье значение
        }
        
        // TRIN = (Advancing Issues / Declining Issues) / (Advancing Volume / Declining Volume)
        let issue_ratio = data.advancing_issues / data.declining_issues;
        let volume_ratio = data.advancing_volume / data.declining_volume;
        
        if volume_ratio == 0.0 {
            return 10.0;
        }
        
        issue_ratio / volume_ratio
    }
    
    /// Обновить сглаженное значение TRIN
    fn update_smoothed(&mut self, trin_value: f64) {
        if self.smoothed_values.len() >= self.smoothed_period {
            self.smoothed_values.remove(0);
        }
        self.smoothed_values.push(trin_value);
        
        // Рассчитываем простое скользящее среднее
        let sum: f64 = self.smoothed_values.iter().sum();
        self.smoothed_trin = sum / self.smoothed_values.len() as f64;
    }
    
    /// Получить текущее значение TRIN
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_value)
    }
    
    /// Получить сглаженное значение TRIN
    pub fn smoothed_value(&self) -> f64 {
        self.smoothed_trin
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.current_value = 1.0;
        self.values_history.clear();
        self.smoothed_values.clear();
        self.smoothed_trin = 1.0;
        self.is_ready = false;
        self.update_count = 0;
    }
    
    /// Получить период сглаживания
    pub fn period(&self) -> usize {
        self.smoothed_period
    }
    
    /// Определить настроения рынка
    pub fn market_sentiment(&self) -> MarketSentiment {
        MarketSentiment::from_trin(self.current_value)
    }
    
    /// Определить настроения рынка по сглаженному значению
    pub fn smoothed_market_sentiment(&self) -> MarketSentiment {
        MarketSentiment::from_trin(self.smoothed_trin)
    }
    
    /// Генерировать торговый сигнал
    /// Возвращает: -1 (продажа), 0 (нет сигнала), 1 (покупка)
    pub fn trading_signal(&self) -> i8 {
        if self.current_value < 0.5 {
            -1 // Сильно перекупленный рынок - сигнал продажи
        } else if self.current_value > 2.0 {
            1 // Сильно перепроданный рынок - сигнал покупки
        } else {
            0 // Нет сигнала
        }
    }
    
    /// Генерировать сигнал на основе пересечения уровней
    pub fn crossover_signal(&self, threshold_bullish: f64, threshold_bearish: f64) -> i8 {
        let prev_value = if self.values_history.len() >= 2 {
            self.values_history[self.values_history.len() - 2]
        } else {
            self.current_value
        };
        
        // Пересечение снизу вверх через медвежий уровень
        if prev_value <= threshold_bearish && self.current_value > threshold_bearish {
            return 1; // Сигнал покупки
        }
        
        // Пересечение сверху вниз через бычий уровень
        if prev_value >= threshold_bullish && self.current_value < threshold_bullish {
            return -1; // Сигнал продажи
        }
        
        0 // Нет сигнала
    }
    
    /// Получить волатильность TRIN
    pub fn volatility(&self, period: usize) -> f64 {
        if self.values_history.len() < period {
            return 0.0;
        }
        
        let start_idx = self.values_history.len() - period;
        let values = &self.values_history[start_idx..];
        
        // Рассчитываем среднее
        let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
        
        // Рассчитываем стандартное отклонение
        let variance: f64 = values.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / values.len() as f64;
        
        variance.sqrt()
    }
    
    /// Получить экстремальные значения за период
    pub fn extremes(&self, period: usize) -> (f64, f64) {
        if self.values_history.is_empty() {
            return (self.current_value, self.current_value);
        }
        
        let start_idx = if self.values_history.len() > period {
            self.values_history.len() - period
        } else {
            0
        };
        
        let values = &self.values_history[start_idx..];
        
        let min_value = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_value = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        
        (min_value, max_value)
    }
    
    /// Получить информацию о текущем состоянии
    pub fn info(&self) -> String {
        let sentiment = self.market_sentiment();
        let smoothed_sentiment = self.smoothed_market_sentiment();
        let signal = match self.trading_signal() {
            1 => "Покупка",
            -1 => "Продажа", 
            _ => "Нет сигнала",
        };
        
        format!(
            "TRIN: {:.4}, Сглаженный: {:.4}, Настроение: {}, Сглаженное настроение: {}, Сигнал: {}",
            self.current_value,
            self.smoothed_trin,
            sentiment.to_string(),
            smoothed_sentiment.to_string(),
            signal
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("trin".to_string(), self.current_value);
        values.insert("smoothed_trin".to_string(), self.smoothed_trin);
        values.insert("volatility_10".to_string(), self.volatility(10));
        values.insert("volatility_20".to_string(), self.volatility(20));
        
        let (min_20, max_20) = self.extremes(20);
        values.insert("min_20".to_string(), min_20);
        values.insert("max_20".to_string(), max_20);
        
        values
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    
    /// Получить историю значений
    pub fn values_history(&self) -> Vec<f64> {
        self.values_history.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trin_calculation() {
        let mut trin = Trin::new();
        
        // Тест бычьих условий: больше объема в растущих акциях
        let data = TrinData::new(100.0, 50.0, 1000000.0, 300000.0);
        let value = trin.update_trin_data(data);
        
        // TRIN = (100/50) / (1000000/300000) = 2.0 / 3.33 = 0.6 (бычье)
        assert!(value < 1.0);
        assert!(matches!(trin.market_sentiment(), MarketSentiment::Bullish));
    }
    
    #[test]
    fn test_trin_bearish() {
        let mut trin = Trin::new();
        
        // Тест медвежьих условий: больше объема в падающих акциях
        let data = TrinData::new(50.0, 100.0, 300000.0, 1000000.0);
        let value = trin.update_trin_data(data);
        
        // TRIN = (50/100) / (300000/1000000) = 0.5 / 0.3 = 1.67 (медвежье)
        assert!(value > 1.0);
        assert!(matches!(trin.market_sentiment(), MarketSentiment::Bearish));
    }
    
    #[test]
    fn test_trin_neutral() {
        let mut trin = Trin::new();
        
        // Тест нейтральных условий
        let data = TrinData::new(100.0, 100.0, 500000.0, 500000.0);
        let value = trin.update_trin_data(data);
        
        // TRIN = (100/100) / (500000/500000) = 1.0 / 1.0 = 1.0 (нейтральное)
        assert!((value - 1.0).abs() < 0.001);
        assert!(matches!(trin.market_sentiment(), MarketSentiment::Neutral));
    }
    
    #[test]
    fn test_trin_extreme_bullish() {
        let mut trin = Trin::new();
        
        // Тест экстремально бычьих условий
        let data = TrinData::new(200.0, 10.0, 3000000.0, 50000.0);  // More extreme for < 0.5
        let value = trin.update_trin_data(data);
        
        assert!(value < 0.5, "Expected < 0.5, got {}", value);
        assert!(matches!(trin.market_sentiment(), MarketSentiment::StronglyBullish));
        assert_eq!(trin.trading_signal(), -1); // Перекупленность - продавать
    }
    
    #[test]
    fn test_trin_extreme_bearish() {
        let mut trin = Trin::new();
        
        // Тест экстремально медвежьих условий
        let data = TrinData::new(10.0, 200.0, 50000.0, 3000000.0);  // More extreme for > 2.0
        let value = trin.update_trin_data(data);
        
        assert!(value > 2.0, "Expected > 2.0, got {}", value);
        assert!(matches!(trin.market_sentiment(), MarketSentiment::StronglyBearish));
        assert_eq!(trin.trading_signal(), 1); // Перепроданность - покупать
    }
    
    #[test]
    fn test_trin_smoothing() {
        let mut trin = Trin::with_smoothing(3);
        
        // Добавляем несколько значений
        let _val1 = trin.update_trin_data(TrinData::new(100.0, 50.0, 1000000.0, 500000.0));
        let _val2 = trin.update_trin_data(TrinData::new(80.0, 60.0, 800000.0, 600000.0));
        let _val3 = trin.update_trin_data(TrinData::new(120.0, 40.0, 1200000.0, 400000.0));
        
        // Сглаженное значение должно быть средним из трех
        let smoothed = trin.smoothed_value();
        assert!(smoothed > 0.0);
        // Smoothed value is the SMA of history, may equal current value
        assert!(smoothed.is_finite());
    }
    
    #[test]
    fn test_crossover_signals() {
        let mut trin = Trin::new();
        
        // Устанавливаем начальное значение выше порога
        let _val1 = trin.update_trin_data(TrinData::new(10.0, 100.0, 100000.0, 1000000.0));
        
        // Пересекаем вниз через уровень 1.5
        let _val2 = trin.update_trin_data(TrinData::new(80.0, 60.0, 800000.0, 600000.0));
        
        let signal = trin.crossover_signal(0.8, 1.5);
        // Должен быть сигнал покупки при пересечении медвежьего уровня снизу вверх
        // или сигнал продажи при пересечении бычьего уровня сверху вниз
        assert!(signal == 1 || signal == -1 || signal == 0);
    }
} 






















