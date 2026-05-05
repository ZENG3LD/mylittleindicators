// Rolling window (ring buffer) version of Efficiency Ratio
// Fast, memory efficient, classic ER

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct EfficiencyRatioRingWindow {
    pub period: usize,
    buf: Vec<f64>,
    deltas: Vec<f64>,
    head: usize,
    len: usize,
    value: f64,
    initialized: bool,
}

impl EfficiencyRatioRingWindow {
    pub fn new(period: usize) -> Self {
        assert!(period > 1);
        Self {
            period,
            buf: vec![0.0; period],
            deltas: vec![0.0; period - 1],
            head: 0,
            len: 0,
            value: 0.0,
            initialized: false,
        }
    }

    pub fn update_raw(&mut self, value: f64) -> f64 {
        let idx = (self.head + self.len) % self.period;
        if self.len < self.period {
            self.buf[idx] = value;
            self.len += 1;
            if self.len == 1 {
                self.value = 0.0;
                self.initialized = false;
                return self.value;
            }
        } else {
            self.buf[self.head] = value;
            self.head = (self.head + 1) % self.period;
        }
        // Compute deltas for the current window
        for i in 0..(self.len.min(self.period) - 1) {
            let idx0 = (self.head + i) % self.period;
            let idx1 = (self.head + i + 1) % self.period;
            self.deltas[i] = (self.buf[idx1] - self.buf[idx0]).abs();
        }
        self.initialized = self.len >= self.period;
        let net_diff = (self.buf[(self.head + self.len - 1) % self.period] - self.buf[self.head]).abs();
        let sum_deltas: f64 = self.deltas[..self.len.min(self.period) - 1].iter().sum();
        self.value = if sum_deltas == 0.0 { 0.0 } else { net_diff / sum_deltas };
        self.value
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    pub fn get_buf(&self) -> Vec<f64> {
        let mut out = Vec::with_capacity(self.len);
        for i in 0..self.len {
            let idx = (self.head + i) % self.period;
            out.push(self.buf[idx]);
        }
        out
    }
    pub fn get_deltas(&self) -> Vec<f64> {
        let mut out = Vec::with_capacity(self.len.saturating_sub(1));
        for i in 0..self.len.saturating_sub(1) {
            out.push(self.deltas[i]);
        }
        out
    }
    pub fn reset(&mut self) {
        for v in &mut self.buf { *v = 0.0; }
        for d in &mut self.deltas { *d = 0.0; }
        self.head = 0;
        self.len = 0;
        self.value = 0.0;
        self.initialized = false;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efficiency_ratio_ring_creation() {
        let ind = EfficiencyRatioRingWindow::new(10);
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_efficiency_ratio_ring_warmup() {
        let mut ind = EfficiencyRatioRingWindow::new(10);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            ind.update_raw(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_efficiency_ratio_ring_values() {
        let mut ind = EfficiencyRatioRingWindow::new(10);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            ind.update_raw(price);
        }
        assert!(ind.value().main().is_finite());
        assert!(ind.value().main() >= 0.0);
    }

    #[test]
    fn test_efficiency_ratio_ring_reset() {
        let mut ind = EfficiencyRatioRingWindow::new(10);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            ind.update_raw(price);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
}






















