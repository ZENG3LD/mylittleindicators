// Elder Impulse System (placeholder): combination of EMA slope and MACD histogram sign

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

#[derive(Debug, Clone)]
pub struct ElderImpulseSystem {
    ema_period: usize,
    ma_type: MovingAverageType,
    source: OhlcvField,
    ema: MovingAverageProvider,
    prev_ema: f64,
    macd_hist: f64,
    value: i8,
}

impl ElderImpulseSystem {
    /// Creates a new Elder Impulse System with default settings (EMA, Close source).
    ///
    /// # Arguments
    /// * `ema_period` - Period for the moving average (typically 13)
    pub fn new(ema_period: usize) -> Self {
        Self::with_ma_type(ema_period, MovingAverageType::EMA)
    }

    /// Creates a new Elder Impulse System with specified MA type.
    ///
    /// # Arguments
    /// * `ema_period` - Period for the moving average
    /// * `ma_type` - Type of moving average to use
    pub fn with_ma_type(ema_period: usize, ma_type: MovingAverageType) -> Self {
        let period = ema_period.max(2);
        Self {
            ema_period: period,
            ma_type,
            source: OhlcvField::Close,
            ema: MovingAverageProvider::new(ma_type, period),
            prev_ema: 0.0,
            macd_hist: 0.0,
            value: 0,
        }
    }

    /// Creates a new Elder Impulse System with custom source field.
    ///
    /// # Arguments
    /// * `ema_period` - Period for the moving average
    /// * `source` - OHLCV field to use as input
    pub fn with_source(ema_period: usize, source: OhlcvField) -> Self {
        let mut ei = Self::with_ma_type(ema_period, MovingAverageType::EMA);
        ei.source = source;
        ei
    }

    /// Creates a new Elder Impulse System with full configuration.
    ///
    /// # Arguments
    /// * `ema_period` - Period for the moving average
    /// * `ma_type` - Type of moving average to use
    /// * `source` - OHLCV field to use as input
    pub fn with_full_config(ema_period: usize, ma_type: MovingAverageType, source: OhlcvField) -> Self {
        let period = ema_period.max(2);
        Self {
            ema_period: period,
            ma_type,
            source,
            ema: MovingAverageProvider::new(ma_type, period),
            prev_ema: 0.0,
            macd_hist: 0.0,
            value: 0,
        }
    }
    /// Returns the MA type used for the moving average.
    #[inline]
    pub fn get_ma_type(&self) -> MovingAverageType {
        self.ma_type
    }

    /// Returns the source field used for calculation.
    #[inline]
    pub fn get_source(&self) -> OhlcvField {
        self.source
    }

