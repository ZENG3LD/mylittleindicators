//! Volume Weighted RSI - RSI взвешенный по объему
//!
//! Улучшенная версия RSI, которая учитывает объем торгов при расчете.
//! Большие объемы получают больший вес в расчете, что делает индикатор
//! более чувствительным к "умным деньгам".
//!
//! Формула: VWRSI основан на взвешенных по объему изменениях цены
//! Gain_weighted = Gain * Volume, Loss_weighted = Loss * Volume

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::momentum::rsi::Rsi;

/// Результат Volume Weighted RSI
#[derive(Debug, Clone, Copy)]
pub struct VolumeWeightedRsiResult {
    pub vw_rsi: f64,             // Volume Weighted RSI (0-100)
    pub regular_rsi: f64,        // Обычный RSI для сравнения (0-100)
    pub volume_factor: f64,      // Фактор влияния объема (0.0-2.0+)
    pub avg_volume: f64,         // Средний объем за период
    pub volume_trend: i8,        // Тренд объема: 1 (растет), -1 (падает), 0 (стабильный)
    pub smart_money_signal: i8,  // Сигнал "умных денег": 1 (покупают), -1 (продают), 0 (нет)
}

impl VolumeWeightedRsiResult {
    pub fn empty() -> Self {
        Self {
            vw_rsi: 50.0,
            regular_rsi: 50.0,
            volume_factor: 1.0,
            avg_volume: 0.0,
            volume_trend: 0,
            smart_money_signal: 0,
        }
    }
    
    /// Определить состояние рынка с учетом объема
    pub fn market_condition(&self) -> &'static str {
        // Адаптивные уровни в зависимости от объема
        let (oversold, overbought) = if self.volume_factor > 1.2 {
            (25.0, 75.0)  // При высоком объеме более жесткие уровни
        } else if self.volume_factor < 0.8 {
            (35.0, 65.0)  // При низком объеме более мягкие уровни
        } else {
            (30.0, 70.0)  // Стандартные уровни
        };
        
        match self.vw_rsi {
            x if x <= oversold => "Перепродан с объемом",
            x if x >= overbought => "Перекуплен с объемом",
            _ => "Нейтральный",
        }
    }
    
    /// Получить описание сигнала умных денег
    pub fn smart_money_description(&self) -> &'static str {
        match self.smart_money_signal {
            1 => "Умные деньги покупают",
            -1 => "Умные деньги продают",
            _ => "Нет активности умных денег",
        }
    }
}

/// Volume Weighted RSI индикатор
/// 🚀 Refactored: regular RSI теперь использует стандартный Rsi struct
#[derive(Clone)]
pub struct VolumeWeightedRsi {
    // Параметры
    period: usize,
    volume_period: usize,        // Период для анализа объема

    // Буферы для VWRSI (уникальная логика - остается inline)
    vw_gains: ArrayVec<f64, 64>,
    vw_losses: ArrayVec<f64, 64>,
    volumes: ArrayVec<f64, 64>,

    // 🚀 Обычный RSI - используем стандартный struct
    regular_rsi: Rsi,

    // Буферы для обычного RSI (сохранены для volume weighting расчета)
    regular_gains: ArrayVec<f64, 64>,
    regular_losses: ArrayVec<f64, 64>,

    // Средние значения
    avg_vw_gain: f64,
    avg_vw_loss: f64,
    avg_regular_gain: f64,
    avg_regular_loss: f64,
    avg_volume: f64,

    // Данные для расчетов
    prev_close: Option<f64>,

