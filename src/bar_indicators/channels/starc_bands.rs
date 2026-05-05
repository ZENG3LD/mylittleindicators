use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;
use crate::bar_indicators::volatility::atr::Atr;

/// STARC Bands: center = MA(close, n); upper = center + k*ATR(m); lower = center - k*ATR(m)
#[derive(Debug, Clone)]
pub struct StarcBands {
    ma_period: usize,
    ma_type: MovingAverageType,
    source: OhlcvField,
    ma: MovingAverageProvider,
    atr: Atr,
    k: f64,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl StarcBands {
    /// Create STARC Bands with default MA type (SMA)
    pub fn new(ma_period: usize, atr_period: usize, k: f64) -> Self {
        Self::new_with_ma_type(ma_period, atr_period, k, MovingAverageType::SMA)
    }

    /// Create STARC Bands with specified MA type
    pub fn new_with_ma_type(ma_period: usize, atr_period: usize, k: f64, ma_type: MovingAverageType) -> Self {
        let ma_p = ma_period.max(1);
        Self {
            ma_period: ma_p,
            ma_type,
            source: OhlcvField::Close,
            ma: MovingAverageProvider::new(ma_type, ma_p),
            atr: Atr::new_wilder(atr_period.max(1)),
            k,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    /// Create STARC Bands with custom source
    pub fn with_source(ma_period: usize, atr_period: usize, k: f64, ma_type: MovingAverageType, source: OhlcvField) -> Self {
        let ma_p = ma_period.max(1);
        Self {
            ma_period: ma_p,
            ma_type,
            source,
            ma: MovingAverageProvider::new(ma_type, ma_p),
            atr: Atr::new_wilder(atr_period.max(1)),
            k,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    /// Set the MA type and reset the indicator
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> (f64, f64, f64) {
        let price = self.source.extract(o, h, l, c, v);
        self.middle = self.ma.update_bar(0.0, 0.0, 0.0, price, 0.0);
        let _ = self.atr.update_bar(0.0, h, l, c, v);
        let atrv = self.atr.value().main();
        self.upper = self.middle + self.k * atrv;
        self.lower = self.middle - self.k * atrv;
        (self.upper, self.middle, self.lower)
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.upper,
            middle: self.middle,
            lower: self.lower,
        }
    }

    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper, self.middle, self.lower)
    }
    pub fn is_ready(&self) -> bool {
        self.ma.is_ready() && self.atr.is_ready()
    }
    pub fn reset(&mut self) {
        self.ma = MovingAverageProvider::new(self.ma_type, self.ma_period);
        self.atr.reset();
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starc_bands_creation() {
        let sb = StarcBands::new(20, 14, 2.0);
        assert!(!sb.is_ready());
        assert_eq!(sb.upper, 0.0);
        assert_eq!(sb.lower, 0.0);
    }

    #[test]
    fn test_starc_bands_warmup() {
        let mut sb = StarcBands::new(20, 14, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sb.is_ready());
    }

    #[test]
    fn test_starc_bands_values() {
        let mut sb = StarcBands::new(20, 14, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            sb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sb.upper >= sb.middle);
        assert!(sb.middle >= sb.lower);
    }

    #[test]
    fn test_starc_bands_with_ema() {
        let mut sb = StarcBands::new_with_ma_type(20, 14, 2.0, MovingAverageType::EMA);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            sb.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
        }
        assert!(sb.is_ready());
    }

    #[test]
    fn test_starc_bands_reset() {
        let mut sb = StarcBands::new(20, 14, 2.0);
        for i in 0..25 {
            sb.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        sb.reset();
        assert!(!sb.is_ready());
        assert_eq!(sb.upper, 0.0);
        assert_eq!(sb.lower, 0.0);
    }
}
