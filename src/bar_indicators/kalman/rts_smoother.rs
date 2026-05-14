// Rauch–Tung–Striebel (RTS) Smoother — streaming forward-filter proxy.
//
// True RTS requires a full backward pass over a stored sequence: it first runs the
// Kalman filter forward storing all (x̂, P) pairs, then sweeps backward computing
// smoothed estimates. This requires O(N) memory and is inherently non-causal (batch).
//
// Streaming constraint: in a bar-by-bar indicator there is no backward pass.
// This implementation provides the causal Kalman forward filter output, which is
// the best achievable real-time approximation of RTS. When the full sequence is
// available offline, apply true RTS backward sweep on top of the stored states.

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
