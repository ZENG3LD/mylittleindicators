use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Intraday Intensity (II) and IIP21 variant
#[derive(Debug, Clone)]
pub struct IntradayIntensity {
    ii_ma: MovingAverageProvider,
    vol_ma: MovingAverageProvider,
    value: f64,
}

impl IntradayIntensity {
    pub fn new(period: usize) -> Self {
        Self {
            ii_ma: MovingAverageProvider::new(MovingAverageType::SMA, period.max(1)),
            vol_ma: MovingAverageProvider::new(MovingAverageType::SMA, period.max(1)),
            value: 0.0,
        }
    }
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let hl = (h - l).abs().max(1e-12);
        let ii = ((2.0 * c - h - l) / hl) * v;
        let num = self.ii_ma.update_bar(0.0, 0.0, 0.0, ii, 0.0);
        let den = self.vol_ma.update_bar(0.0, 0.0, 0.0, v, 0.0);
        self.value = if den.abs() < 1e-12 {
            0.0
        } else {
            100.0 * num / den
        };
        self.value
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.ii_ma.is_ready() && self.vol_ma.is_ready()
    }
    pub fn reset(&mut self) {
        self.ii_ma.reset();
        self.vol_ma.reset();
        self.value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intraday_intensity_creation() {
        let ii = IntradayIntensity::new(21);
        assert!(!ii.is_ready());
        assert_eq!(ii.value().main(), 0.0);
    }

    #[test]
    fn test_intraday_intensity_warmup() {
        let mut ii = IntradayIntensity::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ii.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ii.is_ready());
    }

    #[test]
    fn test_intraday_intensity_values_finite() {
        let mut ii = IntradayIntensity::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ii.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_intraday_intensity_reset() {
        let mut ii = IntradayIntensity::new(14);
        for i in 0..20 {
            ii.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ii.reset();
        assert!(!ii.is_ready());
        assert_eq!(ii.value().main(), 0.0);
    }
}
