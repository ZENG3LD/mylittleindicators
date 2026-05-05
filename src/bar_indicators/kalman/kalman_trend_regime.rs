// Kalman Trend Regime: classifies trend regime by velocity z-score

use crate::bar_indicators::kalman::basic_kalman_filter::BasicKalmanFilter;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct KalmanTrendRegime {
    kf: BasicKalmanFilter,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: i8,
}

impl KalmanTrendRegime {
    pub fn new(dt: f64, process_noise: f64, measurement_noise: f64, z_window: usize) -> Self {
        let w = z_window.max(20);
        Self {
            kf: BasicKalmanFilter::new(dt, process_noise, measurement_noise),
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.kf.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.value)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> i8 {
        let est = self.kf.update(c);
        let vel = est.velocity;
        self.buf[self.idx] = vel;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut m = 0.0;
            for &x in &self.buf {
                m += x;
            }
            m /= self.window as f64;
            let mut s = 0.0;
            for &x in &self.buf {
                let d = x - m;
                s += d * d;
            }
            s = (s / (self.window as f64)).sqrt().max(1e-9);
            let z = (vel - m) / s;
            self.value = if z > 1.0 {
                1
            } else if z < -1.0 {
                -1
            } else {
                0
            };
        } else {
            self.value = 0;
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_trend_regime_creation() {
        let ktr = KalmanTrendRegime::new(1.0, 0.1, 1.0, 20);
        assert!(!ktr.is_ready());
        assert_eq!(ktr.value, 0);
    }

    #[test]
    fn test_kalman_trend_regime_warmup() {
        let mut ktr = KalmanTrendRegime::new(1.0, 0.1, 1.0, 20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ktr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ktr.is_ready());
    }

    #[test]
    fn test_kalman_trend_regime_values_range() {
        let mut ktr = KalmanTrendRegime::new(1.0, 0.1, 1.0, 20);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ktr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= -1 && value <= 1);
        }
    }

    #[test]
    fn test_kalman_trend_regime_reset() {
        let mut ktr = KalmanTrendRegime::new(1.0, 0.1, 1.0, 20);
        for i in 0..30 {
            ktr.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ktr.reset();
        assert!(!ktr.is_ready());
        assert_eq!(ktr.value, 0);
    }
}
