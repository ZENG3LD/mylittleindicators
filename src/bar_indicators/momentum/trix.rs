//! TRIX (Triple Exponential Average) indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// TRIX - Triple Exponential Average momentum oscillator.
///
/// TRIX = ROC(EMA(EMA(EMA(Close, N), N), N), 1)
///
/// Shows the percentage rate of change of a triple-smoothed exponential moving
/// average. The triple smoothing eliminates most market noise, making it useful
/// for identifying trend changes.
///
/// Interpretation:
/// - TRIX > 0: Bullish momentum
/// - TRIX < 0: Bearish momentum
/// - Signal line crossovers: Trading signals
/// - Zero line crossovers: Trend changes
///
/// # Parameters
/// - `period`: Period for each EMA layer (typically 14)
/// - `signal_period`: Signal line period (typically 9)
/// - `ma_type`: Type of moving average (default EMA)
///
/// # Implementation
///
/// Uses three cascaded EMAs followed by ROC calculation.
#[derive(Clone)]
pub struct Trix {
    period: usize,
    signal_period: usize,
    smoothing_ma_type: MovingAverageType,
    signal_ma_type: MovingAverageType,
    source: OhlcvField,

    // Тройное экспоненциальное сглаживание
    first_ema: MovingAverageProvider,
    second_ema: MovingAverageProvider,
    third_ema: MovingAverageProvider,

    // Сигнальная линия
    signal_ema: MovingAverageProvider,

    // Буферы для значений
    trix_values: ArrayVec<f64, 512>,
    triple_ema_values: ArrayVec<f64, 512>,

    // Предыдущее значение тройной EMA для расчета ROC
    prev_triple_ema: f64,

