// KAMA Slope: slope of Kaufman Adaptive Moving Average

use crate::bar_indicators::adaptive::kaufman_adaptive_ma::KaufmanAdaptiveMA;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct KamaSlope {
    kama: KaufmanAdaptiveMA,
    prev: f64,
    value: f64,
}

impl KamaSlope {
    pub fn new(period: usize) -> Self {
        Self {
            kama: KaufmanAdaptiveMA::new(period.max(2), 2, 30),
            prev: 0.0,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.kama.reset();
        self.prev = 0.0;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.kama.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let k = self.kama.update(c);
        self.value = k - self.prev;
        self.prev = k;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kama_slope_creation() {
        let kama = KamaSlope::new(10);
        assert!(!kama.is_ready());
        assert_eq!(kama.value().main(), 0.0);
    }

    #[test]
    fn test_kama_slope_warmup() {
        let mut kama = KamaSlope::new(10);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kama.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(kama.is_ready());
    }

    #[test]
    fn test_kama_slope_values_finite() {
        let mut kama = KamaSlope::new(10);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let value = kama.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_kama_slope_reset() {
        let mut kama = KamaSlope::new(10);
        for i in 0..20 {
            kama.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        kama.reset();
        assert!(!kama.is_ready());
        assert_eq!(kama.value().main(), 0.0);
    }
}
