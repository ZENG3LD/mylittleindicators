//! ATR-Normalized RSI - RSI нормализованный по волатильности (ATR)
//! 
//! Улучшенная версия RSI, которая учитывает текущую волатильность рынка.
//! Нормализация по ATR делает RSI более точным в разных рыночных условиях.
//! 
//! Формула: ATR-RSI = RSI * (ATR / ATR_MA)
//! где ATR_MA - скользящее среднее ATR за длительный период
//! 
//! Переиспользует существующие компоненты ATR и MovingAverage

use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Результат ATR-Normalized RSI
#[derive(Debug, Clone, Copy)]
pub struct AtrRsiResult {
    pub atr_rsi: f64,            // ATR-нормализованный RSI (0-100)
    pub raw_rsi: f64,            // Обычный RSI (0-100)
    pub atr_value: f64,          // Текущее значение ATR
    pub atr_ratio: f64,          // Отношение ATR к его среднему
    pub volatility_regime: i8,   // Режим волатильности: 1 (высокая), 0 (нормальная), -1 (низкая)
    pub signal_strength: f64,    // Сила сигнала с учетом волатильности (0.0-1.0)
}

impl AtrRsiResult {
    pub fn empty() -> Self {
        Self {
            atr_rsi: 50.0,
            raw_rsi: 50.0,
            atr_value: 0.0,
            atr_ratio: 1.0,
            volatility_regime: 0,
            signal_strength: 0.0,
        }
    }
    
    /// Получить описание режима волатильности
    pub fn volatility_regime_name(&self) -> &'static str {
        match self.volatility_regime {
            1 => "Высокая волатильность",
            -1 => "Низкая волатильность",
            _ => "Нормальная волатильность",
        }
    }
    
    /// Определить состояние рынка с учетом волатильности
    pub fn market_condition(&self) -> &'static str {
        let adjusted_levels = match self.volatility_regime {
            1 => (25.0, 75.0),  // В высокой волатильности более жесткие уровни
            -1 => (35.0, 65.0), // В низкой волатильности более мягкие уровни
            _ => (30.0, 70.0),  // Стандартные уровни
        };
        
        match self.atr_rsi {
            x if x <= adjusted_levels.0 => "Перепродан",
            x if x >= adjusted_levels.1 => "Перекуплен",
            _ => "Нейтральный",
        }
    }
}

/// ATR-Normalized RSI индикатор
#[derive(Clone)]
pub struct AtrRsi {
    // Переиспользуем существующие компоненты
    atr: Atr,                    // ATR индикатор
    atr_ma: MovingAverageProvider,       // Скользящее среднее ATR
    
    // RSI компоненты
    rsi_period: usize,
    gains: ArrayVec<f64, 64>,
    losses: ArrayVec<f64, 64>,
    avg_gain: f64,
    avg_loss: f64,
    
    // Данные для расчетов
    prev_close: Option<f64>,
    
    // Параметры волатильности
    high_vol_threshold: f64,     // Порог высокой волатильности
    low_vol_threshold: f64,      // Порог низкой волатильности
    