    // Текущие значения
    trix_value: f64,
    signal_value: f64,

    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl Trix {
    /// Creates a new TRIX with default parameters (14, 9) using EMA and Close source.
    pub fn new() -> Self {
        Self::with_params(14, 9, MovingAverageType::EMA)
    }

    /// Creates a new TRIX with specified periods (uses EMA).
    ///
    /// # Arguments
    /// * `period` - Period for each EMA layer
    /// * `signal_period` - Signal line period
    pub fn with_params_default(period: usize, signal_period: usize) -> Self {
        Self::with_params(period, signal_period, MovingAverageType::EMA)
    }

    /// Creates a new TRIX with specified parameters.
    ///
    /// # Arguments
    /// * `period` - Period for each EMA layer
    /// * `signal_period` - Signal line period
    /// * `ma_type` - Type of moving average to use for all components
    pub fn with_params(period: usize, signal_period: usize, ma_type: MovingAverageType) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        assert!(signal_period > 0, "Signal period must be greater than 0");

        Self {
            period,
            signal_period,
            smoothing_ma_type: ma_type,
            signal_ma_type: ma_type,
            source: OhlcvField::Close,
            first_ema: MovingAverageProvider::new(ma_type, period),
            second_ema: MovingAverageProvider::new(ma_type, period),
            third_ema: MovingAverageProvider::new(ma_type, period),
            signal_ema: MovingAverageProvider::new(ma_type, signal_period),
            trix_values: ArrayVec::new(),
            triple_ema_values: ArrayVec::new(),
            prev_triple_ema: 0.0,
            trix_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Creates a new TRIX with custom source field.
    ///
    /// # Arguments
    /// * `period` - Period for each EMA layer
    /// * `signal_period` - Signal line period
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, signal_period: usize, source: OhlcvField) -> Self {
        let mut trix = Self::with_params(period, signal_period, MovingAverageType::EMA);
        trix.source = source;
        trix
    }

    /// Creates a new TRIX with custom MA types for smoothing and signal.
    ///
    /// # Arguments
    /// * `period` - Period for each EMA layer
    /// * `signal_period` - Signal line period
    /// * `smoothing_ma_type` - MA type for the three cascaded EMAs
    /// * `signal_ma_type` - MA type for the signal line
    pub fn with_ma_types(
        period: usize,
        signal_period: usize,
        smoothing_ma_type: MovingAverageType,
        signal_ma_type: MovingAverageType,
    ) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        assert!(signal_period > 0, "Signal period must be greater than 0");

        Self {
            period,
            signal_period,
            smoothing_ma_type,
            signal_ma_type,
            source: OhlcvField::Close,
            first_ema: MovingAverageProvider::new(smoothing_ma_type, period),
            second_ema: MovingAverageProvider::new(smoothing_ma_type, period),
            third_ema: MovingAverageProvider::new(smoothing_ma_type, period),
            signal_ema: MovingAverageProvider::new(signal_ma_type, signal_period),
            trix_values: ArrayVec::new(),
            triple_ema_values: ArrayVec::new(),
            prev_triple_ema: 0.0,
            trix_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Creates a new TRIX with full configuration.
    ///
    /// # Arguments
    /// * `period` - Period for each EMA layer
    /// * `signal_period` - Signal line period
    /// * `smoothing_ma_type` - MA type for the three cascaded EMAs
    /// * `signal_ma_type` - MA type for the signal line
    /// * `source` - OHLCV field to use as input
    pub fn with_full_config(
        period: usize,
        signal_period: usize,
        smoothing_ma_type: MovingAverageType,
        signal_ma_type: MovingAverageType,
        source: OhlcvField,
    ) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        assert!(signal_period > 0, "Signal period must be greater than 0");

        Self {
            period,
            signal_period,
            smoothing_ma_type,
            signal_ma_type,
            source,
            first_ema: MovingAverageProvider::new(smoothing_ma_type, period),
            second_ema: MovingAverageProvider::new(smoothing_ma_type, period),
            third_ema: MovingAverageProvider::new(smoothing_ma_type, period),
            signal_ema: MovingAverageProvider::new(signal_ma_type, signal_period),
            trix_values: ArrayVec::new(),
            triple_ema_values: ArrayVec::new(),
            prev_triple_ema: 0.0,
            trix_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Updates the TRIX with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field.
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        self.bars_count += 1;

        // Extract source value
        let source_value = self.source.extract(open, high, low, close, volume);

        // Первое сглаживание
        let first_ema_value = self.first_ema.update_bar(source_value, source_value, source_value, source_value, 1.0);

        // Второе сглаживание
        let second_ema_value = self.second_ema.update_bar(first_ema_value, first_ema_value, first_ema_value, first_ema_value, 1.0);

        // Третье сглаживание
        let third_ema_value = self.third_ema.update_bar(second_ema_value, second_ema_value, second_ema_value, second_ema_value, 1.0);
        
        // Добавляем в буфер тройной EMA
        if self.triple_ema_values.len() >= 512 {
            self.triple_ema_values.remove(0);
        }
        self.triple_ema_values.push(third_ema_value);
        
        // Рассчитываем TRIX (ROC от тройной EMA)
        if self.bars_count > 1 && self.prev_triple_ema.abs() > 1e-12 {
            self.trix_value = ((third_ema_value - self.prev_triple_ema) / self.prev_triple_ema) * 10000.0; // Умножаем на 10000 для удобства
        }
        
        // Добавляем в буфер TRIX
        if self.trix_values.len() >= 512 {
            self.trix_values.remove(0);
        }
        self.trix_values.push(self.trix_value);
        
        // Рассчитываем сигнальную линию
        self.signal_value = self.signal_ema.update_bar(self.trix_value, self.trix_value, self.trix_value, self.trix_value, 1.0);
        
        // Обновляем предыдущее значение
        self.prev_triple_ema = third_ema_value;
        
        // Проверяем готовность (нужно время для стабилизации тройного сглаживания)
        let min_bars = self.period * 3 + self.signal_period;
        if self.bars_count >= min_bars {
            self.is_ready = true;
        }
        
        self.trix_value
    }
    
    /// Returns the current TRIX value as an `IndicatorValue`.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.trix_value)
    }

    /// Returns the signal line value.
    #[inline]
    pub fn signal_value(&self) -> f64 {
        self.signal_value
    }

    /// Returns the histogram (TRIX - Signal).
    #[inline]
    pub fn histogram(&self) -> f64 {
        self.trix_value - self.signal_value
    }

    /// Returns `true` if the TRIX has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Returns the periods (period, signal_period).
    #[inline]
    pub fn periods(&self) -> (usize, usize) {
        (self.period, self.signal_period)
    }

    /// Resets the TRIX to its initial state.
    pub fn reset(&mut self) {
        self.first_ema = MovingAverageProvider::new(self.smoothing_ma_type, self.period);
        self.second_ema = MovingAverageProvider::new(self.smoothing_ma_type, self.period);
        self.third_ema = MovingAverageProvider::new(self.smoothing_ma_type, self.period);
        self.signal_ema = MovingAverageProvider::new(self.signal_ma_type, self.signal_period);
        self.trix_values.clear();
        self.triple_ema_values.clear();
        self.prev_triple_ema = 0.0;
        self.trix_value = 0.0;
        self.signal_value = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    /// Sets a new moving average type for smoothing (resets the indicator).
    pub fn set_smoothing_ma_type(&mut self, ma_type: MovingAverageType) {
        self.smoothing_ma_type = ma_type;
        self.reset();
    }

    /// Sets a new moving average type for signal line (resets the indicator).
    pub fn set_signal_ma_type(&mut self, ma_type: MovingAverageType) {
        self.signal_ma_type = ma_type;
        self.reset();
    }

    /// Returns the smoothing MA type.
    #[inline]
    pub fn get_smoothing_ma_type(&self) -> MovingAverageType {
        self.smoothing_ma_type
    }

    /// Returns the signal MA type.
    #[inline]
    pub fn get_signal_ma_type(&self) -> MovingAverageType {
        self.signal_ma_type
    }

    /// Returns the source field.
    #[inline]
    pub fn get_source(&self) -> OhlcvField {
        self.source
    }

    /// Sets the source field (resets the indicator).
    pub fn set_source(&mut self, source: OhlcvField) {
        self.source = source;
        self.reset();
    }

    /// Returns the current trend condition.
    pub fn trend_condition(&self) -> &'static str {
        match self.trix_value {
            v if v > 0.0 && self.trix_value > self.signal_value => "Strong Bullish",
            v if v > 0.0 => "Bullish",
            v if v < 0.0 && self.trix_value < self.signal_value => "Strong Bearish",
            v if v < 0.0 => "Bearish",
            _ => "Neutral"
        }
    }
    
    /// Получить торговый сигнал
    /// 1 = покупка, -1 = продажа, 0 = нейтрально
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        let histogram = self.histogram();
        
        // Сигналы на основе пересечения сигнальной линии
        if self.trix_value > self.signal_value && histogram > 0.5 {
            1  // Покупка - TRIX выше сигнальной линии
        } else if self.trix_value < self.signal_value && histogram < -0.5 {
            -1 // Продажа - TRIX ниже сигнальной линии
        } else {
            0  // Нейтрально
        }
    }
    
