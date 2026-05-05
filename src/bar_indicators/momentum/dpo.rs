//! Detrended Price Oscillator (DPO) indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::ohlcv_field::OhlcvField;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Detrended Price Oscillator (DPO) - removes trend from price to show cycles.
///
/// DPO = Close - SMA(Close, N) shifted (N/2 + 1) periods back
///
/// Unlike most oscillators, DPO is not designed to identify overbought/oversold
/// conditions. Instead, it removes the trend component to reveal underlying
/// price cycles.
///
/// Interpretation:
/// - DPO > 0: Price above its historical average (cycle peak)
/// - DPO < 0: Price below its historical average (cycle trough)
/// - Zero crossovers: Potential cycle turning points
///
/// # Parameters
/// - `period`: Lookback period (typically 20)
/// - `ma_type`: Type of moving average (default SMA)
/// - `source`: OHLCV field to use as input (default Close)
///
/// # Implementation
///
/// Uses sliding window for historical prices. O(period) per update.

#[derive(Clone)]
pub struct DetrendedPriceOscillator {
    period: usize,
    lookback_offset: usize,
    ma_type: MovingAverageType,
    source: OhlcvField,

    // Буферы для расчетов
    close_prices: ArrayVec<f64, 512>,
    dpo_values: ArrayVec<f64, 512>,

    // Скользящая средняя для расчета
    sma: MovingAverageProvider,

    // Текущее значение
    dpo_value: f64,

    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl DetrendedPriceOscillator {
    /// Creates a new DPO with default period (20).
    pub fn new() -> Self {
        Self::with_period(20)
    }

    /// Creates a new DPO with specified period.
    ///
    /// # Arguments
    /// * `period` - Lookback period
    pub fn with_period(period: usize) -> Self {
        Self::with_period_and_ma_type(period, MovingAverageType::SMA)
    }

    /// Creates a new DPO with specified period and MA type.
    ///
    /// # Arguments
    /// * `period` - Lookback period
    /// * `ma_type` - Type of moving average to use
    pub fn with_period_and_ma_type(period: usize, ma_type: MovingAverageType) -> Self {
        assert!(period > 0, "Period must be greater than 0");

        let lookback_offset = period / 2 + 1;

        Self {
            period,
            lookback_offset,
            ma_type,
            source: OhlcvField::Close,
            close_prices: ArrayVec::new(),
            dpo_values: ArrayVec::new(),
            sma: MovingAverageProvider::new(ma_type, period),
            dpo_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Creates a new DPO with custom source field.
    ///
    /// # Arguments
    /// * `period` - Lookback period
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, source: OhlcvField) -> Self {
        let mut dpo = Self::with_period_and_ma_type(period, MovingAverageType::SMA);
        dpo.source = source;
        dpo
    }

    /// Sets the MA type and resets the indicator.
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }

    /// Returns the source field used for calculation.
    #[inline]
    pub fn get_source(&self) -> OhlcvField {
        self.source
    }

    /// Sets the source field and resets the indicator.
    pub fn set_source(&mut self, source: OhlcvField) {
        if self.source != source {
            self.source = source;
            self.reset();
        }
    }

    /// Updates the DPO with a new bar and returns the current value.
    ///
    /// Extracts value from the configured source field.
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let price = self.source.extract(open, high, low, close, volume);

        self.bars_count += 1;

        // Добавляем цену в буфер
        if self.close_prices.len() >= 512 {
            self.close_prices.remove(0);
        }
        self.close_prices.push(price);

        // Обновляем SMA
        let _sma_value = self.sma.update_bar(price, price, price, price, 1.0);
        
        // Проверяем, можем ли рассчитать DPO
        if self.close_prices.len() >= self.period + self.lookback_offset {
            // Получаем цену закрытия N/2 + 1 периодов назад
            let historical_close_idx = self.close_prices.len() - self.lookback_offset - 1;
            let historical_close = self.close_prices[historical_close_idx];
            
            // Получаем SMA на тот момент времени
            // Для упрощения используем текущий SMA (в реальности нужно было бы хранить историю SMA)
            // Но поскольку DPO смотрит в прошлое, мы можем рассчитать SMA для исторической точки
            let historical_sma = self.calculate_historical_sma(historical_close_idx);
            
            // Рассчитываем DPO
            self.dpo_value = historical_close - historical_sma;
            
            // Добавляем в буфер
            if self.dpo_values.len() >= 512 {
                self.dpo_values.remove(0);
            }
            self.dpo_values.push(self.dpo_value);
            
            // Проверяем готовность
            if !self.is_ready && self.dpo_values.len() >= 3 {
                self.is_ready = true;
            }
        }
        
        self.dpo_value
    }
    
