//! True Strength Index (TSI) indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// True Strength Index (TSI) - double-smoothed momentum indicator by William Blau.
///
/// TSI = 100 × (Smoothed(Smoothed(PC)) / Smoothed(Smoothed(|PC|)))
///
/// where PC = Price Change = Close - Close_prev
///
/// Double exponential smoothing reduces noise while maintaining responsiveness.
///
/// Oscillates between -100 and +100:
/// - Above +25: Strong bullish momentum
/// - Below -25: Strong bearish momentum
/// - Zero line crossings: Trend changes
///
/// # Parameters
/// - `first_smoothing`: Long-term smoothing (typically 25)
/// - `second_smoothing`: Short-term smoothing (typically 13)
/// - `signal_period`: Signal line smoothing (typically 13)
///
/// # Implementation
///
/// Uses four EMA instances for double smoothing. O(1) update complexity.
#[derive(Clone)]
pub struct TrueStrengthIndex {
    first_smoothing: usize,
    second_smoothing: usize,
    ma_type: MovingAverageType,          // Legacy field for backward compatibility
    smoothing_ma_type: MovingAverageType, // MA type for the 4 main smoothing EMAs
    signal_ma_type: MovingAverageType,    // MA type for signal line
    source: OhlcvField,                   // Price source for TSI calculation

    // Буферы для расчетов
    tsi_values: ArrayVec<f64, 512>,

    // Предыдущая цена
    prev_price: f64,

    // Двойное сглаживание для Price Change
    first_pc_ema: MovingAverageProvider,
    second_pc_ema: MovingAverageProvider,

    // Двойное сглаживание для Absolute Price Change
    first_apc_ema: MovingAverageProvider,
    second_apc_ema: MovingAverageProvider,

    // Сигнальная линия (EMA от TSI)
    signal_line_period: usize,
    signal_ema: MovingAverageProvider,