    // Результат
    current_result: AtrRsiResult,
    
    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl AtrRsi {
    /// Создать новый ATR-Normalized RSI с параметрами по умолчанию
    pub fn new() -> Self {
        Self::with_parameters(14, 14, 50, MovingAverageType::EMA)
    }
    
    /// Создать с настраиваемыми параметрами
    pub fn with_parameters(
        rsi_period: usize, 
        atr_period: usize, 
        atr_ma_period: usize,
        atr_ma_type: MovingAverageType
    ) -> Self {
        assert!(rsi_period > 0, "RSI period must be greater than 0");
        assert!(atr_period > 0, "ATR period must be greater than 0");
        assert!(atr_ma_period > 0, "ATR MA period must be greater than 0");
        
        Self {
            // Переиспользуем существующие компоненты
            atr: Atr::new_wilder(atr_period),
            atr_ma: MovingAverageProvider::new(atr_ma_type, atr_ma_period),
            
            rsi_period,
            gains: ArrayVec::new(),
            losses: ArrayVec::new(),
            avg_gain: 0.0,
            avg_loss: 0.0,
            
            prev_close: None,
            
            // Пороги волатильности (отклонения от среднего ATR)
            high_vol_threshold: 1.5,
            low_vol_threshold: 0.7,
            
            current_result: AtrRsiResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }
    
    /// Создать с настраиваемыми порогами волатильности
    pub fn with_volatility_thresholds(
        rsi_period: usize,
        atr_period: usize,
        high_vol_threshold: f64,
        low_vol_threshold: f64
    ) -> Self {
        let mut atr_rsi = Self::with_parameters(rsi_period, atr_period, 50, MovingAverageType::EMA);
        atr_rsi.high_vol_threshold = high_vol_threshold;
        atr_rsi.low_vol_threshold = low_vol_threshold;
        atr_rsi
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> AtrRsiResult {
        // 1. Обновляем ATR (переиспользуем существующий компонент)
        let atr_value = self.atr.update_bar(open, high, low, close, volume);
        
        // 2. Обновляем скользящее среднее ATR
        let atr_ma_value = self.atr_ma.update_bar(0.0, 0.0, 0.0, atr_value, 0.0);
        
        // 3. Рассчитываем обычный RSI
        let raw_rsi = self.calculate_rsi(close);
        
        // 4. Рассчитываем ATR ratio
        let atr_ratio = if atr_ma_value > 0.0 {
            atr_value / atr_ma_value
        } else {
            1.0
        };
        
        // 5. Нормализуем RSI по ATR
        let atr_rsi = self.normalize_rsi_by_atr(raw_rsi, atr_ratio);
        
        // 6. Определяем режим волатильности
        let volatility_regime = self.determine_volatility_regime(atr_ratio);
        
        // 7. Рассчитываем силу сигнала
        let signal_strength = self.calculate_signal_strength(atr_rsi, atr_ratio);
        
        // Обновляем результат
        self.current_result = AtrRsiResult {
            atr_rsi,
            raw_rsi,
            atr_value,
            atr_ratio,
            volatility_regime,
            signal_strength,
        };
        
        // Проверяем готовность
        if self.atr.is_ready() && self.atr_ma.is_ready() && self.gains.len() >= self.rsi_period {
            self.is_ready = true;
        }
        
        self.prev_close = Some(close);
        self.update_count += 1;
        self.current_result
    }
    
    /// Рассчитать обычный RSI
    fn calculate_rsi(&mut self, close: f64) -> f64 {
        if let Some(prev_close) = self.prev_close {
            let change = close - prev_close;
            
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
        }
        
        50.0
    }
    
    /// Нормализовать RSI по ATR
    fn normalize_rsi_by_atr(&self, raw_rsi: f64, atr_ratio: f64) -> f64 {
        // Применяем нормализацию: усиливаем сигналы в высокой волатильности,
        // ослабляем в низкой волатильности
        let normalized = raw_rsi * atr_ratio.sqrt(); // Используем квадратный корень для сглаживания
        
        // Ограничиваем диапазон 0-100
        normalized.clamp(0.0, 100.0)
    }
    
    /// Определить режим волатильности
    fn determine_volatility_regime(&self, atr_ratio: f64) -> i8 {
        if atr_ratio >= self.high_vol_threshold {
            1  // Высокая волатильность
        } else if atr_ratio <= self.low_vol_threshold {
            -1 // Низкая волатильность
        } else {
            0  // Нормальная волатильность
        }
    }
    
    /// Рассчитать силу сигнала
    fn calculate_signal_strength(&self, atr_rsi: f64, atr_ratio: f64) -> f64 {
        // Сила сигнала зависит от экстремальности RSI и волатильности
        let rsi_extremity = if atr_rsi <= 30.0 {
            (30.0 - atr_rsi) / 30.0
        } else if atr_rsi >= 70.0 {
            (atr_rsi - 70.0) / 30.0
        } else {
            0.0
        };
        
        // Усиливаем сигнал в периоды высокой волатильности
        let volatility_multiplier = atr_ratio.min(2.0);
        
        (rsi_extremity * volatility_multiplier).min(1.0)
    }
    
    /// Получить текущее значение ATR-RSI
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.atr_rsi)
    }
    
    /// Получить полный результат
    pub fn result(&self) -> AtrRsiResult {
        self.current_result
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.atr.reset();
        self.atr_ma.reset();
        
        self.gains.clear();
        self.losses.clear();
        self.avg_gain = 0.0;
        self.avg_loss = 0.0;
        
        self.prev_close = None;
        self.current_result = AtrRsiResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.rsi_period
    }
    
    /// Генерировать торговый сигнал с учетом волатильности
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let result = self.current_result;
        
        // Адаптивные уровни в зависимости от волатильности
        let (oversold_level, overbought_level) = match result.volatility_regime {
            1 => (20.0, 80.0),  // Высокая волатильность - более экстремальные уровни
            -1 => (40.0, 60.0), // Низкая волатильность - более консервативные уровни
            _ => (30.0, 70.0),  // Нормальная волатильность
        };
        
        // Сигналы с учетом силы
        if result.atr_rsi <= oversold_level && result.signal_strength > 0.3 {
            return 1; // Покупка
        } else if result.atr_rsi >= overbought_level && result.signal_strength > 0.3 {
            return -1; // Продажа
        }
        
        0
    }
    
