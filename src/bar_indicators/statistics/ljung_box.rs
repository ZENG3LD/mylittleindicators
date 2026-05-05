// Rolling Ljung-Box Q statistic over k lags on returns; outputs Q (proxy for p-value usage upstream)

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct LjungBox {
    window: usize,
    lags: usize,
    vals: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    q_stat: f64,
}

impl LjungBox {
    pub fn new(window: usize, lags: usize) -> Self {
        let w = window.max(20);
        let k = lags.max(1).min(w / 2);
        Self {
            window: w,
            lags: k,
            vals: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            q_stat: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.vals.fill(0.0);
        self.last_close = None;
        self.q_stat = 0.0;
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
            self.vals[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if self.idx == 0 {
                self.filled = true;
            }
            if self.filled {
                self.q_stat = self.compute_q();
            }
        }
        self.last_close = Some(close);
        self.q_stat
    }

    fn compute_q(&self) -> f64 {
        let n = self.window as f64;
        // mean-adjusted
        let mut mean = 0.0;
        for v in &self.vals {
            mean += *v;
        }
        mean /= self.window as f64;
        let x: Vec<f64> = self.vals.iter().map(|&v| v - mean).collect();
        let mut var = 0.0;
        for v in &x {
            var += *v * *v;
        }
        var /= n;
        if var <= 1e-12 {
            return 0.0;
        }
        let mut q = 0.0;
        for h in 1..=self.lags {
            // autocorr at lag h
            let mut num = 0.0;
            for t in h..self.window {
                num += x[t] * x[t - h];
            }
            let rho_h = num / (n * var);
            q += rho_h * rho_h / (n - h as f64);
        }
        q * n * (n + 2.0)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.q_stat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ljung_box_creation() {
        let lb = LjungBox::new(50, 10);
        assert!(!lb.is_ready());
        assert_eq!(lb.value().main(), 0.0);
    }

    #[test]
    fn test_ljung_box_warmup() {
        let mut lb = LjungBox::new(50, 10);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            lb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(lb.is_ready());
    }

    #[test]
    fn test_ljung_box_non_negative() {
        let mut lb = LjungBox::new(50, 10);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = lb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "Q-stat should be non-negative");
        }
    }

    #[test]
    fn test_ljung_box_reset() {
        let mut lb = LjungBox::new(50, 10);
        for i in 0..60 {
            lb.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        lb.reset();
        assert!(!lb.is_ready());
        assert_eq!(lb.value().main(), 0.0);
    }
}
