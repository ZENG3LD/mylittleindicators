// Residual Stationarity Proxy: run regression close ~ SMA(window) and compute variance ratio of residuals

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct ResidualStationarity {
    window: usize,
    closes: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl ResidualStationarity {
    pub fn new(window: usize) -> Self {
        let w = window.max(20);
        Self {
            window: w,
            closes: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.closes.fill(0.0);
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let n = self.window;
        self.closes[self.idx] = c;
        self.idx = (self.idx + 1) % n;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            self.value = self.compute_ratio();
        }
        self.value
    }

    fn compute_ratio(&self) -> f64 {
        let n = self.window;
        let mut mean = 0.0;
        for i in 0..n {
            mean += self.closes[i];
        }
        mean /= n as f64;
        // simple SMA as regressor; residuals = close - SMA
        let mut sma = vec![0.0; n];
        let mut sum = 0.0;
        for (i, slot) in sma.iter_mut().enumerate() {
            sum += self.closes[i];
            *slot = sum / ((i + 1) as f64);
        }
        let res: Vec<f64> = self.closes[..n].iter().zip(sma.iter()).map(|(&c, &s)| c - s).collect();
        // variance ratio: Var(res) / Var(close)
        let mut var_res = 0.0;
        let mut var_close = 0.0;
        for (&r, &c) in res.iter().zip(self.closes[..n].iter()) {
            var_res += r * r;
            var_close += (c - mean) * (c - mean);
        }
        if var_close > 0.0 {
            (var_res / n as f64) / (var_close / n as f64)
        } else {
            0.0
        }
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_residual_stationarity_creation() {
        let rs = ResidualStationarity::new(50);
        assert!(!rs.is_ready());
        assert_eq!(rs.value, 0.0);
    }

    #[test]
    fn test_residual_stationarity_warmup() {
        let mut rs = ResidualStationarity::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rs.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rs.is_ready());
    }

    #[test]
    fn test_residual_stationarity_non_negative() {
        let mut rs = ResidualStationarity::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = rs.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "Variance ratio should be non-negative");
        }
    }

    #[test]
    fn test_residual_stationarity_reset() {
        let mut rs = ResidualStationarity::new(50);
        for i in 0..60 {
            rs.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        rs.reset();
        assert!(!rs.is_ready());
        assert_eq!(rs.value, 0.0);
    }
}
