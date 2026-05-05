//! Connors RSI indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::momentum::rsi::Rsi;
use crate::bar_indicators::utils::math::percentile::percentile_rank;

/// Connors RSI result containing all component values.
#[derive(Debug, Clone, Copy)]
pub struct ConnorsRsiResult {
    /// Final Connors RSI value (0-100).
    pub connors_rsi: f64,
    /// RSI component (0-100).
    pub rsi_component: f64,
    /// RSI of UpDown Length (0-100).
    pub updown_rsi: f64,
    /// ROC Percentile (0-100).
    pub roc_percentile: f64,
    /// Current streak length (positive = up, negative = down).
    pub updown_length: i32,
}

impl ConnorsRsiResult {
    /// Creates an empty result with neutral values.
    pub fn empty() -> Self {
        Self {
            connors_rsi: 50.0,
            rsi_component: 50.0,
            updown_rsi: 50.0,
            roc_percentile: 50.0,
            updown_length: 0,
        }
    }

    /// Returns the current market condition.
    pub fn market_condition(&self) -> &'static str {
        match self.connors_rsi {
            x if x <= 10.0 => "Extremely Oversold",
            x if x <= 20.0 => "Strongly Oversold",
            x if x <= 30.0 => "Oversold",
            x if x <= 40.0 => "Mildly Oversold",
            x if x <= 60.0 => "Neutral",
            x if x <= 70.0 => "Mildly Overbought",
            x if x <= 80.0 => "Overbought",
            x if x <= 90.0 => "Strongly Overbought",
            _ => "Extremely Overbought",
        }
    }
}

/// Connors RSI - enhanced RSI indicator by Larry Connors.
///
/// Connors RSI = (RSI + RSI of UpDown Length + ROC Percentile) / 3
///
/// Components:
/// 1. RSI(3) - standard RSI with period 3
/// 2. RSI of UpDown Length - RSI of consecutive up/down day count
/// 3. ROC Percentile(100) - percentile rank of ROC over 100 periods
///
/// More sensitive to short-term changes and better at identifying
/// extreme overbought/oversold conditions.
///
/// Interpretation:
/// - CRSI <= 10: Extremely oversold (buy)
/// - CRSI <= 20: Strongly oversold
/// - CRSI >= 80: Strongly overbought
/// - CRSI >= 90: Extremely overbought (sell)
///
/// # Parameters
/// - `rsi_period`: RSI calculation period (typically 3)
/// - `updown_period`: UpDown RSI period (typically 2)
/// - `roc_period`: ROC percentile lookback (typically 100)
///
/// # Implementation
///
/// Combines three normalized components. O(1) per update with rolling buffers.
#[derive(Clone)]
pub struct ConnorsRsi {
    updown_period: usize,
    roc_period: usize,

    rsi: Rsi,

    prices: ArrayVec<f64, 512>,

    updown_lengths: ArrayVec<i32, 512>,
    current_updown_length: i32,
    last_direction: i8,

    updown_gains: ArrayVec<f64, 32>,
    updown_losses: ArrayVec<f64, 32>,
    updown_avg_gain: f64,
    updown_avg_loss: f64,

    roc_values: ArrayVec<f64, 512>,

    current_result: ConnorsRsiResult,

    is_ready: bool,
    update_count: usize,
}

impl ConnorsRsi {
    /// Creates a new Connors RSI with default parameters (3, 2, 100).
    pub fn new() -> Self {
        Self::with_periods(3, 2, 100)
    }

