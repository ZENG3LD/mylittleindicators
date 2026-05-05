// Rauch–Tung–Striebel smoother proxy: simple backward smoothing over kalman output (placeholder)

use crate::bar_indicators::kalman::basic_kalman_filter::BasicKalmanFilter;
use crate::bar_indicators::indicator_value::IndicatorValue;


#[derive(Clone)]
pub struct RtsSmoother {
    kf: BasicKalmanFilter,
    last_value: f64,
}

impl Default for RtsSmoother {
    fn default() -> Self {
        Self::new()
    }
}

impl RtsSmoother {
    pub fn new() -> Self {
        Self {
            kf: BasicKalmanFilter::new(1.0, 1.0, 1.0),
            last_value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.kf.reset();
        self.last_value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.kf.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        // forward filter
        let x = self.kf.update(c).filtered_value;
        // no backward pass in streaming; return filtered value as proxy
        self.last_value = x;
        self.last_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rts_smoother_creation() {
        let rts = RtsSmoother::new();
        assert!(!rts.is_ready());
        assert_eq!(rts.value().main(), 0.0);
    }

    #[test]
    fn test_rts_smoother_warmup() {
        let mut rts = RtsSmoother::new();
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rts.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rts.is_ready());
    }

    #[test]
    fn test_rts_smoother_values_finite() {
        let mut rts = RtsSmoother::new();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = rts.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_rts_smoother_reset() {
        let mut rts = RtsSmoother::new();
        for i in 0..10 {
            rts.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        rts.reset();
        assert!(!rts.is_ready());
        assert_eq!(rts.value().main(), 0.0);
    }
}