    // Результат
    current_result: VolumeWeightedRsiResult,

    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl VolumeWeightedRsi {
    /// Создать новый Volume Weighted RSI с параметрами по умолчанию
    pub fn new() -> Self {
        Self::with_periods(14, 20)
    }
    
    /// Создать с настраиваемыми периодами
    pub fn with_periods(rsi_period: usize, volume_period: usize) -> Self {
        assert!(rsi_period > 0, "RSI period must be greater than 0");
        assert!(volume_period > 0, "Volume period must be greater than 0");

        Self {
            period: rsi_period,
            volume_period,

            vw_gains: ArrayVec::new(),
            vw_losses: ArrayVec::new(),
            volumes: ArrayVec::new(),
            regular_rsi: Rsi::new(rsi_period), // 🚀 Используем стандартный RSI
            regular_gains: ArrayVec::new(),
            regular_losses: ArrayVec::new(),

            avg_vw_gain: 0.0,
            avg_vw_loss: 0.0,
            avg_regular_gain: 0.0,
            avg_regular_loss: 0.0,
            avg_volume: 0.0,

            prev_close: None,
            current_result: VolumeWeightedRsiResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }
    
    /// Обновить индикатор новым баром
    /// 🚀 Refactored: regular RSI использует стандартный Rsi struct
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, volume: f64) -> VolumeWeightedRsiResult {
        // 🚀 Обновляем стандартный RSI для regular_rsi (already returns 0-100)
        let regular_rsi_value = self.regular_rsi.update_bar(0.0, 0.0, 0.0, close, 0.0);
        self.current_result.regular_rsi = regular_rsi_value;

        // Добавляем объем в буфер
        if self.volumes.len() >= self.volume_period {
            self.volumes.remove(0);
        }
        self.volumes.push(volume);

        // Рассчитываем средний объем
        self.avg_volume = self.volumes.iter().sum::<f64>() / self.volumes.len() as f64;

        if let Some(prev_close) = self.prev_close {
            let price_change = close - prev_close;

            // Рассчитываем обычные gain/loss (нужны для volume weighting)
            let regular_gain = if price_change > 0.0 { price_change } else { 0.0 };
            let regular_loss = if price_change < 0.0 { -price_change } else { 0.0 };
            
            // Рассчитываем взвешенные по объему gain/loss
            let volume_weight = if self.avg_volume > 0.0 { 
                volume / self.avg_volume 
            } else { 
                1.0 
            };
            
            let vw_gain = regular_gain * volume_weight;
            let vw_loss = regular_loss * volume_weight;
            
            // Добавляем в буферы
            self.add_to_buffers(regular_gain, regular_loss, vw_gain, vw_loss);
            
            // Рассчитываем RSI значения
            self.calculate_rsi_values();
            
            // Анализируем объем и умные деньги
            self.analyze_volume_and_smart_money();
        }
        
        self.prev_close = Some(close);
        self.update_count += 1;
        self.current_result
    }
    
    /// Добавить значения в буферы
    fn add_to_buffers(&mut self, regular_gain: f64, regular_loss: f64, vw_gain: f64, vw_loss: f64) {
        // Буферы для обычного RSI
        if self.regular_gains.len() >= self.period {
            self.regular_gains.remove(0);
        }
        self.regular_gains.push(regular_gain);
        
        if self.regular_losses.len() >= self.period {
            self.regular_losses.remove(0);
        }
        self.regular_losses.push(regular_loss);
        
        // Буферы для VWRSI
        if self.vw_gains.len() >= self.period {
            self.vw_gains.remove(0);
        }
        self.vw_gains.push(vw_gain);
        
        if self.vw_losses.len() >= self.period {
            self.vw_losses.remove(0);
        }
        self.vw_losses.push(vw_loss);
    }
    
    /// Рассчитать значения RSI
    fn calculate_rsi_values(&mut self) {
        if self.regular_gains.len() == self.period {
            // Рассчитываем обычный RSI
            if self.avg_regular_gain == 0.0 && self.avg_regular_loss == 0.0 {
                // Первый расчет - простое среднее
                self.avg_regular_gain = self.regular_gains.iter().sum::<f64>() / self.period as f64;
                self.avg_regular_loss = self.regular_losses.iter().sum::<f64>() / self.period as f64;
                self.avg_vw_gain = self.vw_gains.iter().sum::<f64>() / self.period as f64;
                self.avg_vw_loss = self.vw_losses.iter().sum::<f64>() / self.period as f64;
            } else {
                // Экспоненциальное сглаживание (Wilder's method)
                let alpha = 1.0 / self.period as f64;
                
                let latest_regular_gain = self.regular_gains[self.regular_gains.len() - 1];
                let latest_regular_loss = self.regular_losses[self.regular_losses.len() - 1];
                let latest_vw_gain = self.vw_gains[self.vw_gains.len() - 1];
                let latest_vw_loss = self.vw_losses[self.vw_losses.len() - 1];
                
                self.avg_regular_gain = alpha * latest_regular_gain + (1.0 - alpha) * self.avg_regular_gain;
                self.avg_regular_loss = alpha * latest_regular_loss + (1.0 - alpha) * self.avg_regular_loss;
                self.avg_vw_gain = alpha * latest_vw_gain + (1.0 - alpha) * self.avg_vw_gain;
                self.avg_vw_loss = alpha * latest_vw_loss + (1.0 - alpha) * self.avg_vw_loss;
            }

            // 🚀 Regular RSI уже рассчитан через стандартный Rsi struct (в начале update_bar)

            // Рассчитываем Volume Weighted RSI
            self.current_result.vw_rsi = if self.avg_vw_loss == 0.0 {
                100.0
            } else {
                let vw_rs = self.avg_vw_gain / self.avg_vw_loss;
                100.0 - (100.0 / (1.0 + vw_rs))
            };
            
            self.is_ready = true;
        }
    }
    
    /// Анализировать объем и активность умных денег
    fn analyze_volume_and_smart_money(&mut self) {
        if self.volumes.len() < 3 {
            return;
        }
        
        // Рассчитываем фактор влияния объема
        let current_volume = self.volumes[self.volumes.len() - 1];
        self.current_result.volume_factor = if self.avg_volume > 0.0 {
            current_volume / self.avg_volume
        } else {
            1.0
        };
        
        // Определяем тренд объема
        let recent_volumes = &self.volumes[self.volumes.len().saturating_sub(3)..];
        if recent_volumes.len() >= 3 {
            let volume_change1 = recent_volumes[1] - recent_volumes[0];
            let volume_change2 = recent_volumes[2] - recent_volumes[1];
            
            if volume_change1 > 0.0 && volume_change2 > 0.0 {
                self.current_result.volume_trend = 1;  // Объем растет
            } else if volume_change1 < 0.0 && volume_change2 < 0.0 {
                self.current_result.volume_trend = -1; // Объем падает
            } else {
                self.current_result.volume_trend = 0;  // Объем стабильный
            }
        }
        
        // Анализируем активность умных денег
        self.analyze_smart_money();
    }
    
    /// Анализировать активность умных денег
    fn analyze_smart_money(&mut self) {
        if !self.is_ready {
            return;
        }
        
        let vw_rsi = self.current_result.vw_rsi;
        let regular_rsi = self.current_result.regular_rsi;
        let volume_factor = self.current_result.volume_factor;
        
        // Умные деньги активны при:
        // 1. Высоком объеме (volume_factor > 1.5)
        // 2. Расхождении между VWRSI и обычным RSI
        
        if volume_factor > 1.5 {
            let rsi_divergence = vw_rsi - regular_rsi;
            
            if rsi_divergence > 5.0 && vw_rsi < 40.0 {
                // VWRSI значительно выше обычного RSI в зоне перепроданности
                // Умные деньги покупают на падении
                self.current_result.smart_money_signal = 1;
            } else if rsi_divergence < -5.0 && vw_rsi > 60.0 {
                // VWRSI значительно ниже обычного RSI в зоне перекупленности
                // Умные деньги продают на росте
                self.current_result.smart_money_signal = -1;
            } else {
                self.current_result.smart_money_signal = 0;
            }
        } else {
            self.current_result.smart_money_signal = 0;
        }
        
        // Обновляем средний объем в результате
        self.current_result.avg_volume = self.avg_volume;
    }
    
    /// Получить текущее значение VWRSI
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.vw_rsi)
    }
    
    /// Получить полный результат
    pub fn result(&self) -> VolumeWeightedRsiResult {
        self.current_result
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.regular_rsi.reset(); // 🚀 Сбрасываем стандартный RSI
        self.vw_gains.clear();
        self.vw_losses.clear();
        self.volumes.clear();
        self.regular_gains.clear();
        self.regular_losses.clear();

        self.avg_vw_gain = 0.0;
        self.avg_vw_loss = 0.0;
        self.avg_regular_gain = 0.0;
        self.avg_regular_loss = 0.0;
        self.avg_volume = 0.0;

        self.prev_close = None;
        self.current_result = VolumeWeightedRsiResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Генерировать торговый сигнал с учетом объема
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let result = self.current_result;
        
        // Адаптивные уровни в зависимости от объема
        let (oversold, overbought) = if result.volume_factor > 1.2 {
            (25.0, 75.0)  // При высоком объеме более жесткие уровни
        } else if result.volume_factor < 0.8 {
            (35.0, 65.0)  // При низком объеме более мягкие уровни
        } else {
            (30.0, 70.0)  // Стандартные уровни
        };
        
        if result.vw_rsi <= oversold && result.volume_factor > 1.0 {
            return 1; // Покупка при перепроданности с объемом
        } else if result.vw_rsi >= overbought && result.volume_factor > 1.0 {
            return -1; // Продажа при перекупленности с объемом
        }
        
        0
    }
    
    /// Генерировать сигнал умных денег
    pub fn smart_money_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        self.current_result.smart_money_signal
    }
    
    /// Генерировать дивергентный сигнал (VWRSI vs обычный RSI)
    pub fn divergence_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let result = self.current_result;
        let divergence = result.vw_rsi - result.regular_rsi;
        
        // Значительная дивергенция при достаточном объеме
        if result.volume_factor > 1.2 {
            if divergence > 10.0 {
                return 1; // VWRSI сильно выше - покупка
            } else if divergence < -10.0 {
                return -1; // VWRSI сильно ниже - продажа
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
            "VW-RSI: {:.1}, RSI: {:.1}, Объем: {:.1}x, Состояние: {}, {}, Сигнал: {}",
            result.vw_rsi,
            result.regular_rsi,
            result.volume_factor,
            result.market_condition(),
            result.smart_money_description(),
            signal
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("vw_rsi".to_string(), self.current_result.vw_rsi);
        values.insert("regular_rsi".to_string(), self.current_result.regular_rsi);
        values.insert("volume_factor".to_string(), self.current_result.volume_factor);
        values.insert("avg_volume".to_string(), self.current_result.avg_volume);
        values.insert("volume_trend".to_string(), self.current_result.volume_trend as f64);
        values.insert("smart_money_signal".to_string(), self.current_result.smart_money_signal as f64);
        values.insert("rsi_divergence".to_string(), self.current_result.vw_rsi - self.current_result.regular_rsi);
        values
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    
    /// Получить параметры
    pub fn parameters(&self) -> (usize, usize) {
        (self.period, self.volume_period)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_weighted_rsi_creation() {
        let vw_rsi = VolumeWeightedRsi::new();
        assert!(!vw_rsi.is_ready());
        assert_eq!(vw_rsi.parameters(), (14, 20));
    }
    
    #[test]
    fn test_volume_weighted_rsi_with_periods() {
        let vw_rsi = VolumeWeightedRsi::with_periods(21, 30);
        assert_eq!(vw_rsi.parameters(), (21, 30));
    }
    
    #[test]
    fn test_volume_weighted_rsi_update() {
        let mut vw_rsi = VolumeWeightedRsi::new();
        
        // Добавляем данные с изменяющимся объемом
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            let volume = 1000.0 + (i as f64 * 0.2).cos() * 500.0;
            
            let result = vw_rsi.update_bar(price, price + 0.5, price - 0.5, price, volume);
            
            if i > 15 {
                assert!(vw_rsi.is_ready());
                assert!(result.vw_rsi >= 0.0 && result.vw_rsi <= 100.0);
                assert!(result.regular_rsi >= 0.0 && result.regular_rsi <= 100.0);
                assert!(result.volume_factor >= 0.0);
                assert!(result.avg_volume > 0.0);
            }
        }
    }
    
    #[test]
    fn test_smart_money_detection() {
        let mut vw_rsi = VolumeWeightedRsi::new();
        
        // Симулируем активность умных денег: падающие цены с высоким объемом
        let mut price = 100.0;
        for i in 0..20 {
            price -= 0.5; // Падающие цены
            let volume = if i > 10 { 2000.0 } else { 1000.0 }; // Высокий объем в конце
            
            let result = vw_rsi.update_bar(price, price + 0.1, price - 0.1, price, volume);
            
            if i > 15 && vw_rsi.is_ready() {
                // При падающих ценах и высоком объеме может быть сигнал умных денег
                assert!(result.volume_factor >= 0.0);
                assert!(result.smart_money_signal >= -1 && result.smart_money_signal <= 1);
            }
        }
    }
    
    #[test]
    fn test_trading_signals() {
        let mut vw_rsi = VolumeWeightedRsi::new();
        
        // Создаем условия для перепроданности с объемом
        let mut price = 100.0;
        for i in 0..20 {
            price -= 1.0;
            let volume = 1500.0; // Повышенный объем
            
            let _result = vw_rsi.update_bar(price, price + 0.1, price - 0.1, price, volume);
            
            if i > 15 && vw_rsi.is_ready() {
                let signal = vw_rsi.trading_signal();
                let smart_signal = vw_rsi.smart_money_signal();
                let div_signal = vw_rsi.divergence_signal();
                
                assert!(signal >= -1 && signal <= 1);
                assert!(smart_signal >= -1 && smart_signal <= 1);
                assert!(div_signal >= -1 && div_signal <= 1);
            }
        }
    }
    
    #[test]
    fn test_volume_analysis() {
        let mut vw_rsi = VolumeWeightedRsi::new();
        
        // Тестируем различные объемы
        let volumes = [500.0, 1000.0, 1500.0, 2000.0, 1000.0];
        for (i, &volume) in volumes.iter().enumerate() {
            let price = 100.0 + i as f64;
            let result = vw_rsi.update_bar(price, price + 0.5, price - 0.5, price, volume);
            
            if i > 2 {
                assert!(result.volume_factor > 0.0);
                assert!(result.volume_trend >= -1 && result.volume_trend <= 1);
            }
        }
    }
} 






















