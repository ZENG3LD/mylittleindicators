use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Twiggs Money Flow (TMF) = EMA( (Close - Low) - (High - Close) / (High - Low), n ) * Volume smoothed / Volume smoothed
#[derive(Debug, Clone)]
pub struct Tmf {
    #[allow(dead_code)]
    n: usize,
    cmf_num_ma: MovingAverageProvider,
    vol_ma: MovingAverageProvider,
    value: f64,
}

impl Tmf {
    pub fn new(period: usize) -> Self {
        Self {
            n: period.max(1),
            cmf_num_ma: MovingAverageProvider::new(MovingAverageType::EMA, period.max(1)),
            vol_ma: MovingAverageProvider::new(MovingAverageType::EMA, period.max(1)),
            value: 0.0,
        }
    }
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let hl = (h - l).abs().max(1e-12);
        let mf = ((c - l) - (h - c)) / hl;
        let num = self.cmf_num_ma.update_bar(0.0, 0.0, 0.0, mf * v, 0.0);
        let den = self.vol_ma.update_bar(0.0, 0.0, 0.0, v, 0.0);
        self.value = if den.abs() < 1e-12 { 0.0 } else { num / den };
        self.value
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.cmf_num_ma.is_ready() && self.vol_ma.is_ready()
    }
    pub fn reset(&mut self) {
        self.cmf_num_ma.reset();
        self.vol_ma.reset();
        self.value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tmf_creation() {
        let tmf = Tmf::new(21);
        assert!(!tmf.is_ready());
        assert_eq!(tmf.value().main(), 0.0);
    }

    #[test]
    fn test_tmf_warmup() {
        let mut tmf = Tmf::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            tmf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(tmf.is_ready());
    }

    #[test]
    fn test_tmf_values_finite() {
        let mut tmf = Tmf::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = tmf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_tmf_values_range() {
        let mut tmf = Tmf::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = tmf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= -1.0 && value <= 1.0);
        }
    }

    #[test]
    fn test_tmf_reset() {
        let mut tmf = Tmf::new(14);
        for i in 0..20 {
            tmf.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        tmf.reset();
        assert!(!tmf.is_ready());
        assert_eq!(tmf.value().main(), 0.0);
    }
}