    /// Sets the MA type and resets the indicator.
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        if self.ma_type != ma_type {
            self.ma_type = ma_type;
            self.ema = MovingAverageProvider::new(ma_type, self.ema_period);
            self.prev_ema = 0.0;
            self.macd_hist = 0.0;
            self.value = 0;
        }
    }

    /// Sets the source field and resets the indicator.
    pub fn set_source(&mut self, source: OhlcvField) {
        if self.source != source {
            self.source = source;
            self.reset();
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ema = MovingAverageProvider::new(self.ma_type, self.ema_period);
        self.prev_ema = 0.0;
        self.macd_hist = 0.0;
        self.value = 0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ema.is_ready()
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.value)
    }

    /// Updates the Elder Impulse System with a new bar.
    ///
    /// Extracts value from the configured source field.
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> i8 {
        let price = self.source.extract(open, high, low, close, volume);

        let ema_now = self.ema.update_bar(0.0, 0.0, 0.0, price, 0.0);
        let ema_slope_up = ema_now > self.prev_ema + 1e-12;
        self.prev_ema = ema_now;
        // placeholder MACD histogram sign using simple momentum
        self.macd_hist = price - ema_now;
        let macd_up = self.macd_hist > 0.0;
        self.value = match (ema_slope_up, macd_up) {
            (true, true) => 1,
            (false, false) => -1,
            _ => 0,
        };
        self.value
    }

    pub fn value_signal(&self) -> i8 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elder_impulse_creation() {
        let ei = ElderImpulseSystem::new(13);
        assert!(!ei.is_ready());
        assert_eq!(ei.value_signal(), 0);
    }

    #[test]
    fn test_elder_impulse_uptrend() {
        let mut ei = ElderImpulseSystem::new(10);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            ei.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ei.is_ready());
        // In uptrend: EMA rising + close > EMA => signal 1
        assert_eq!(ei.value_signal(), 1, "Elder Impulse should signal 1 in uptrend");
    }

    #[test]
    fn test_elder_impulse_downtrend() {
        let mut ei = ElderImpulseSystem::new(10);
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            ei.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ei.is_ready());
        // In downtrend: EMA falling + close < EMA => signal -1
        assert_eq!(ei.value_signal(), -1, "Elder Impulse should signal -1 in downtrend");
    }

    #[test]
    fn test_elder_impulse_reset() {
        let mut ei = ElderImpulseSystem::new(10);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            ei.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ei.is_ready());
        ei.reset();
        assert!(!ei.is_ready());
        assert_eq!(ei.value_signal(), 0);
    }

    #[test]
    fn test_elder_impulse_signal_range() {
        let mut ei = ElderImpulseSystem::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let signal = ei.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(signal >= -1 && signal <= 1, "Elder Impulse should be -1, 0, or 1, got {}", signal);
        }
    }

    #[test]
    fn test_elder_impulse_value_types() {
        let ei = ElderImpulseSystem::new(13);
        match ei.value() {
            IndicatorValue::Signal(s) => assert_eq!(s, 0),
            _ => panic!("Elder Impulse should return Signal value"),
        }
    }

    // =========================================================================
    // MA type tests
    // =========================================================================

    #[test]
    fn test_elder_impulse_with_ma_type() {
        let mut ei = ElderImpulseSystem::with_ma_type(10, MovingAverageType::SMA);

        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            ei.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }

        assert!(ei.is_ready());
        assert_eq!(ei.get_ma_type(), MovingAverageType::SMA);
        assert_eq!(ei.value_signal(), 1, "Elder Impulse with SMA should signal 1 in uptrend");
    }

    #[test]
    fn test_elder_impulse_with_wma() {
        let mut ei = ElderImpulseSystem::with_ma_type(10, MovingAverageType::WMA);

        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            ei.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }

        assert!(ei.is_ready());
        assert_eq!(ei.get_ma_type(), MovingAverageType::WMA);
        assert_eq!(ei.value_signal(), 1);
    }

    #[test]
    fn test_elder_impulse_set_ma_type() {
        let mut ei = ElderImpulseSystem::new(10);

        // Feed some data
        for i in 1..=15 {
            let price = 100.0 + i as f64;
            ei.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ei.is_ready());

        // Change MA type - should reset
        ei.set_ma_type(MovingAverageType::SMA);
        assert_eq!(ei.get_ma_type(), MovingAverageType::SMA);
        assert!(!ei.is_ready());
        assert_eq!(ei.value_signal(), 0);
    }

    #[test]
    fn test_elder_impulse_default_ma_type_is_ema() {
        let ei = ElderImpulseSystem::new(13);
        assert_eq!(ei.get_ma_type(), MovingAverageType::EMA);
    }

    // =========================================================================
    // Source field tests
    // =========================================================================

    #[test]
    fn test_elder_impulse_with_source() {
        let mut ei = ElderImpulseSystem::with_source(10, OhlcvField::High);

        for i in 1..=30 {
            let high = 100.0 + i as f64 * 2.0;
            ei.update_bar(90.0, high, 80.0, 95.0, 1000.0);
        }

        assert!(ei.is_ready());
        assert_eq!(ei.get_source(), OhlcvField::High);
        // High is trending up strongly, should signal 1
        assert_eq!(ei.value_signal(), 1);
    }

    #[test]
    fn test_elder_impulse_with_hl2_source() {
        let mut ei = ElderImpulseSystem::with_source(10, OhlcvField::HL2);

        for i in 1..=30 {
            let base = 100.0 + i as f64 * 2.0;
            ei.update_bar(base, base + 10.0, base - 10.0, base + 5.0, 1000.0);
        }

        assert!(ei.is_ready());
        assert_eq!(ei.get_source(), OhlcvField::HL2);
        // HL2 is trending up, should signal 1
        assert_eq!(ei.value_signal(), 1);
    }

    #[test]
    fn test_elder_impulse_default_source_is_close() {
        let ei = ElderImpulseSystem::new(13);
        assert_eq!(ei.get_source(), OhlcvField::Close);
    }

    #[test]
    fn test_elder_impulse_source_affects_calculation() {
        let mut ei_close = ElderImpulseSystem::with_source(10, OhlcvField::Close);
        let mut ei_open = ElderImpulseSystem::with_source(10, OhlcvField::Open);

        // Feed data where close is trending up but open is trending down
        for i in 1..=30 {
            let open = 200.0 - i as f64 * 2.0;  // Downtrend
            let close = 100.0 + i as f64 * 2.0;  // Uptrend
            ei_close.update_bar(open, 150.0, 50.0, close, 1000.0);
            ei_open.update_bar(open, 150.0, 50.0, close, 1000.0);
        }

        assert!(ei_close.is_ready());
        assert!(ei_open.is_ready());

        // Close-based should signal bullish (1)
        assert_eq!(ei_close.value_signal(), 1, "Elder Impulse on Close should be bullish");
        // Open-based should signal bearish (-1)
        assert_eq!(ei_open.value_signal(), -1, "Elder Impulse on Open should be bearish");
    }

    #[test]
    fn test_elder_impulse_set_source() {
        let mut ei = ElderImpulseSystem::new(10);

        // Feed some data
        for i in 1..=15 {
            let price = 100.0 + i as f64;
            ei.update_bar(price, price + 10.0, price - 10.0, price, 1000.0);
        }
        assert!(ei.is_ready());

        // Change source - should reset
        ei.set_source(OhlcvField::HL2);
        assert_eq!(ei.get_source(), OhlcvField::HL2);
        assert!(!ei.is_ready());
    }

    // =========================================================================
    // Full config tests
    // =========================================================================

    #[test]
    fn test_elder_impulse_with_full_config() {
        let mut ei = ElderImpulseSystem::with_full_config(
            10,
            MovingAverageType::SMA,
            OhlcvField::HL2,
        );

        for i in 1..=30 {
            let base = 100.0 + i as f64 * 2.0;
            ei.update_bar(base, base + 10.0, base - 10.0, base + 5.0, 1000.0);
        }

        assert!(ei.is_ready());
        assert_eq!(ei.get_ma_type(), MovingAverageType::SMA);
        assert_eq!(ei.get_source(), OhlcvField::HL2);
        assert_eq!(ei.value_signal(), 1);
    }

    #[test]
    fn test_elder_impulse_default_and_close_source_match() {
        let mut ei_default = ElderImpulseSystem::new(10);
        let mut ei_close = ElderImpulseSystem::with_source(10, OhlcvField::Close);

        // Both should produce the same result
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            let v1 = ei_default.update_bar(90.0, 110.0, 80.0, price, 1000.0);
            let v2 = ei_close.update_bar(90.0, 110.0, 80.0, price, 1000.0);
            assert_eq!(v1, v2, "Default and Close source should match at bar {}", i);
        }

        assert_eq!(ei_default.value_signal(), ei_close.value_signal());
    }
}
