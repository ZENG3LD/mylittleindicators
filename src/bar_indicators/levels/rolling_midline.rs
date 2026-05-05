// Rolling Midline: average of High/Low over window

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct RollingMidline {
    window: usize,
    sum: f64,
    buffer: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl RollingMidline {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            sum: 0.0,
            buffer: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.sum = 0.0;
        self.buffer.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        high: f64,
        low: f64,
        _close: f64,
        _volume: f64,
    ) -> f64 {
        let mid = 0.5 * (high + low);
        self.sum += mid - self.buffer[self.idx];
        self.buffer[self.idx] = mid;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        let denom = if self.filled {
            self.window as f64
        } else {
            self.idx as f64
        };
        self.value = if denom > 0.0 { self.sum / denom } else { 0.0 };
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_midline_creation() {
        let rm = RollingMidline::new(20);
        assert!(!rm.is_ready());
        assert_eq!(rm.value().main(), 0.0);
    }

    #[test]
    fn test_rolling_midline_warmup() {
        let mut rm = RollingMidline::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rm.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rm.is_ready());
    }

    #[test]
    fn test_rolling_midline_positive() {
        let mut rm = RollingMidline::new(20);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let value = rm.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value > 0.0, "Midline should be positive");
        }
    }

    #[test]
    fn test_rolling_midline_reset() {
        let mut rm = RollingMidline::new(20);
        for i in 0..25 {
            rm.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0, 1000.0);
        }
        rm.reset();
        assert!(!rm.is_ready());
        assert_eq!(rm.value().main(), 0.0);
    }
}
