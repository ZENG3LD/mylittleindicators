// Realized Quarticity - estimator for variance of volatility (fourth power of returns)

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct RealizedQuarticity {
    window: usize,
    r4_buffer: Vec<f64>,
    idx: usize,
    filled: bool,
    prev_close: f64,
    value: f64,
}

impl RealizedQuarticity {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            r4_buffer: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            prev_close: 0.0,
            value: 0.0,
        }
    }
    pub fn reset(&mut self) {
        self.r4_buffer.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.prev_close = 0.0;
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
        if self.prev_close <= 0.0 {
            self.prev_close = c.max(1e-12);
            return self.value;
        }
        let r = (c / self.prev_close).ln();
        self.prev_close = c.max(1e-12);
        let r4 = r * r * r * r;
        let _old = self.r4_buffer[self.idx];
        self.r4_buffer[self.idx] = r4;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        let n = if self.filled {
            self.window as f64
        } else {
            self.idx as f64
        };
        if n > 0.0 {
            // Масштабируем для лучшей читаемости (четвертая степень очень мала)
            self.value = (self.r4_buffer.iter().take(n as usize).sum::<f64>() / n) * 1000000.0;
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_realized_quarticity_creation() {
        let rq = RealizedQuarticity::new(20);
        assert!(!rq.is_ready());
        assert_eq!(rq.value().main(), 0.0);
    }

    #[test]
    fn test_realized_quarticity_warmup() {
        let mut rq = RealizedQuarticity::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rq.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rq.is_ready());
    }

    #[test]
    fn test_realized_quarticity_non_negative() {
        let mut rq = RealizedQuarticity::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = rq.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "Realized quarticity should be non-negative");
        }
    }

    #[test]
    fn test_realized_quarticity_reset() {
        let mut rq = RealizedQuarticity::new(20);
        for i in 0..25 {
            rq.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        rq.reset();
        assert!(!rq.is_ready());
        assert_eq!(rq.value().main(), 0.0);
    }
}
