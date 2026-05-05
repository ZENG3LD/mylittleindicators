// Simplified CUSUM break detector: accumulate returns deviations; emit score of break intensity

#[derive(Clone)]
pub struct CusumBreakDetector {
    threshold: f64,
    kappa: f64,
    pos: f64,
    neg: f64,
    last_close: Option<f64>,
    pub value: f64,
}

impl CusumBreakDetector {
    pub fn new(threshold: f64, kappa: f64) -> Self {
        Self {
            threshold,
            kappa,
            pos: 0.0,
            neg: 0.0,
            last_close: None,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.pos = 0.0;
        self.neg = 0.0;
        self.last_close = None;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.last_close.is_some()
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        if let Some(prev) = self.last_close {
            let r = (c / prev).ln();
            self.pos = (self.kappa * (self.pos + r)).max(0.0);
            self.neg = (self.kappa * (self.neg - r)).max(0.0);
            let hit_pos = (self.pos - self.threshold).max(0.0);
            let hit_neg = (self.neg - self.threshold).max(0.0);
            self.value = hit_pos.max(hit_neg);
            if hit_pos > 0.0 {
                self.pos = 0.0;
            }
            if hit_neg > 0.0 {
                self.neg = 0.0;
            }
        }
        self.last_close = Some(c);
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cusum_break_detector_creation() {
        let cbd = CusumBreakDetector::new(0.05, 0.95);
        assert!(!cbd.is_ready());
        assert_eq!(cbd.value, 0.0);
    }

    #[test]
    fn test_cusum_break_detector_warmup() {
        let mut cbd = CusumBreakDetector::new(0.05, 0.95);
        cbd.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0);
        assert!(cbd.is_ready());
    }

    #[test]
    fn test_cusum_break_detector_values() {
        let mut cbd = CusumBreakDetector::new(0.05, 0.95);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = cbd.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "CUSUM should be non-negative");
        }
    }

    #[test]
    fn test_cusum_break_detector_reset() {
        let mut cbd = CusumBreakDetector::new(0.05, 0.95);
        for i in 0..10 {
            cbd.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        cbd.reset();
        assert!(!cbd.is_ready());
        assert_eq!(cbd.value, 0.0);
    }
}
