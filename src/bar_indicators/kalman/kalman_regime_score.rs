// Continuous Kalman regime score based on velocity z-score with sigmoid mapping

use crate::bar_indicators::kalman::kalman_trend_slope::KalmanTrendSlope;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct KalmanRegimeScore {
    inner: KalmanTrendSlope,
    mean: f64,
    var: f64,
    #[allow(dead_code)]
    decay: f64,
    pub value: f64,
}

impl KalmanRegimeScore {
    pub fn new(
        dt: f64,
        process_noise: f64,
        measurement_noise: f64,
        window: usize,
        decay: f64,
    ) -> Self {
        Self {
            inner: KalmanTrendSlope::new(dt, process_noise, measurement_noise, window),
            mean: 0.0,
            var: 1e-6,
            decay: decay.clamp(0.0, 1.0),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.mean = 0.0;
        self.var = 1e-6;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let (_s, z) = self.inner.update_bar(o, h, l, c, v); // use z directly, smooth with tanh
        self.value = 0.5 * (z.tanh() + 1.0);
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_regime_score_creation() {
        let krs = KalmanRegimeScore::new(1.0, 0.1, 1.0, 20, 0.94);
        assert!(!krs.is_ready());
        assert_eq!(krs.value, 0.0);
    }

    #[test]
    fn test_kalman_regime_score_warmup() {
        let mut krs = KalmanRegimeScore::new(1.0, 0.1, 1.0, 10, 0.94);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            krs.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(krs.is_ready());
    }

    #[test]
    fn test_kalman_regime_score_values_range() {
        let mut krs = KalmanRegimeScore::new(1.0, 0.1, 1.0, 10, 0.94);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = krs.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0);
        }
    }

    #[test]
    fn test_kalman_regime_score_reset() {
        let mut krs = KalmanRegimeScore::new(1.0, 0.1, 1.0, 10, 0.94);
        for i in 0..20 {
            krs.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        krs.reset();
        assert!(!krs.is_ready());
        assert_eq!(krs.value, 0.0);
    }
}