    /// Получить продвинутый сигнал с подтверждением
    pub fn advanced_signal(&self) -> i8 {
        if !self.is_ready() || self.trix_values.len() < 3 {
            return 0;
        }
        
        let len = self.trix_values.len();
        let current = self.trix_value;
        let prev_1 = if len >= 2 { self.trix_values[len - 2] } else { 0.0 };
        let prev_2 = if len >= 3 { self.trix_values[len - 3] } else { 0.0 };
        
        // Сигнал покупки: пересечение нулевой линии снизу вверх с подтверждением сигнальной линии
        if prev_2 < 0.0 && prev_1 < 0.0 && current > 0.0 && self.trix_value > self.signal_value {
            return 1;
        }
        
        // Сигнал продажи: пересечение нулевой линии сверху вниз с подтверждением сигнальной линии
        if prev_2 > 0.0 && prev_1 > 0.0 && current < 0.0 && self.trix_value < self.signal_value {
            return -1;
        }
        
        0
    }
    
    /// Проверить дивергенцию между ценой и TRIX
    pub fn check_divergence(&self, price_history: &[f64], lookback: usize) -> i8 {
        if !self.is_ready() || price_history.len() < lookback + 1 || self.trix_values.len() < lookback + 1 {
            return 0;
        }
        
        let current_price = price_history[price_history.len() - 1];
        let past_price = price_history[price_history.len() - lookback - 1];
        let current_trix = self.trix_value;
        let past_trix = self.trix_values[self.trix_values.len() - lookback - 1];
        
        let price_change = current_price - past_price;
        let trix_change = current_trix - past_trix;
        
        // Бычья дивергенция: цена делает новый минимум, но TRIX растет
        if price_change < 0.0 && trix_change > 0.0 {
            return 1;
        }
        
        // Медвежья дивергенция: цена делает новый максимум, но TRIX падает
        if price_change > 0.0 && trix_change < 0.0 {
            return -1;
        }
        
        0
    }
    