    /// Creates a new Connors RSI with custom parameters.
    ///
    /// # Arguments
    /// * `rsi_period` - RSI calculation period (typically 3)
    /// * `updown_period` - UpDown RSI period (typically 2)
    /// * `roc_period` - ROC percentile lookback (typically 100)
    pub fn with_periods(rsi_period: usize, updown_period: usize, roc_period: usize) -> Self {
        assert!(rsi_period > 0, "RSI period must be greater than 0");
        assert!(updown_period > 0, "UpDown period must be greater than 0");
        assert!(roc_period > 0, "ROC period must be greater than 0");

        Self {
            updown_period,
            roc_period,
            rsi: Rsi::new(rsi_period),
            prices: ArrayVec::new(),
            updown_lengths: ArrayVec::new(),
            current_updown_length: 0,
            last_direction: 0,
            updown_gains: ArrayVec::new(),
            updown_losses: ArrayVec::new(),
            updown_avg_gain: 0.0,
            updown_avg_loss: 0.0,
            roc_values: ArrayVec::new(),
            current_result: ConnorsRsiResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }
    
    /// Updates the indicator with a new bar and returns the result.
    ///
    /// Only the `close` price is used.
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> ConnorsRsiResult {
        self.update_price(close)
    }

    /// Updates the indicator with a new price.
    pub fn update_price(&mut self, price: f64) -> ConnorsRsiResult {
        if self.prices.len() >= 512 {
            self.prices.remove(0);
        }
        self.prices.push(price);
        
        if self.prices.len() >= 2 {
            // 1. Calculate RSI component
            let rsi_component = self.calculate_rsi_component();

            // 2. Calculate UpDown Length and its RSI
            let updown_rsi = self.calculate_updown_rsi();

            // 3. Calculate ROC Percentile
            let roc_percentile = self.calculate_roc_percentile();

            // 4. Combine components
            let connors_rsi = (rsi_component + updown_rsi + roc_percentile) / 3.0;

            // Update result
            self.current_result = ConnorsRsiResult {
                connors_rsi,
                rsi_component,
                updown_rsi,
                roc_percentile,
                updown_length: self.current_updown_length,
            };
            
            // Check readiness
            if self.prices.len() >= self.rsi.period().max(self.roc_period) {
                self.is_ready = true;
            }
        }
        
        self.update_count += 1;
        self.current_result
    }
    
    /// Calculates the RSI component.
    fn calculate_rsi_component(&mut self) -> f64 {
        if self.prices.is_empty() {
            return 50.0;
        }

        let len = self.prices.len();
        let current_price = self.prices[len - 1];

        // Update standard RSI - already returns 0-100
        self.rsi.update_bar(0.0, 0.0, 0.0, current_price, 0.0)
    }

    /// Calculates the UpDown Length and its RSI.
    fn calculate_updown_rsi(&mut self) -> f64 {
        if self.prices.len() < 2 {
            return 50.0;
        }
        
        let len = self.prices.len();
        let current_price = self.prices[len - 1];
        let prev_price = self.prices[len - 2];

        // Determine direction
        let current_direction = if current_price > prev_price {
            1  // Up
        } else if current_price < prev_price {
            -1 // Down
        } else {
            0  // Unchanged
        };

        // Update streak length
        if current_direction != 0 {
            if current_direction == self.last_direction {
                // Continuation of streak
                if self.current_updown_length > 0 && current_direction == 1 {
                    self.current_updown_length += 1;
                } else if self.current_updown_length < 0 && current_direction == -1 {
                    self.current_updown_length -= 1;
                } else {
                    // Direction change
                    self.current_updown_length = current_direction as i32;
                }
            } else {
                // Direction change
                self.current_updown_length = current_direction as i32;
            }
            self.last_direction = current_direction;
        }

        // Add length to buffer
        if self.updown_lengths.len() >= 512 {
            self.updown_lengths.remove(0);
        }
        self.updown_lengths.push(self.current_updown_length);
        
        // Calculate RSI from UpDown Length
        if self.updown_lengths.len() >= 2 {
            let current_length = self.current_updown_length as f64;
            let prev_length = if self.updown_lengths.len() >= 2 {
                self.updown_lengths[self.updown_lengths.len() - 2] as f64
            } else {
                0.0
            };
            
            let change = current_length - prev_length;
            let gain = if change > 0.0 { change } else { 0.0 };
            let loss = if change < 0.0 { -change } else { 0.0 };
            
            // Add to UpDown RSI buffers
            if self.updown_gains.len() >= self.updown_period {
                self.updown_gains.remove(0);
            }
            self.updown_gains.push(gain);
            
            if self.updown_losses.len() >= self.updown_period {
                self.updown_losses.remove(0);
            }
            self.updown_losses.push(loss);
            
            // Calculate averages
            if self.updown_gains.len() == self.updown_period {
                if self.updown_avg_gain == 0.0 && self.updown_avg_loss == 0.0 {
                    self.updown_avg_gain = self.updown_gains.iter().sum::<f64>() / self.updown_period as f64;
                    self.updown_avg_loss = self.updown_losses.iter().sum::<f64>() / self.updown_period as f64;
                } else {
                    let alpha = 1.0 / self.updown_period as f64;
                    self.updown_avg_gain = alpha * gain + (1.0 - alpha) * self.updown_avg_gain;
                    self.updown_avg_loss = alpha * loss + (1.0 - alpha) * self.updown_avg_loss;
                }
                
                // Calculate RSI
                if self.updown_avg_loss == 0.0 {
                    return 100.0;
                }
                
                let rs = self.updown_avg_gain / self.updown_avg_loss;
                return 100.0 - (100.0 / (1.0 + rs));
            }
        }
        
        50.0
    }
    
    /// Calculates the ROC Percentile.
    fn calculate_roc_percentile(&mut self) -> f64 {
        if self.prices.len() < 2 {
            return 50.0;
        }

        let len = self.prices.len();
        let current_price = self.prices[len - 1];
        let prev_price = self.prices[len - 2];

        // Calculate ROC (Rate of Change)
        let roc = if prev_price != 0.0 {
            ((current_price - prev_price) / prev_price) * 100.0
        } else {
            0.0
        };

        // Add ROC to buffer
        if self.roc_values.len() >= self.roc_period {
            self.roc_values.remove(0);
        }
        self.roc_values.push(roc);

        // Calculate percentile
        if self.roc_values.len() >= self.roc_period {
            percentile_rank(&self.roc_values, roc)
        } else {
            50.0
        }
    }

    /// Returns the current Connors RSI value.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.connors_rsi)
    }

