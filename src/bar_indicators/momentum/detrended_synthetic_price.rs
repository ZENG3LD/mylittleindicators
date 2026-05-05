// Detrended Synthetic Price (DSP) - simple proxy: price - MA(price)

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct DetrendedSyntheticPrice {
    period: usize,
    ma_type: MovingAverageType,
    ma: MovingAverageProvider,
    value: f64,
}

impl DetrendedSyntheticPrice {
    /// Create Detrended Synthetic Price with default MA type (SMA)
    pub fn new(period: usize) -> Self {
        Self::new_with_ma_type(period, MovingAverageType::SMA)
    }

    /// Create Detrended Synthetic Price with specified MA type
    pub fn new_with_ma_type(period: usize, ma_type: MovingAverageType) -> Self {
        let p = period.max(2);
        Self {
            period: p,
            ma_type,
            ma: MovingAverageProvider::new(ma_type, p),
            value: 0.0,
        }
    }

    /// Set the MA type and reset the indicator
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }
    #[inline]
    pub fn reset(&mut self) {
        self.ma = MovingAverageProvider::new(self.ma_type, self.period);
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ma.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let m = self.ma.update_bar(0.0, 0.0, 0.0, c, 0.0);
        self.value = c - m;
        self.value
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dsp_creation() {
        let dsp = DetrendedSyntheticPrice::new(14);
        assert!(!dsp.is_ready());
        assert_eq!(dsp.value().main(), 0.0);
        assert_eq!(dsp.period(), 14);
    }

    #[test]
    fn test_dsp_uptrend() {
        let mut dsp = DetrendedSyntheticPrice::new(14);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            dsp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(dsp.is_ready());
        assert!(dsp.value().main() > 0.0, "DSP should be > 0 in uptrend, got {}", dsp.value().main());
    }

    #[test]
    fn test_dsp_downtrend() {
        let mut dsp = DetrendedSyntheticPrice::new(14);
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            dsp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(dsp.is_ready());
        assert!(dsp.value().main() < 0.0, "DSP should be < 0 in downtrend, got {}", dsp.value().main());
    }

    #[test]
    fn test_dsp_finite() {
        let mut dsp = DetrendedSyntheticPrice::new_with_ma_type(14, MovingAverageType::EMA);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = dsp.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "DSP should always be finite");
        }
    }

    #[test]
    fn test_dsp_reset() {
        let mut dsp = DetrendedSyntheticPrice::new(14);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            dsp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(dsp.is_ready());
        dsp.reset();
        assert!(!dsp.is_ready());
        assert_eq!(dsp.value().main(), 0.0);
    }
}
