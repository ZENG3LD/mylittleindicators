// CUSUM filter on returns: emits event when cumulative sum crosses +/- threshold

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct CusumFilter {
    threshold: f64,
    last_close: Option<f64>,
    pos_sum: f64,
    neg_sum: f64,
    pub event: i8, // -1, 0, +1
}

impl CusumFilter {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold: threshold.abs().max(1e-12),
            last_close: None,
            pos_sum: 0.0,
            neg_sum: 0.0,
            event: 0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.last_close = None;
        self.pos_sum = 0.0;
        self.neg_sum = 0.0;
        self.event = 0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.last_close.is_some()
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> i8 {
        self.event = 0;
        if let Some(prev) = self.last_close {
            let r = (close / prev) - 1.0;
            self.pos_sum = (self.pos_sum + r).max(0.0);
            self.neg_sum = (self.neg_sum + r).min(0.0);
            if self.pos_sum > self.threshold {
                self.event = 1;
                self.pos_sum = 0.0;
                self.neg_sum = 0.0;
            }
            if self.neg_sum < -self.threshold {
                self.event = -1;
                self.pos_sum = 0.0;
                self.neg_sum = 0.0;
            }
        }
        self.last_close = Some(close);
        self.event
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.event)
    }

    pub fn threshold(&self) -> f64 {
        self.threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cusum_creation() {
        let cusum = CusumFilter::new(0.05);
        assert!(!cusum.is_ready());
        assert_eq!(cusum.event, 0);
        assert!((cusum.threshold() - 0.05).abs() < 1e-9);
    }

    #[test]
    fn test_cusum_positive_event() {
        let mut cusum = CusumFilter::new(0.05);
        // Feed rising prices to trigger positive event
        let mut price = 100.0;
        let mut triggered = false;
        for _ in 0..20 {
            price *= 1.02; // 2% increase each bar
            let event = cusum.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if event == 1 {
                triggered = true;
                break;
            }
        }
        assert!(triggered, "CUSUM should trigger positive event on rising prices");
    }

    #[test]
    fn test_cusum_negative_event() {
        let mut cusum = CusumFilter::new(0.05);
        let mut price = 100.0;
        let mut triggered = false;
        for _ in 0..20 {
            price *= 0.98; // 2% decrease each bar
            let event = cusum.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if event == -1 {
                triggered = true;
                break;
            }
        }
        assert!(triggered, "CUSUM should trigger negative event on falling prices");
    }

    #[test]
    fn test_cusum_reset() {
        let mut cusum = CusumFilter::new(0.05);
        cusum.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0);
        assert!(cusum.is_ready());
        cusum.reset();
        assert!(!cusum.is_ready());
        assert_eq!(cusum.event, 0);
    }
}