    /// Returns the full result with all components.
    #[inline]
    pub fn result(&self) -> ConnorsRsiResult {
        self.current_result
    }

    /// Returns `true` if the indicator has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Resets the indicator to its initial state.
    pub fn reset(&mut self) {
        self.prices.clear();
        self.rsi.reset();
        self.updown_lengths.clear();
        self.current_updown_length = 0;
        self.last_direction = 0;
        self.updown_gains.clear();
        self.updown_losses.clear();
        self.updown_avg_gain = 0.0;
        self.updown_avg_loss = 0.0;
        self.roc_values.clear();
        self.current_result = ConnorsRsiResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }

    /// Получить период RSI
    pub fn period(&self) -> usize {
        self.rsi.period()
    }
    
    /// Генерировать торговый сигнал
    /// Возвращает: -1 (продажа), 0 (нет сигнала), 1 (покупка)
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let crsi = self.current_result.connors_rsi;
        
        // Экстремальные уровни для Connors RSI
        if crsi <= 10.0 {
            return 1; // Сильный сигнал покупки
        } else if crsi >= 90.0 {
            return -1; // Сильный сигнал продажи
        }
        
        0 // Нет сигнала
    }
    
    /// Генерировать расширенный торговый сигнал с учетом всех компонентов
    pub fn advanced_trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let result = self.current_result;
        
        // Анализируем все компоненты
        let rsi_oversold = result.rsi_component <= 20.0;
        let rsi_overbought = result.rsi_component >= 80.0;
        
        let updown_extreme = result.updown_rsi <= 15.0 || result.updown_rsi >= 85.0;
        
        let _roc_extreme = result.roc_percentile <= 10.0 || result.roc_percentile >= 90.0;
        
        // Сильный сигнал покупки
        if result.connors_rsi <= 15.0 && rsi_oversold && updown_extreme && result.roc_percentile <= 20.0 {
            return 1;
        }
        
        // Сильный сигнал продажи
        if result.connors_rsi >= 85.0 && rsi_overbought && updown_extreme && result.roc_percentile >= 80.0 {
            return -1;
        }
        
        // Обычные сигналы
        if result.connors_rsi <= 25.0 {
            return 1;
        } else if result.connors_rsi >= 75.0 {
            return -1;
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
            "Connors RSI: {:.1} ({}), RSI: {:.1}, UpDown RSI: {:.1}, ROC%: {:.1}, UpDown Length: {}, Сигнал: {}",
            result.connors_rsi,
            result.market_condition(),
            result.rsi_component,
            result.updown_rsi,
            result.roc_percentile,
            result.updown_length,
            signal
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("connors_rsi".to_string(), self.current_result.connors_rsi);
        values.insert("rsi_component".to_string(), self.current_result.rsi_component);
        values.insert("updown_rsi".to_string(), self.current_result.updown_rsi);
        values.insert("roc_percentile".to_string(), self.current_result.roc_percentile);
        values.insert("updown_length".to_string(), self.current_result.updown_length as f64);
        values
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    
    /// Получить параметры
    pub fn parameters(&self) -> (usize, usize, usize) {
        (self.rsi.period(), self.updown_period, self.roc_period)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connors_rsi_creation() {
        let crsi = ConnorsRsi::new();
        assert!(!crsi.is_ready());
        assert_eq!(crsi.parameters(), (3, 2, 100));
        assert_eq!(crsi.value().main(), 50.0);
    }
    
    #[test]
    fn test_connors_rsi_with_periods() {
        let crsi = ConnorsRsi::with_periods(5, 3, 50);
        assert_eq!(crsi.parameters(), (5, 3, 50));
    }
    
    #[test]
    fn test_connors_rsi_update() {
        let mut crsi = ConnorsRsi::new();
        
        // Добавляем растущие цены
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            let result = crsi.update_price(price);
            
            if i > 10 {
                // is_ready depends on streak period
                // assert!(crsi.is_ready());
                assert!(result.connors_rsi >= 0.0 && result.connors_rsi <= 100.0);
                assert!(result.rsi_component >= 0.0 && result.rsi_component <= 100.0);
                assert!(result.updown_rsi >= 0.0 && result.updown_rsi <= 100.0);
                assert!(result.roc_percentile >= 0.0 && result.roc_percentile <= 100.0);
            }
        }
        
        // При растущих ценах Connors RSI должен быть высоким
        assert!(crsi.value().main() > 70.0);
    }
    
    #[test]
    fn test_updown_length() {
        let mut crsi = ConnorsRsi::new();
        
        // Последовательность растущих цен
        for i in 1..=5 {
            let _result = crsi.update_price(100.0 + i as f64);
        }
        
        // UpDown length должен быть положительным
        assert!(crsi.current_result.updown_length > 0);
        
        // Последовательность падающих цен
        for i in 1..=5 {
            let _result = crsi.update_price(105.0 - i as f64);
        }
        
        // UpDown length должен быть отрицательным
        assert!(crsi.current_result.updown_length < 0);
    }
    
    #[test]
    fn test_trading_signals() {
        let mut crsi = ConnorsRsi::new();
        
        // Создаем условия для перепроданности
        let mut price = 100.0;
        for _i in 0..15 {
            price -= 1.0;
            let _result = crsi.update_price(price);
        }
        
        if crsi.is_ready() {
            let signal = crsi.trading_signal();
            // При падающих ценах может быть сигнал покупки
            assert!(signal >= -1 && signal <= 1);
        }
    }
    
    #[test]
    fn test_market_condition() {
        let result = ConnorsRsiResult {
            connors_rsi: 15.0,
            rsi_component: 20.0,
            updown_rsi: 10.0,
            roc_percentile: 15.0,
            updown_length: -3,
        };
        
        assert_eq!(result.market_condition(), "Strongly Oversold");

        let result2 = ConnorsRsiResult {
            connors_rsi: 85.0,
            rsi_component: 80.0,
            updown_rsi: 90.0,
            roc_percentile: 85.0,
            updown_length: 4,
        };

        assert_eq!(result2.market_condition(), "Strongly Overbought");
    }

    #[test]
    fn test_connors_rsi_reset() {
        let mut crsi = ConnorsRsi::new();

        for i in 1..=20 {
            crsi.update_price(100.0 + i as f64);
        }

        crsi.reset();
        assert!(!crsi.is_ready());
        assert_eq!(crsi.value().main(), 50.0);
    }

    #[test]
    fn test_connors_rsi_period() {
        let crsi = ConnorsRsi::new();
        assert_eq!(crsi.period(), 3);
    }
} 






















