// ADF proxy: rolling AR(1) phi and t-like statistic on close returns

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct AdfProxy {
    window: usize,
    vals: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    pub phi: f64,
    pub t_stat: f64,
}

impl AdfProxy {
    pub fn new(window: usize) -> Self {
        let w = window.max(20);
        Self {
            window: w,
            vals: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            phi: 0.0,
            t_stat: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.vals.fill(0.0);
        self.last_close = None;
        self.phi = 0.0;
        self.t_stat = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Returns t-statistic as main value
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.t_stat)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> (f64, f64) {
        if let Some(prev) = self.last_close {
            let r = (c / prev).ln();
            self.vals[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if !self.filled && self.idx == 0 {
                self.filled = true;
            }
        }
        self.last_close = Some(c);
        if self.filled {
            let n = self.window;
            let mut sx = 0.0;
            let mut sy = 0.0;
            let mut sxx = 0.0;
            let mut sxy = 0.0;
            let mut se = 0.0;
            let mut count = 0.0;
            for i in 1..n {
                let y = self.vals[(self.idx + i) % n];
                let x = self.vals[(self.idx + i - 1) % n];
                sx += x;
                sy += y;
                sxx += x * x;
                sxy += x * y;
                count += 1.0;
            }
            let denom = count * sxx - sx * sx;
            self.phi = if denom.abs() > 1e-12 {
                (count * sxy - sx * sy) / denom
            } else {
                0.0
            };
            // residual variance
            for i in 1..n {
                let y = self.vals[(self.idx + i) % n];
                let x = self.vals[(self.idx + i - 1) % n];
                let e = y - self.phi * x;
                se += e * e;
            }
            let var = se.max(1e-12) / (count - 1.0).max(1.0);
            let se_phi = (var / (sxx - sx * sx / count).max(1e-12)).sqrt();
            self.t_stat = if se_phi > 0.0 {
                (self.phi - 1.0) / se_phi
            } else {
                0.0
            };
        }
        (self.phi, self.t_stat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adf_proxy_creation() {
        let adf = AdfProxy::new(50);
        assert!(!adf.is_ready());
        assert_eq!(adf.phi, 0.0);
        assert_eq!(adf.t_stat, 0.0);
    }

    #[test]
    fn test_adf_proxy_warmup() {
        let mut adf = AdfProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            adf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(adf.is_ready());
    }

    #[test]
    fn test_adf_proxy_values() {
        let mut adf = AdfProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (phi, t_stat) = adf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(phi.is_finite(), "Phi should be finite");
            assert!(t_stat.is_finite(), "T-stat should be finite");
        }
    }

    #[test]
    fn test_adf_proxy_reset() {
        let mut adf = AdfProxy::new(50);
        for i in 0..60 {
            adf.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        adf.reset();
        assert!(!adf.is_ready());
        assert_eq!(adf.phi, 0.0);
        assert_eq!(adf.t_stat, 0.0);
    }
}