    // Текущие значения
    tsi_value: f64,
    signal_value: f64,

    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl TrueStrengthIndex {
    /// Создать новый TSI с параметрами по умолчанию (25, 13, 13)
    pub fn new() -> Self {
        Self::with_params(25, 13, 13, MovingAverageType::EMA)
    }

    /// Создать новый TSI с настраиваемыми параметрами (backward compatibility)
    pub fn with_params_default(first_smoothing: usize, second_smoothing: usize, signal_period: usize) -> Self {
        Self::with_params(first_smoothing, second_smoothing, signal_period, MovingAverageType::EMA)
    }

    /// Создать новый TSI с настраиваемыми параметрами
    pub fn with_params(first_smoothing: usize, second_smoothing: usize, signal_period: usize, ma_type: MovingAverageType) -> Self {
        assert!(first_smoothing > 0, "First smoothing period must be greater than 0");
        assert!(second_smoothing > 0, "Second smoothing period must be greater than 0");
        assert!(signal_period > 0, "Signal period must be greater than 0");

        Self {
            first_smoothing,
            second_smoothing,
            ma_type,
            smoothing_ma_type: ma_type,
            signal_ma_type: ma_type,
            source: OhlcvField::Close,
            tsi_values: ArrayVec::new(),
            prev_price: 0.0,
            first_pc_ema: MovingAverageProvider::new(ma_type, first_smoothing),
            second_pc_ema: MovingAverageProvider::new(ma_type, second_smoothing),
            first_apc_ema: MovingAverageProvider::new(ma_type, first_smoothing),
            second_apc_ema: MovingAverageProvider::new(ma_type, second_smoothing),
            signal_line_period: signal_period,
            signal_ema: MovingAverageProvider::new(ma_type, signal_period),
            tsi_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Creates a new TSI with custom source field.
    ///
    /// # Arguments
    /// * `source` - OHLCV field to use as input for price change calculation
    pub fn with_source(source: OhlcvField) -> Self {
        let mut tsi = Self::new();
        tsi.source = source;
        tsi
    }

    /// Creates a new TSI with independent MA types for smoothing and signal.
    ///
    /// # Arguments
    /// * `smoothing_ma_type` - MA type for the 4 main smoothing components (first/second PC and APC EMAs)
    /// * `signal_ma_type` - MA type for the signal line
    pub fn with_ma_types(smoothing_ma_type: MovingAverageType, signal_ma_type: MovingAverageType) -> Self {
        Self::with_full_config(25, 13, 13, smoothing_ma_type, signal_ma_type, OhlcvField::Close)
    }

    /// Creates a new TSI with full configuration.
    ///
    /// # Arguments
    /// * `first_smoothing` - First smoothing period (typically 25)
    /// * `second_smoothing` - Second smoothing period (typically 13)
    /// * `signal_period` - Signal line period (typically 13)
    /// * `smoothing_ma_type` - MA type for the 4 main smoothing components
    /// * `signal_ma_type` - MA type for the signal line
    /// * `source` - OHLCV field to use as input
    pub fn with_full_config(
        first_smoothing: usize,
        second_smoothing: usize,
        signal_period: usize,
        smoothing_ma_type: MovingAverageType,
        signal_ma_type: MovingAverageType,
        source: OhlcvField,
    ) -> Self {
        assert!(first_smoothing > 0, "First smoothing period must be greater than 0");
        assert!(second_smoothing > 0, "Second smoothing period must be greater than 0");
        assert!(signal_period > 0, "Signal period must be greater than 0");

        Self {
            first_smoothing,
            second_smoothing,
            ma_type: smoothing_ma_type,  // For backward compatibility
            smoothing_ma_type,
            signal_ma_type,
            source,
            tsi_values: ArrayVec::new(),
            prev_price: 0.0,
            first_pc_ema: MovingAverageProvider::new(smoothing_ma_type, first_smoothing),
            second_pc_ema: MovingAverageProvider::new(smoothing_ma_type, second_smoothing),
            first_apc_ema: MovingAverageProvider::new(smoothing_ma_type, first_smoothing),
            second_apc_ema: MovingAverageProvider::new(smoothing_ma_type, second_smoothing),
            signal_line_period: signal_period,
            signal_ema: MovingAverageProvider::new(signal_ma_type, signal_period),
            tsi_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        self.bars_count += 1;

        // Extract price from configured source
        let current_price = self.source.extract(open, high, low, close, volume);

        if self.bars_count == 1 {
            // Первый бар - инициализируем предыдущую цену
            self.prev_price = current_price;
            return self.tsi_value;
        }

        // Рассчитываем Price Change и Absolute Price Change
        let price_change = current_price - self.prev_price;
        let abs_price_change = price_change.abs();

        // Первое сглаживание
        let first_pc_smooth = self.first_pc_ema.update_bar(price_change, price_change, price_change, price_change, 1.0);
        let first_apc_smooth = self.first_apc_ema.update_bar(abs_price_change, abs_price_change, abs_price_change, abs_price_change, 1.0);

        // Второе сглаживание
        let second_pc_smooth = self.second_pc_ema.update_bar(first_pc_smooth, first_pc_smooth, first_pc_smooth, first_pc_smooth, 1.0);
        let second_apc_smooth = self.second_apc_ema.update_bar(first_apc_smooth, first_apc_smooth, first_apc_smooth, first_apc_smooth, 1.0);

        // Рассчитываем TSI
        self.tsi_value = if second_apc_smooth.abs() < 1e-12 {
            0.0
        } else {
            100.0 * (second_pc_smooth / second_apc_smooth)
        };

        // Добавляем в буфер
        if self.tsi_values.len() >= 512 {
            self.tsi_values.remove(0);
        }
        self.tsi_values.push(self.tsi_value);

        // Рассчитываем сигнальную линию
        self.signal_value = self.signal_ema.update_bar(self.tsi_value, self.tsi_value, self.tsi_value, self.tsi_value, 1.0);

        // Обновляем предыдущую цену
        self.prev_price = current_price;

        // Проверяем готовность (нужно время для стабилизации двойного сглаживания)
        let min_bars = self.first_smoothing + self.second_smoothing + self.signal_line_period;
        if self.bars_count >= min_bars {
            self.is_ready = true;
        }

        self.tsi_value
    }
    
    /// Получить значения TSI и Signal
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.tsi_value, self.signal_value)
    }
    
    /// Получить значение сигнальной линии
    pub fn signal_value(&self) -> f64 {
        self.signal_value
    }
    
    /// Получить гистограмму (TSI - Signal)
    pub fn histogram(&self) -> f64 {
        self.tsi_value - self.signal_value
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить периоды сглаживания
    pub fn periods(&self) -> (usize, usize, usize) {
        (self.first_smoothing, self.second_smoothing, self.signal_line_period)
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.tsi_values.clear();
        self.prev_price = 0.0;
        self.first_pc_ema = MovingAverageProvider::new(self.smoothing_ma_type, self.first_smoothing);
        self.second_pc_ema = MovingAverageProvider::new(self.smoothing_ma_type, self.second_smoothing);
        self.first_apc_ema = MovingAverageProvider::new(self.smoothing_ma_type, self.first_smoothing);
        self.second_apc_ema = MovingAverageProvider::new(self.smoothing_ma_type, self.second_smoothing);
        self.signal_ema = MovingAverageProvider::new(self.signal_ma_type, self.signal_line_period);
        self.tsi_value = 0.0;
        self.signal_value = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    /// Установить новый тип скользящей средней (backward compatibility - sets both types)
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.smoothing_ma_type = ma_type;
        self.signal_ma_type = ma_type;
        self.reset();
    }

    /// Set smoothing MA type for the 4 main smoothing components
    pub fn set_smoothing_ma_type(&mut self, ma_type: MovingAverageType) {
        self.smoothing_ma_type = ma_type;
        self.ma_type = ma_type;  // Update legacy field
        self.reset();
    }

    /// Set signal MA type for the signal line
    pub fn set_signal_ma_type(&mut self, ma_type: MovingAverageType) {
        self.signal_ma_type = ma_type;
        self.reset();
    }

    /// Set source field for price input
    pub fn set_source(&mut self, source: OhlcvField) {
        self.source = source;
        self.reset();
    }

    /// Get current source field
    pub fn source(&self) -> OhlcvField {
        self.source
    }

    /// Get current smoothing MA type
    pub fn smoothing_ma_type(&self) -> MovingAverageType {
        self.smoothing_ma_type
    }

    /// Get current signal MA type
    pub fn signal_ma_type(&self) -> MovingAverageType {
        self.signal_ma_type
    }
    
    /// Определить состояние рынка
    pub fn market_condition(&self) -> &'static str {
        match self.tsi_value {
            v if v > 25.0 => "Strong Bullish",
            v if v > 0.0 => "Bullish",
            v if v < -25.0 => "Strong Bearish",
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
        if self.tsi_value > self.signal_value && histogram > 0.5 {
            1  // Покупка - TSI выше сигнальной линии
        } else if self.tsi_value < self.signal_value && histogram < -0.5 {
            -1 // Продажа - TSI ниже сигнальной линии
        } else {
            0  // Нейтрально
        }
    }
    
    /// Получить продвинутый сигнал с подтверждением
    pub fn advanced_signal(&self) -> i8 {
        if !self.is_ready() || self.tsi_values.len() < 3 {
            return 0;
        }
        
        let len = self.tsi_values.len();
        let current = self.tsi_value;
        let prev_1 = if len >= 2 { self.tsi_values[len - 2] } else { 0.0 };
        let prev_2 = if len >= 3 { self.tsi_values[len - 3] } else { 0.0 };
        
        // Сигнал покупки: пересечение нулевой линии снизу вверх с подтверждением
        if prev_2 < 0.0 && prev_1 < 0.0 && current > 0.0 && self.tsi_value > self.signal_value {
            return 1;
        }
        
        // Сигнал продажи: пересечение нулевой линии сверху вниз с подтверждением
        if prev_2 > 0.0 && prev_1 > 0.0 && current < 0.0 && self.tsi_value < self.signal_value {
            return -1;
        }
        
        0
    }
    
    /// Проверить дивергенцию между ценой и TSI
    pub fn check_divergence(&self, price_history: &[f64], lookback: usize) -> i8 {
        if !self.is_ready() || price_history.len() < lookback + 1 || self.tsi_values.len() < lookback + 1 {
            return 0;
        }
        
        let current_price = price_history[price_history.len() - 1];
        let past_price = price_history[price_history.len() - lookback - 1];
        let current_tsi = self.tsi_value;
        let past_tsi = self.tsi_values[self.tsi_values.len() - lookback - 1];
        
        let price_change = current_price - past_price;
        let tsi_change = current_tsi - past_tsi;
        
        // Бычья дивергенция: цена делает новый минимум, но TSI растет
        if price_change < 0.0 && tsi_change > 0.0 {
            return 1;
        }
        
        // Медвежья дивергенция: цена делает новый максимум, но TSI падает
        if price_change > 0.0 && tsi_change < 0.0 {
            return -1;
        }
        
        0
    }
    
    /// Получить силу momentum (абсолютное значение TSI)
    pub fn momentum_strength(&self) -> f64 {
        self.tsi_value.abs()
    }
    
    /// Получить скорость изменения TSI
    pub fn rate_of_change(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.tsi_values.len() < periods + 1 {
            return 0.0;
        }
        
        let current = self.tsi_value;
        let past = self.tsi_values[self.tsi_values.len() - periods - 1];
        
        if past.abs() < 1e-12 {
            0.0
        } else {
            (current - past) / past * 100.0
        }
    }
    
    /// Получить уровни перекупленности/перепроданности
    pub fn overbought_oversold_levels(&self) -> (&'static str, f64, f64) {
        // Стандартные уровни для TSI
        let overbought = 25.0;
        let oversold = -25.0;
        
        let condition = match self.tsi_value {
            v if v >= overbought => "Overbought",
            v if v <= oversold => "Oversold",
            _ => "Normal"
        };
        
        (condition, overbought, oversold)
    }
    
    /// Получить тренд TSI
    pub fn trend_direction(&self, lookback: usize) -> i8 {
        if !self.is_ready() || self.tsi_values.len() < lookback + 1 {
            return 0;
        }
        
        let current = self.tsi_value;
        let past = self.tsi_values[self.tsi_values.len() - lookback - 1];
        
        if current > past {
            1  // Восходящий тренд
        } else if current < past {
            -1 // Нисходящий тренд
        } else {
            0  // Боковой тренд
        }
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        let (condition, ob_level, os_level) = self.overbought_oversold_levels();
        format!(
            "TSI: {:.2}, Signal: {:.2}, Histogram: {:.2}, Condition: {} (OB: {:.1}, OS: {:.1}), Strength: {:.2}",
            self.tsi_value,
            self.signal_value,
            self.histogram(),
            condition,
            ob_level,
            os_level,
            self.momentum_strength()
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
    fn test_tsi_basic_calculation() {
        let mut tsi = TrueStrengthIndex::new();

        // Feed uptrend data
        for i in 1..=60 {
            tsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(tsi.is_ready());
        // In uptrend, TSI should be positive
        assert!(tsi.value().main() > 0.0, "TSI in uptrend should be positive, got {}", tsi.value().main());
    }

    #[test]
    fn test_tsi_downtrend() {
        let mut tsi = TrueStrengthIndex::new();

        // Feed downtrend data
        for i in 1..=60 {
            tsi.update_bar(0.0, 0.0, 0.0, 200.0 - i as f64, 0.0);
        }

        assert!(tsi.is_ready());
        // In downtrend, TSI should be negative
        assert!(tsi.value().main() < 0.0, "TSI in downtrend should be negative, got {}", tsi.value().main());
    }

    #[test]
    fn test_tsi_range() {
        let mut tsi = TrueStrengthIndex::new();

        // Feed mixed data
        for i in 1..=60 {
            let price = if i % 2 == 0 { 100.0 + i as f64 } else { 100.0 };
            tsi.update_bar(0.0, 0.0, 0.0, price, 0.0);
        }

        if tsi.is_ready() {
            let val = tsi.value().main();
            assert!(val >= -100.0 && val <= 100.0, "TSI should be in [-100, 100], got {}", val);
        }
    }

    #[test]
    fn test_tsi_with_custom_params() {
        let mut tsi = TrueStrengthIndex::with_params(25, 13, 7, MovingAverageType::EMA);

        for i in 1..=60 {
            tsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(tsi.is_ready());
        let (first, second, signal) = tsi.periods();
        assert_eq!(first, 25);
        assert_eq!(second, 13);
        assert_eq!(signal, 7);
    }

    #[test]
    fn test_tsi_signal_line() {
        let mut tsi = TrueStrengthIndex::new();

        for i in 1..=60 {
            tsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(tsi.is_ready());
        // Signal line should exist
        let signal = tsi.signal_value();
        assert!(signal.is_finite());
    }

    #[test]
    fn test_tsi_histogram() {
        let mut tsi = TrueStrengthIndex::new();

        for i in 1..=60 {
            tsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(tsi.is_ready());
        let hist = tsi.histogram();
        let expected = tsi.value().main() - tsi.signal_value();
        assert!((hist - expected).abs() < 1e-10);
    }

    #[test]
    fn test_tsi_reset() {
        let mut tsi = TrueStrengthIndex::new();

        for i in 1..=60 {
            tsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(tsi.is_ready());

        tsi.reset();
        assert!(!tsi.is_ready());
        assert!((tsi.value().main()).abs() < 1e-10);
    }

    #[test]
    fn test_tsi_market_condition() {
        let mut tsi = TrueStrengthIndex::new();

        // Strong uptrend
        for i in 1..=60 {
            tsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64 * 2.0, 0.0);
        }

        if tsi.is_ready() && tsi.value().main() > 25.0 {
            assert_eq!(tsi.market_condition(), "Strong Bullish");
        }
    }

    #[test]
    fn test_tsi_momentum_strength() {
        let mut tsi = TrueStrengthIndex::new();

        for i in 1..=60 {
            tsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        if tsi.is_ready() {
            let strength = tsi.momentum_strength();
            assert!(strength >= 0.0);
            assert_eq!(strength, tsi.value().main().abs());
        }
    }

    // =========================================================================
    // Source field tests
    // =========================================================================

    #[test]
    fn test_tsi_with_source_hl2() {
        let mut tsi = TrueStrengthIndex::with_source(OhlcvField::HL2);

        // Feed data with clear high/low pattern
        for i in 1..=60 {
            let high = 110.0 + i as f64;
            let low = 90.0 + i as f64;
            tsi.update_bar(100.0, high, low, 105.0, 1000.0);
        }

        assert!(tsi.is_ready());
        assert_eq!(tsi.source(), OhlcvField::HL2);
        // HL2 is trending up, so TSI should be positive
        assert!(tsi.value().main() > 0.0);
    }

    #[test]
    fn test_tsi_with_source_ohlc4() {
        let mut tsi = TrueStrengthIndex::with_source(OhlcvField::OHLC4);

        // Feed uptrend data
        for i in 1..=60 {
            let open = 100.0 + i as f64;
            let high = 110.0 + i as f64;
            let low = 90.0 + i as f64;
            let close = 105.0 + i as f64;
            tsi.update_bar(open, high, low, close, 1000.0);
        }

        assert!(tsi.is_ready());
        assert_eq!(tsi.source(), OhlcvField::OHLC4);
        // OHLC4 is trending up, so TSI should be positive
        assert!(tsi.value().main() > 0.0);
    }

    #[test]
    fn test_tsi_source_affects_calculation() {
        let mut tsi_close = TrueStrengthIndex::with_source(OhlcvField::Close);
        let mut tsi_open = TrueStrengthIndex::with_source(OhlcvField::Open);

        // Feed data where close is trending up but open is trending down
        for i in 1..=60 {
            let open = 200.0 - i as f64;  // Downtrend
            let close = 100.0 + i as f64;  // Uptrend
            tsi_close.update_bar(open, 150.0, 50.0, close, 1000.0);
            tsi_open.update_bar(open, 150.0, 50.0, close, 1000.0);
        }

        if tsi_close.is_ready() && tsi_open.is_ready() {
            // TSI based on Close should be positive (uptrend)
            assert!(tsi_close.value().main() > 0.0, "TSI on Close should be positive");
            // TSI based on Open should be negative (downtrend)
            assert!(tsi_open.value().main() < 0.0, "TSI on Open should be negative");
        }
    }

    // =========================================================================
    // MA type tests
    // =========================================================================

    #[test]
    fn test_tsi_with_ma_types() {
        let tsi = TrueStrengthIndex::with_ma_types(MovingAverageType::SMA, MovingAverageType::WMA);

        assert_eq!(tsi.smoothing_ma_type(), MovingAverageType::SMA);
        assert_eq!(tsi.signal_ma_type(), MovingAverageType::WMA);
        assert_eq!(tsi.source(), OhlcvField::Close);
    }

    #[test]
    fn test_tsi_with_full_config() {
        let tsi = TrueStrengthIndex::with_full_config(
            20, 10, 7,
            MovingAverageType::EMA,
            MovingAverageType::SMA,
            OhlcvField::HL2,
        );

        let (first, second, signal) = tsi.periods();
        assert_eq!(first, 20);
        assert_eq!(second, 10);
        assert_eq!(signal, 7);
        assert_eq!(tsi.smoothing_ma_type(), MovingAverageType::EMA);
        assert_eq!(tsi.signal_ma_type(), MovingAverageType::SMA);
        assert_eq!(tsi.source(), OhlcvField::HL2);
    }

    #[test]
    fn test_tsi_set_smoothing_ma_type() {
        let mut tsi = TrueStrengthIndex::new();

        // Feed some data
        for i in 1..=30 {
            tsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        // Change smoothing MA type - should reset
        tsi.set_smoothing_ma_type(MovingAverageType::SMA);
        assert!(!tsi.is_ready());
        assert_eq!(tsi.smoothing_ma_type(), MovingAverageType::SMA);
    }

    #[test]
    fn test_tsi_set_signal_ma_type() {
        let mut tsi = TrueStrengthIndex::new();

        // Feed some data
        for i in 1..=30 {
            tsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        // Change signal MA type - should reset
        tsi.set_signal_ma_type(MovingAverageType::WMA);
        assert!(!tsi.is_ready());
        assert_eq!(tsi.signal_ma_type(), MovingAverageType::WMA);
    }

    #[test]
    fn test_tsi_set_source() {
        let mut tsi = TrueStrengthIndex::new();

        // Feed some data
        for i in 1..=30 {
            tsi.update_bar(0.0, 110.0, 90.0, 100.0 + i as f64, 0.0);
        }

        // Change source - should reset
        tsi.set_source(OhlcvField::HL2);
        assert!(!tsi.is_ready());
        assert_eq!(tsi.source(), OhlcvField::HL2);
    }

    #[test]
    fn test_tsi_backward_compatibility() {
        // Test that old code still works
        let mut tsi = TrueStrengthIndex::with_params(25, 13, 13, MovingAverageType::EMA);

        for i in 1..=60 {
            tsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(tsi.is_ready());
        // Both smoothing and signal should use the same MA type
        assert_eq!(tsi.smoothing_ma_type(), MovingAverageType::EMA);
        assert_eq!(tsi.signal_ma_type(), MovingAverageType::EMA);
        // Source should default to Close
        assert_eq!(tsi.source(), OhlcvField::Close);
    }
}






