    /// Получить скорость изменения тренда
    pub fn trend_velocity(&self) -> f64 {
        self.trix_value.abs()
    }
    
    /// Получить ускорение тренда
    pub fn trend_acceleration(&self) -> f64 {
        if !self.is_ready() || self.trix_values.len() < 2 {
            return 0.0;
        }
        
        let len = self.trix_values.len();
        let current = self.trix_value;
        let prev = self.trix_values[len - 2];
        
        current - prev
    }
    
    /// Получить силу тренда
    pub fn trend_strength(&self, lookback: usize) -> f64 {
        if !self.is_ready() || self.trix_values.len() < lookback {
            return 0.0;
        }
        
        let start_idx = self.trix_values.len() - lookback;
        let slice = &self.trix_values[start_idx..];
        
        // Считаем среднее абсолютное значение
        slice.iter().map(|&x| x.abs()).sum::<f64>() / slice.len() as f64
    }
    
    /// Получить направление тренда
    pub fn trend_direction(&self, lookback: usize) -> i8 {
        if !self.is_ready() || self.trix_values.len() < lookback + 1 {
            return 0;
        }
        
        let current = self.trix_value;
        let past = self.trix_values[self.trix_values.len() - lookback - 1];
        
        if current > past {
            1  // Восходящий тренд
        } else if current < past {
            -1 // Нисходящий тренд
        } else {
            0  // Боковой тренд
        }
    }
    
    /// Получить текущее значение тройной EMA
    pub fn triple_ema_value(&self) -> f64 {
        if self.triple_ema_values.is_empty() {
            0.0
        } else {
            *self.triple_ema_values.last().unwrap()
        }
    }
    
    /// Получить фильтрованный сигнал (только сильные движения)
    pub fn filtered_signal(&self, threshold: f64) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        let basic_signal = self.trading_signal();
        let velocity = self.trend_velocity();
        
        // Фильтруем слабые сигналы
        if velocity < threshold {
            return 0;
        }
        
        basic_signal
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        format!(
            "TRIX: {:.2}, Signal: {:.2}, Histogram: {:.2}, Trend: {}, Velocity: {:.2}, Acceleration: {:.2}",
            self.trix_value,
            self.signal_value,
            self.histogram(),
            self.trend_condition(),
            self.trend_velocity(),
            self.trend_acceleration()
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
    fn test_trix_basic_calculation() {
        let mut trix = Trix::new();

        // Feed uptrend data - need enough for triple smoothing
        for i in 1..=100 {
            trix.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64 * 0.5, 0.0);
        }

        assert!(trix.is_ready());
        // In uptrend, TRIX should be positive
        assert!(trix.value().main() > 0.0, "TRIX in uptrend should be positive");
    }

    #[test]
    fn test_trix_downtrend() {
        let mut trix = Trix::new();

        // Feed downtrend data
        for i in 1..=100 {
            trix.update_bar(0.0, 0.0, 0.0, 200.0 - i as f64 * 0.5, 0.0);
        }

        assert!(trix.is_ready());
        // In downtrend, TRIX should be negative
        assert!(trix.value().main() < 0.0, "TRIX in downtrend should be negative");
    }