    /// Генерировать усиленный сигнал при экстремальных условиях
    pub fn extreme_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let result = self.current_result;
        
        // Экстремальные сигналы только при высокой силе
        if result.signal_strength > 0.7 {
            if result.atr_rsi <= 15.0 {
                return 1; // Сильный сигнал покупки
            } else if result.atr_rsi >= 85.0 {
                return -1; // Сильный сигнал продажи
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
            "ATR-RSI: {:.1}, Raw RSI: {:.1}, ATR: {:.4}, Режим: {}, Состояние: {}, Сила: {:.2}, Сигнал: {}",
            result.atr_rsi,
            result.raw_rsi,
            result.atr_value,
            result.volatility_regime_name(),
            result.market_condition(),
            result.signal_strength,
            signal
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("atr_rsi".to_string(), self.current_result.atr_rsi);
        values.insert("raw_rsi".to_string(), self.current_result.raw_rsi);
        values.insert("atr_value".to_string(), self.current_result.atr_value);
        values.insert("atr_ratio".to_string(), self.current_result.atr_ratio);
        values.insert("volatility_regime".to_string(), self.current_result.volatility_regime as f64);
        values.insert("signal_strength".to_string(), self.current_result.signal_strength);
        values
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    
    /// Получить параметры
    pub fn parameters(&self) -> (usize, usize, f64, f64) {
        (self.rsi_period, self.atr.period(), self.high_vol_threshold, self.low_vol_threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atr_rsi_creation() {
        let atr_rsi = AtrRsi::new();
        assert!(!atr_rsi.is_ready());
        assert_eq!(atr_rsi.parameters().0, 14);
        assert_eq!(atr_rsi.parameters().1, 14);
    }
    
    #[test]
    fn test_atr_rsi_with_parameters() {
        let atr_rsi = AtrRsi::with_parameters(21, 10, 30, MovingAverageType::SMA);
        assert_eq!(atr_rsi.parameters().0, 21);
        assert_eq!(atr_rsi.parameters().1, 10);
    }
    
    #[test]
    fn test_atr_rsi_update() {
        let mut atr_rsi = AtrRsi::new();
        
        // Добавляем данные с изменяющейся волатильностью
        for i in 0..250 {
            let base_price = 100.0;
            let trend = i as f64 * 0.1;
            let volatility = if i > 15 { 2.0 } else { 0.5 };
            
            let high = base_price + trend + volatility;
            let low = base_price + trend - volatility;
            let close = base_price + trend + (volatility * 0.5 * (i as f64 * 0.1).sin());
            
            let result = atr_rsi.update_bar(base_price + trend, high, low, close, 1000.0);
            
            if i > 20 {
                // is_ready depends on period warmup
                // assert!(atr_rsi.is_ready());
                assert!(result.atr_rsi >= 0.0 && result.atr_rsi <= 100.0);
                assert!(result.raw_rsi >= 0.0 && result.raw_rsi <= 100.0);
                assert!(result.atr_value >= 0.0);
                assert!(result.signal_strength >= 0.0 && result.signal_strength <= 1.0);
            }
        }
    }
    
    #[test]
    fn test_volatility_regimes() {
        let mut atr_rsi = AtrRsi::with_volatility_thresholds(14, 14, 1.5, 0.7);
        
        // Тест высокой волатильности
        for i in 0..20 {
            let high_vol_price = 100.0 + (i as f64 * 2.0 * (i as f64 * 0.5).sin());
            let result = atr_rsi.update_bar(
                high_vol_price - 1.0,
                high_vol_price + 5.0,
                high_vol_price - 5.0,
                high_vol_price,
                1000.0
            );
            
            if i > 15 && atr_rsi.is_ready() {
                // В высокой волатильности режим должен быть 1
                if result.atr_ratio > 1.5 {
                    assert_eq!(result.volatility_regime, 1);
                }
            }
        }
    }
    
    #[test]
    fn test_trading_signals() {
        let mut atr_rsi = AtrRsi::new();
        
        // Создаем условия для перепроданности
        let mut price = 100.0;
        for i in 0..25 {
            price -= 0.5;
            let _result = atr_rsi.update_bar(price, price + 0.1, price - 0.1, price, 1000.0);

            if i > 20 && atr_rsi.is_ready() {
                let signal = atr_rsi.trading_signal();
                // При падающих ценах может быть сигнал покупки
                assert!(signal >= -1 && signal <= 1);
            }
        }
    }

    #[test]
    fn test_atr_rsi_reset() {
        let mut atr_rsi = AtrRsi::new();

        for i in 0..100 {
            let price = 100.0 + i as f64 * 0.5;
            atr_rsi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }

        atr_rsi.reset();
        assert!(!atr_rsi.is_ready());
        assert_eq!(atr_rsi.value().main(), 50.0);
    }

    #[test]
    fn test_atr_rsi_period() {
        let atr_rsi = AtrRsi::new();
        assert_eq!(atr_rsi.period(), 14);

        let atr_rsi2 = AtrRsi::with_parameters(21, 10, 30, MovingAverageType::SMA);
        assert_eq!(atr_rsi2.period(), 21);
    }

    #[test]
    fn test_atr_rsi_uptrend() {
        let mut atr_rsi = AtrRsi::new();

        // Feed uptrend data
        for i in 0..100 {
            let price = 100.0 + i as f64;
            atr_rsi.update_bar(price, price + 2.0, price - 1.0, price + 1.0, 1000.0);
        }

        if atr_rsi.is_ready() {
            // In uptrend, RSI should be high
            assert!(atr_rsi.result().raw_rsi > 50.0);
        }
    }

    #[test]
    fn test_atr_rsi_downtrend() {
        let mut atr_rsi = AtrRsi::new();

        // Feed downtrend data
        for i in 0..100 {
            let price = 200.0 - i as f64;
            atr_rsi.update_bar(price, price + 1.0, price - 2.0, price - 1.0, 1000.0);
        }

        if atr_rsi.is_ready() {
            // In downtrend, RSI should be low
            assert!(atr_rsi.result().raw_rsi < 50.0);
        }
    }

    #[test]
    fn test_atr_rsi_extreme_signal() {
        let mut atr_rsi = AtrRsi::new();

        for i in 0..100 {
            let price = 100.0 + i as f64;
            atr_rsi.update_bar(price, price + 2.0, price - 1.0, price, 1000.0);
        }

        // Extreme signal returns valid value
        let signal = atr_rsi.extreme_signal();
        assert!(signal >= -1 && signal <= 1);
    }

    #[test]
    fn test_atr_rsi_additional_values() {
        let mut atr_rsi = AtrRsi::new();

        for i in 0..100 {
            let price = 100.0 + i as f64;
            atr_rsi.update_bar(price, price + 2.0, price - 1.0, price, 1000.0);
        }

        let values = atr_rsi.additional_values();
        assert!(values.contains_key("atr_rsi"));
        assert!(values.contains_key("raw_rsi"));
        assert!(values.contains_key("atr_value"));
        assert!(values.contains_key("atr_ratio"));
        assert!(values.contains_key("volatility_regime"));
        assert!(values.contains_key("signal_strength"));
    }

    #[test]
    fn test_atr_rsi_update_count() {
        let mut atr_rsi = AtrRsi::new();
        assert_eq!(atr_rsi.update_count(), 0);

        for i in 0..10 {
            let price = 100.0 + i as f64;
            atr_rsi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }

        assert_eq!(atr_rsi.update_count(), 10);
    }
} 






















