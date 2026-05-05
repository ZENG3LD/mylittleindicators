use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Rolling Z-Score of Close: (close - MA(close,n)) / StdDev(close,n)
#[derive(Debug, Clone)]
pub struct PriceZScore {
    mean_ma: MovingAverageProvider,
    var_ma: MovingAverageProvider,
    value: f64,
}

impl PriceZScore {
    pub fn new(period: usize) -> Self {
        let n = period.max(2);
        Self {
            mean_ma: MovingAverageProvider::new(MovingAverageType::SMA, n),
            var_ma: MovingAverageProvider::new(MovingAverageType::SMA, n),
            value: 0.0,
        }
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let mean = self.mean_ma.update_bar(0.0, 0.0, 0.0, c, 0.0);
        let diff = c - mean;
        let var = self.var_ma.update_bar(0.0, 0.0, 0.0, diff * diff, 0.0);
        let std = var.max(0.0).sqrt();
        self.value = if std > 0.0 { diff / std } else { 0.0 };
        self.value
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.mean_ma.is_ready() && self.var_ma.is_ready()
    }
    pub fn reset(&mut self) {
        self.mean_ma.reset();
        self.var_ma.reset();
        self.value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_zscore_creation() {
        let pz = PriceZScore::new(20);
        assert!(!pz.is_ready());
        assert_eq!(pz.value().main(), 0.0);
    }

    #[test]
    fn test_price_zscore_warmup() {
        let mut pz = PriceZScore::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pz.is_ready());
    }

    #[test]
    fn test_price_zscore_values() {
        let mut pz = PriceZScore::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Z-score should be finite");
        }
    }

    #[test]
    fn test_price_zscore_reset() {
        let mut pz = PriceZScore::new(20);
        for i in 0..25 {
            pz.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        pz.reset();
        assert!(!pz.is_ready());
        assert_eq!(pz.value().main(), 0.0);
    }
}
