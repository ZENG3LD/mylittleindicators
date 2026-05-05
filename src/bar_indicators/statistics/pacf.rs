// Rolling Partial Autocorrelation Function (PACF) at given lag k via Levinson-Durbin on window

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct Pacf {
    window: usize,
    lag: usize,
    values: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    pacf_k: f64,
}

impl Pacf {
    pub fn new(window: usize, lag: usize) -> Self {
        let w = window.max(10);
        let k = lag.max(1).min(w - 2);
        Self {
            window: w,
            lag: k,
            values: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            pacf_k: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.last_close = None;
        self.values.fill(0.0);
        self.pacf_k = 0.0;
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
        // work on returns
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            self.values[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if self.idx == 0 {
                self.filled = true;
            }
            if self.filled {
                self.pacf_k = self.compute_pacf_k();
            }
        }
        self.last_close = Some(close);
        self.pacf_k
    }

    fn compute_pacf_k(&self) -> f64 {
        // Yule-Walker via Levinson-Durbin
        let n = self.window;
        let mean: f64 = self.values.iter().sum::<f64>() / n as f64;
        let x: Vec<f64> = self.values.iter().map(|&v| v - mean).collect();
        let k = self.lag;
        let mut autoc = vec![0.0; k + 1];
        for lag in 0..=k {
            let mut s = 0.0;
            for t in lag..n {
                s += x[t] * x[t - lag];
            }
            autoc[lag] = s / (n as f64);
        }
        if autoc[0].abs() < 1e-12 {
            return 0.0;
        }
        let mut phi = vec![0.0; k + 1];
        let mut v = autoc[0];
        for m in 1..=k {
            let mut sum = 0.0;
            for j in 1..m {
                sum += phi[j] * autoc[m - j];
            }
            let km = (autoc[m] - sum) / v.max(1e-12);
            let mut new_phi = phi.clone();
            new_phi[m] = km;
            for j in 1..m {
                new_phi[j] = phi[j] - km * phi[m - j];
            }
            phi = new_phi;
            v *= 1.0 - km * km;
        }
        phi[k].clamp(-1.0, 1.0)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.pacf_k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pacf_creation() {
        let pacf = Pacf::new(50, 5);
        assert!(!pacf.is_ready());
        assert_eq!(pacf.value().main(), 0.0);
    }

    #[test]
    fn test_pacf_warmup() {
        let mut pacf = Pacf::new(50, 5);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pacf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pacf.is_ready());
    }

    #[test]
    fn test_pacf_range() {
        let mut pacf = Pacf::new(50, 5);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pacf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= -1.0 && value <= 1.0, "PACF should be in [-1, 1]");
        }
    }

    #[test]
    fn test_pacf_reset() {
        let mut pacf = Pacf::new(50, 5);
        for i in 0..60 {
            pacf.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        pacf.reset();
        assert!(!pacf.is_ready());
        assert_eq!(pacf.value().main(), 0.0);
    }
}
