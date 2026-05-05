// High-performance Pressure indicator
// (c) 2024

use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::average::moving_average::{
    MovingAverageType, MovingAverageWithField, OhlcvField
};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct Pressure {
    atr: Atr,
    avg_volume: MovingAverageWithField,
    value: f64,
    value_cumulative: f64,
}

impl Pressure {
    /// Create new Pressure indicator
    ///
    /// # Arguments
    /// * `period` - Lookback period for ATR and volume averaging
    /// * `ma_type` - Type of moving average for volume smoothing
    pub fn new(period: usize, ma_type: MovingAverageType) -> Self {
        Self {
            atr: Atr::new(period, MovingAverageType::RMA),
            avg_volume: MovingAverageWithField::new(ma_type, period, OhlcvField::Volume),
            value: 0.0,
            value_cumulative: 0.0,
        }
    }
    /// Update Pressure with new OHLCV bar
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        self.atr.update_bar(open, high, low, close, volume);
        // Update average volume (extracts Volume field automatically)
        self.avg_volume.update_bar(open, high, low, close, volume);

        if !self.atr.is_ready() || !self.avg_volume.is_ready() {
            self.value = 0.0;
        } else {
            let atr = self.atr.value().main();
            let avg_volume = self.avg_volume.value().main();

            if avg_volume.abs() < 1e-12 || atr.abs() < 1e-12 {
                self.value = 0.0;
            } else {
                let rel_volume = volume / avg_volume;
                let buy_pressure = ((close - low) / atr) * rel_volume;
                let sell_pressure = ((high - close) / atr) * rel_volume;
                self.value = buy_pressure - sell_pressure;
            }
        }
        self.value_cumulative += self.value;
        self.value
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn value_cumulative(&self) -> f64 {
        self.value_cumulative
    }
    pub fn is_ready(&self) -> bool {
        self.atr.is_ready() && self.avg_volume.is_ready()
    }
    pub fn reset(&mut self) {
        self.atr.reset();
        self.avg_volume.reset();
        self.value = 0.0;
        self.value_cumulative = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pressure_creation() {
        let p = Pressure::new(14, MovingAverageType::EMA);
        assert!(!p.is_ready());
        assert_eq!(p.value().main(), 0.0);
    }

    #[test]
    fn test_pressure_bullish() {
        let mut p = Pressure::new(10, MovingAverageType::EMA);
        for i in 1..=30 {
            let low = 100.0 + i as f64;
            let high = low + 5.0;
            let close = high - 0.5; // close near high = bullish
            p.update_bar(low + 2.0, high, low, close, 1000.0);
        }
        assert!(p.is_ready());
        // Close near high = buy pressure > sell pressure
        assert!(p.value().main() > 0.0, "Pressure should be positive when close near high, got {}", p.value().main());
    }

    #[test]
    fn test_pressure_bearish() {
        let mut p = Pressure::new(10, MovingAverageType::EMA);
        for i in 1..=30 {
            let high = 200.0 - i as f64;
            let low = high - 5.0;
            let close = low + 0.5; // close near low = bearish
            p.update_bar(high - 2.0, high, low, close, 1000.0);
        }
        assert!(p.is_ready());
        // Close near low = sell pressure > buy pressure
        assert!(p.value().main() < 0.0, "Pressure should be negative when close near low, got {}", p.value().main());
    }

    #[test]
    fn test_pressure_reset() {
        let mut p = Pressure::new(10, MovingAverageType::EMA);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            p.update_bar(price, price + 2.0, price - 2.0, price + 1.0, 1000.0);
        }
        assert!(p.is_ready());
        p.reset();
        assert!(!p.is_ready());
        assert_eq!(p.value().main(), 0.0);
        assert_eq!(p.value_cumulative(), 0.0);
    }

    #[test]
    fn test_pressure_finite_values() {
        let mut p = Pressure::new(10, MovingAverageType::EMA);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = p.update_bar(price, price + 2.0, price - 2.0, price + 1.0, 1000.0);
            assert!(value.is_finite(), "Pressure should always be finite");
        }
    }
}






















