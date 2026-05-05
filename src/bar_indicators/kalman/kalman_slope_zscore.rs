// Kalman slope Z-score over rolling window

use crate::bar_indicators::kalman::basic_kalman_filter::BasicKalmanFilter;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct KalmanSlopeZscore {
    kf: BasicKalmanFilter,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl KalmanSlopeZscore {
    pub fn new(dt: f64, process_noise: f64, measurement_noise: f64, window: usize) -> Self {
        let w = window.max(20);
        Self {
            kf: BasicKalmanFilter::new(dt, process_noise, measurement_noise),
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.kf.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let estimate = self.kf.update(c);
        let slope = estimate.velocity;
        self.buf[self.idx] = slope;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let n = self.window;
            let mut mean = 0.0;
            for i in 0..n {
                mean += self.buf[i];
            }
            mean /= n as f64;
            let mut var = 0.0;
            for i in 0..n {
                let d = self.buf[i] - mean;
                var += d * d;
            }
            let std = (var / (n as f64)).sqrt().max(1e-9);
            self.value = (slope - mean) / std;
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_slope_zscore_creation() {
        let ksz = KalmanSlopeZscore::new(1.0, 0.1, 1.0, 20);
        assert!(!ksz.is_ready());
        assert_eq!(ksz.value, 0.0);
    }

    #[test]
    fn test_kalman_slope_zscore_warmup() {
        let mut ksz = KalmanSlopeZscore::new(1.0, 0.1, 1.0, 20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ksz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ksz.is_ready());
    }

    #[test]
    fn test_kalman_slope_zscore_values_finite() {
        let mut ksz = KalmanSlopeZscore::new(1.0, 0.1, 1.0, 20);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ksz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_kalman_slope_zscore_reset() {
        let mut ksz = KalmanSlopeZscore::new(1.0, 0.1, 1.0, 20);
        for i in 0..30 {
            ksz.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ksz.reset();
        assert!(!ksz.is_ready());
        assert_eq!(ksz.value, 0.0);
    }
}