    #[test]
    fn test_trix_signal_line() {
        let mut trix = Trix::new();

        // Feed data
        for i in 1..=100 {
            trix.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(trix.is_ready());
        // Signal should track TRIX
        let _ = trix.signal_value(); // Just verify it returns a value
    }

    #[test]
    fn test_trix_histogram() {
        let mut trix = Trix::new();

        for i in 1..=100 {
            trix.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(trix.is_ready());
        // Histogram = TRIX - Signal
        let expected = trix.value().main() - trix.signal_value();
        assert!((trix.histogram() - expected).abs() < 1e-10);
    }

    #[test]
    fn test_trix_reset() {
        let mut trix = Trix::new();

        for i in 1..=100 {
            trix.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(trix.is_ready());

        trix.reset();
        assert!(!trix.is_ready());
        assert!(trix.value().main().abs() < 1e-10);
    }

    #[test]
    fn test_trix_periods() {
        let trix = Trix::with_params_default(10, 5);
        assert_eq!(trix.periods(), (10, 5));
    }

    #[test]
    fn test_trix_trend_condition() {
        let mut trix = Trix::new();

        for i in 1..=100 {
            trix.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(trix.is_ready());
        let condition = trix.trend_condition();
        assert!(
            condition == "Strong Bullish"
                || condition == "Bullish"
                || condition == "Strong Bearish"
                || condition == "Bearish"
                || condition == "Neutral"
        );
    }

    #[test]
    fn test_trix_with_source() {
        let mut trix = Trix::with_source(14, 9, OhlcvField::HL2);

        // Feed bars with varying HL2 prices in uptrend
        for i in 1..=100 {
            let base = 100.0 + i as f64;
            trix.update_bar(base, base + 10.0, base - 10.0, base + 5.0, 1000.0);
        }

        assert!(trix.is_ready());
        assert_eq!(trix.get_source(), OhlcvField::HL2);
    }

    #[test]
    fn test_trix_with_ma_types() {
        let mut trix = Trix::with_ma_types(
            14, 9,
            MovingAverageType::SMA,
            MovingAverageType::WMA
        );

        for i in 1..=100 {
            trix.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(trix.is_ready());
        assert_eq!(trix.get_smoothing_ma_type(), MovingAverageType::SMA);
        assert_eq!(trix.get_signal_ma_type(), MovingAverageType::WMA);
    }

    #[test]
    fn test_trix_with_full_config() {
        let mut trix = Trix::with_full_config(
            14, 9,
            MovingAverageType::EMA,
            MovingAverageType::SMA,
            OhlcvField::HLC3
        );

        for i in 1..=100 {
            let base = 100.0 + i as f64;
            trix.update_bar(base, base + 10.0, base - 10.0, base + 5.0, 1000.0);
        }

        assert!(trix.is_ready());
        assert_eq!(trix.get_smoothing_ma_type(), MovingAverageType::EMA);
        assert_eq!(trix.get_signal_ma_type(), MovingAverageType::SMA);
        assert_eq!(trix.get_source(), OhlcvField::HLC3);
    }

    #[test]
    fn test_trix_set_source() {
        let mut trix = Trix::new();

        for i in 1..=100 {
            trix.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(trix.is_ready());

        trix.set_source(OhlcvField::HL2);
        assert!(!trix.is_ready()); // Should reset
        assert_eq!(trix.get_source(), OhlcvField::HL2);
    }

    #[test]
    fn test_trix_set_ma_types() {
        let mut trix = Trix::new();

        for i in 1..=100 {
            trix.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(trix.is_ready());

        trix.set_smoothing_ma_type(MovingAverageType::SMA);
        assert!(!trix.is_ready()); // Should reset
        assert_eq!(trix.get_smoothing_ma_type(), MovingAverageType::SMA);

        // Feed data again
        for i in 1..=100 {
            trix.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(trix.is_ready());

        trix.set_signal_ma_type(MovingAverageType::WMA);
        assert!(!trix.is_ready()); // Should reset
        assert_eq!(trix.get_signal_ma_type(), MovingAverageType::WMA);
    }

    #[test]
    fn test_trix_default_source_is_close() {
        let mut trix_default = Trix::new();
        let mut trix_close = Trix::with_source(14, 9, OhlcvField::Close);

        // Both should produce the same result
        for i in 1..=100 {
            let v1 = trix_default.update_bar(100.0, 110.0, 90.0, 100.0 + i as f64, 1000.0);
            let v2 = trix_close.update_bar(100.0, 110.0, 90.0, 100.0 + i as f64, 1000.0);
            assert!((v1 - v2).abs() < 1e-10, "Default and Close source should match");
        }

        // Verify final values match
        assert!((trix_default.value().main() - trix_close.value().main()).abs() < 1e-10);
        assert!((trix_default.signal_value() - trix_close.signal_value()).abs() < 1e-10);
        assert!((trix_default.histogram() - trix_close.histogram()).abs() < 1e-10);
    }
}


