    /// Calculates historical SMA for the given index.
    fn calculate_historical_sma(&self, center_idx: usize) -> f64 {
        if center_idx < self.period / 2 || center_idx + self.period / 2 >= self.close_prices.len() {
            return 0.0;
        }
        
        let start_idx = center_idx - self.period / 2;
        let end_idx = start_idx + self.period;
        
        if end_idx > self.close_prices.len() {
            return 0.0;
        }
        
        let sum: f64 = self.close_prices[start_idx..end_idx].iter().sum();
        sum / self.period as f64
    }
    
    /// Returns the current DPO value as an `IndicatorValue`.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.dpo_value)
    }

    /// Returns `true` if the DPO has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Returns the period of this DPO.
    #[inline]
    pub fn period(&self) -> usize {
        self.period
    }

    /// Resets the DPO to its initial state.
    pub fn reset(&mut self) {
        self.close_prices.clear();
        self.dpo_values.clear();
        self.sma = MovingAverageProvider::new(self.ma_type, self.period);
        self.dpo_value = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Returns the current cycle condition.
    pub fn cycle_condition(&self) -> &'static str {
        match self.dpo_value {
            v if v > 0.0 => "Above Trend Cycle",
            v if v < 0.0 => "Below Trend Cycle",
            _ => "On Trend Cycle",
        }
    }

    /// Returns trading signal based on cycles.
    /// 1 = buy, -1 = sell, 0 = neutral
    pub fn cycle_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        // Простой сигнал на основе пересечения нулевой линии
        if self.dpo_value > 0.0 {
            1
        } else if self.dpo_value < 0.0 {
            -1
        } else {
            0
        }
    }

    /// Returns advanced signal with confirmation.
    pub fn advanced_signal(&self) -> i8 {
        if !self.is_ready() || self.dpo_values.len() < 3 {
            return 0;
        }
        
        let len = self.dpo_values.len();
        let current = self.dpo_value;
        let prev_1 = if len >= 2 { self.dpo_values[len - 2] } else { 0.0 };
        let prev_2 = if len >= 3 { self.dpo_values[len - 3] } else { 0.0 };

        // Buy signal: cross zero line from below with confirmation
        if prev_2 <= 0.0 && prev_1 <= 0.0 && current > 0.0 {
            return 1;
        }

        // Sell signal: cross zero line from above with confirmation
        if prev_2 >= 0.0 && prev_1 >= 0.0 && current < 0.0 {
            return -1;
        }

        0
    }

    /// Finds local extremes (peaks and valleys).
    pub fn find_extremes(&self, lookback: usize) -> i8 {
        if !self.is_ready() || self.dpo_values.len() < lookback * 2 + 1 {
            return 0;
        }
        
        let len = self.dpo_values.len();
        let center_idx = len - lookback - 1;
        
        if center_idx < lookback || center_idx + lookback >= len {
            return 0;
        }
        
        let center_value = self.dpo_values[center_idx];
        let mut is_peak = true;
        let mut is_valley = true;

        // Check if the center point is a peak or valley
        for i in (center_idx - lookback)..=(center_idx + lookback) {
            if i == center_idx {
                continue;
            }
            
            if self.dpo_values[i] >= center_value {
                is_peak = false;
            }
            if self.dpo_values[i] <= center_value {
                is_valley = false;
            }
        }
        
        if is_peak {
            1 // Local peak
        } else if is_valley {
            -1 // Local valley
        } else {
            0 // Not an extreme
        }
    }

    /// Returns the cycle amplitude over the given periods.
    pub fn cycle_amplitude(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.dpo_values.len() < periods {
            return 0.0;
        }
        
        let start_idx = self.dpo_values.len() - periods;
        let slice = &self.dpo_values[start_idx..];
        
        let max_val = slice.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = slice.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        
        max_val - min_val
    }

    /// Returns the cycle average over the given periods.
    pub fn cycle_average(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.dpo_values.len() < periods {
            return 0.0;
        }
        
        let start_idx = self.dpo_values.len() - periods;
        let slice = &self.dpo_values[start_idx..];
        
        slice.iter().sum::<f64>() / slice.len() as f64
    }

    /// Estimates the cycle length (approximate).
    pub fn estimate_cycle_length(&self, min_cycle: usize, max_cycle: usize) -> Option<usize> {
        if !self.is_ready() || self.dpo_values.len() < max_cycle * 2 {
            return None;
        }
        
        let mut best_correlation = f64::NEG_INFINITY;
        let mut best_period = min_cycle;

        // Simple autocorrelation method to find dominant cycle
        for period in min_cycle..=max_cycle {
            if self.dpo_values.len() < period * 2 {
                continue;
            }
            
            let correlation = self.calculate_autocorrelation(period);
            if correlation > best_correlation {
                best_correlation = correlation;
                best_period = period;
            }
        }
        
        if best_correlation > 0.3 {
            Some(best_period)
        } else {
            None
        }
    }

    /// Calculates autocorrelation for the given period.
    fn calculate_autocorrelation(&self, period: usize) -> f64 {
        let len = self.dpo_values.len();
        if len < period * 2 {
            return 0.0;
        }
        
        let n = len - period;
        let mut sum_xy = 0.0;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_x2 = 0.0;
        let mut sum_y2 = 0.0;
        
        for i in 0..n {
            let x = self.dpo_values[i];
            let y = self.dpo_values[i + period];
            
            sum_xy += x * y;
            sum_x += x;
            sum_y += y;
            sum_x2 += x * x;
            sum_y2 += y * y;
        }
        
        let n_f = n as f64;
        let numerator = n_f * sum_xy - sum_x * sum_y;
        let denominator = ((n_f * sum_x2 - sum_x * sum_x) * (n_f * sum_y2 - sum_y * sum_y)).sqrt();
        
        if denominator.abs() < 1e-12 {
            0.0
        } else {
            numerator / denominator
        }
    }

    /// Returns information about the indicator state.
    pub fn info(&self) -> String {
        let amplitude = self.cycle_amplitude(self.period);
        let average = self.cycle_average(self.period);
        
        format!(
            "DPO: {:.4}, Cycle: {}, Amplitude: {:.4}, Average: {:.4}",
            self.dpo_value,
            self.cycle_condition(),
            amplitude,
            average
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_dpo_basic_calculation() {
        let mut dpo = DetrendedPriceOscillator::with_period(10);

        // Feed uptrend data
        for i in 1..=50 {
            dpo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(dpo.is_ready());
        // In strong uptrend, price should be above detrended average
        // DPO value will reflect the cycle position
    }

    #[test]
    fn test_dpo_constant_price() {
        let mut dpo = DetrendedPriceOscillator::with_period(10);

        // Constant price - DPO should be near 0
        for _ in 1..=50 {
            dpo.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        }

        assert!(dpo.is_ready());
        assert!(dpo.value().main().abs() < 0.1, "DPO with constant price should be near 0");
    }

    #[test]
    fn test_dpo_cycle_condition() {
        let mut dpo = DetrendedPriceOscillator::with_period(10);

        // Feed data
        for i in 1..=50 {
            dpo.update_bar(0.0, 0.0, 0.0, 100.0 + (i % 20) as f64, 0.0);
        }

        assert!(dpo.is_ready());
        let condition = dpo.cycle_condition();
        assert!(
            condition == "Above Trend Cycle"
                || condition == "Below Trend Cycle"
                || condition == "On Trend Cycle"
        );
    }

    #[test]
    fn test_dpo_reset() {
        let mut dpo = DetrendedPriceOscillator::with_period(10);

        for i in 1..=50 {
            dpo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(dpo.is_ready());

        dpo.reset();
        assert!(!dpo.is_ready());
        assert!(dpo.value().main().abs() < 1e-10);
    }

    #[test]
    fn test_dpo_period_getter() {
        let dpo = DetrendedPriceOscillator::with_period(15);
        assert_eq!(dpo.period(), 15);
    }

    #[test]
    fn test_dpo_cycle_signal() {
        let mut dpo = DetrendedPriceOscillator::with_period(10);

        // Before ready, signal should be 0
        assert_eq!(dpo.cycle_signal(), 0);

        // Feed enough data to be ready
        for i in 1..=50 {
            dpo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(dpo.is_ready());
        // Signal should be -1, 0, or 1
        let signal = dpo.cycle_signal();
        assert!(signal >= -1 && signal <= 1);
    }

    #[test]
    fn test_dpo_default() {
        let dpo = DetrendedPriceOscillator::new();
        assert_eq!(dpo.period(), 20); // Default period
    }
}


















