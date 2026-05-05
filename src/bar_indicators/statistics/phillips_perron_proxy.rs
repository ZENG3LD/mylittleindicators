// Phillips-Perron unit root test proxy (rolling ADF-like z-score)

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct PhillipsPerronProxy {
    window: usize,
    mean: f64,
    m2: f64,
    count: usize,
    value: f64,
}

impl PhillipsPerronProxy {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(10),
            mean: 0.0,
            m2: 0.0,
            count: 0,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.mean = 0.0;
        self.m2 = 0.0;
        self.count = 0;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.window
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        // Welford online variance for returns
        self.count += 1;
        let x = c;
        let delta = x - self.mean;
        self.mean += delta / (self.count as f64);
        self.m2 += delta * (x - self.mean);
        if self.is_ready() {
            let var = self.m2 / ((self.count as f64) - 1.0);
            self.value = if var > 0.0 {
                (x - self.mean) / var.sqrt()
            } else {
                0.0
            };
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phillips_perron_proxy_creation() {
        let pp = PhillipsPerronProxy::new(50);
        assert!(!pp.is_ready());
        assert_eq!(pp.value().main(), 0.0);
    }

    #[test]
    fn test_phillips_perron_proxy_warmup() {
        let mut pp = PhillipsPerronProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pp.is_ready());
    }

    #[test]
    fn test_phillips_perron_proxy_values() {
        let mut pp = PhillipsPerronProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "PP should be finite");
        }
    }

    #[test]
    fn test_phillips_perron_proxy_reset() {
        let mut pp = PhillipsPerronProxy::new(50);
        for i in 0..60 {
            pp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        pp.reset();
        assert!(!pp.is_ready());
        assert_eq!(pp.value().main(), 0.0);
    }
}
