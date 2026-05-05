// Kalman Trend Slope and Z-Score using BasicKalmanFilter velocity/acceleration

use crate::bar_indicators::kalman::basic_kalman_filter::BasicKalmanFilter;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct KalmanTrendSlope {
    kf: BasicKalmanFilter,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub slope: f64,
    pub slope_z: f64,
}

impl KalmanTrendSlope {
    pub fn new(dt: f64, process_noise: f64, measurement_noise: f64, window: usize) -> Self {
        Self {
            kf: BasicKalmanFilter::new(dt, process_noise, measurement_noise),
            window: window.max(10),
            buf: vec![0.0; window.max(10)],
            idx: 0,
            filled: false,
            slope: 0.0,
            slope_z: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.kf.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.slope = 0.0;
        self.slope_z = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.window >= 10
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.slope, self.slope_z)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> (f64, f64) {
        let res = self.kf.update(c);
        self.slope = res.velocity;
        // ring buffer for z-score
        let _old = self.buf[self.idx];
        self.buf[self.idx] = self.slope;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        let n = if self.filled {
            self.window
        } else {
            self.idx.max(1)
        };
        let mut sum = 0.0;
        let mut sumsq = 0.0;
        for i in 0..n {
            let v = self.buf[i];
            sum += v;
            sumsq += v * v;
        }
        let mean = sum / n as f64;
        let var = (sumsq / n as f64) - mean * mean;
        let sd = var.max(1e-12).sqrt();
        self.slope_z = (self.slope - mean) / sd;
        (self.slope, self.slope_z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_trend_slope_creation() {
        let kts = KalmanTrendSlope::new(1.0, 0.1, 1.0, 20);
        assert!(!kts.is_ready());
        assert_eq!(kts.slope, 0.0);
        assert_eq!(kts.slope_z, 0.0);
    }

    #[test]
    fn test_kalman_trend_slope_warmup() {
        let mut kts = KalmanTrendSlope::new(1.0, 0.1, 1.0, 10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kts.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(kts.is_ready());
    }

    #[test]
    fn test_kalman_trend_slope_values_finite() {
        let mut kts = KalmanTrendSlope::new(1.0, 0.1, 1.0, 10);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (slope, slope_z) = kts.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(slope.is_finite());
            assert!(slope_z.is_finite());
        }
    }

    #[test]
    fn test_kalman_trend_slope_reset() {
        let mut kts = KalmanTrendSlope::new(1.0, 0.1, 1.0, 10);
        for i in 0..20 {
            kts.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        kts.reset();
        assert!(!kts.is_ready());
        assert_eq!(kts.slope, 0.0);
        assert_eq!(kts.slope_z, 0.0);
    }
}
