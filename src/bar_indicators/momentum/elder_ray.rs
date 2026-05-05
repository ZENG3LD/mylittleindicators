use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Elder-Ray: Bull Power = High - EMA(close), Bear Power = Low - EMA(close)
#[derive(Debug, Clone)]
pub struct ElderRay {
    ma_type: MovingAverageType,
    period: usize,
    ema: MovingAverageProvider,
    bull: f64,
    bear: f64,
    ready: bool,
}

impl ElderRay {
    pub fn new(period: usize) -> Self {
        Self::new_with_ma_type(period, MovingAverageType::EMA)
    }

    pub fn new_default(period: usize) -> Self {
        Self::new_with_ma_type(period, MovingAverageType::EMA)
    }

    pub fn new_with_ma_type(period: usize, ma_type: MovingAverageType) -> Self {
        let p = period.max(1);
        Self {
            ma_type,
            period: p,
            ema: MovingAverageProvider::new(ma_type, p),
            bull: 0.0,
            bear: 0.0,
            ready: false,
        }
    }

    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, _v: f64) -> (f64, f64) {
        self.ema.update_bar(0.0, 0.0, 0.0, c, 0.0);
        let ema = self.ema.value().main();
        self.bull = h - ema;
        self.bear = l - ema;
        self.ready = self.ema.is_ready();
        (self.bull, self.bear)
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.bull, self.bear)
    }
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn reset(&mut self) {
        self.ema = MovingAverageProvider::new(self.ma_type, self.period);
        self.bull = 0.0;
        self.bear = 0.0;
        self.ready = false;
    }

    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elder_ray_creation() {
        let er = ElderRay::new(13);
        assert!(!er.is_ready());
        assert_eq!(er.value(), IndicatorValue::Double(0.0, 0.0));
        assert_eq!(er.period(), 13);
    }

    #[test]
    fn test_elder_ray_default() {
        let er1 = ElderRay::new(13);
        let er2 = ElderRay::new_default(13);
        assert_eq!(er1.period(), er2.period());
    }

    #[test]
    fn test_elder_ray_uptrend() {
        let mut er = ElderRay::new(10);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            er.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(er.is_ready());
        if let IndicatorValue::Double(bull, bear) = er.value() {
            // In uptrend: high > EMA, so bull > 0
            assert!(bull > 0.0, "Bull power should be positive in uptrend, got {}", bull);
            // Bear power = low - EMA, could be negative or positive
            assert!(bear.is_finite());
        } else { panic!("Expected Double"); }
    }

    #[test]
    fn test_elder_ray_downtrend() {
        let mut er = ElderRay::new(10);
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            er.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(er.is_ready());
        if let IndicatorValue::Double(bull, bear) = er.value() {
            // In downtrend: low < EMA, so bear < 0
            assert!(bear < 0.0, "Bear power should be negative in downtrend, got {}", bear);
            assert!(bull.is_finite());
        } else { panic!("Expected Double"); }
    }

    #[test]
    fn test_elder_ray_reset() {
        let mut er = ElderRay::new(10);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            er.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(er.is_ready());
        er.reset();
        assert!(!er.is_ready());
        assert_eq!(er.value(), IndicatorValue::Double(0.0, 0.0));
    }

    #[test]
    fn test_elder_ray_finite_values() {
        let mut er = ElderRay::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let (bull, bear) = er.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(bull.is_finite(), "Bull power should always be finite");
            assert!(bear.is_finite(), "Bear power should always be finite");
        }
    }

    #[test]
    fn test_elder_ray_set_ma_type() {
        let mut er = ElderRay::new(10);
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            er.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(er.is_ready());
        er.set_ma_type(MovingAverageType::SMA);
        assert!(!er.is_ready()); // should reset
    }
}
