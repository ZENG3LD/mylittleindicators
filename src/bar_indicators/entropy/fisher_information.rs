// Rolling Fisher Information proxy for Gaussian mean: I = n / var(returns)

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct RollingFisherInformation {
    window: usize,
    rets: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    value: f64,
}

impl RollingFisherInformation {
    pub fn new(window: usize) -> Self {
        let w = window.max(10);
        Self {
            window: w,
            rets: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.last_close = None;
        self.rets.fill(0.0);
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> f64 {
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            self.rets[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if self.idx == 0 {
                self.filled = true;
            }
            if self.filled {
                let n = self.window as f64;
                let mut mean = 0.0;
                for i in 0..self.window {
                    mean += self.rets[i];
                }
                mean /= n;
                let mut var = 0.0;
                for i in 0..self.window {
                    let d = self.rets[i] - mean;
                    var += d * d;
                }
                var /= n;
                self.value = if var <= 1e-12 { 0.0 } else { n / var };
            }
        }
        self.last_close = Some(close);
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
    fn test_rolling_fisher_information_creation() {
        let fi = RollingFisherInformation::new(20);
        assert!(!fi.is_ready());
        assert_eq!(fi.value().main(), 0.0);
    }

    #[test]
    fn test_rolling_fisher_information_warmup() {
        let mut fi = RollingFisherInformation::new(15);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            fi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(fi.is_ready());
    }

    #[test]
    fn test_rolling_fisher_information_values_finite() {
        let mut fi = RollingFisherInformation::new(15);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = fi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_rolling_fisher_information_reset() {
        let mut fi = RollingFisherInformation::new(15);
        for i in 0..25 {
            fi.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        fi.reset();
        assert!(!fi.is_ready());
        assert_eq!(fi.value().main(), 0.0);
    }
}
