// Engle–Granger ADF Proxy: OLS residuals of close ~ SMA(ma_period), then AR(1) t-stat on residuals

use crate::bar_indicators::average::sma::Sma;

#[derive(Clone)]
pub struct EngleGrangerAdfProxy {
    window: usize,
    sma: Sma,
    residuals: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    pub phi: f64,
    pub t_stat: f64,
}

impl EngleGrangerAdfProxy {
    pub fn new(window: usize, ma_period: usize) -> Self {
        let w = window.max(32);
        Self {
            window: w,
            sma: Sma::new(ma_period.max(5)),
            residuals: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            phi: 0.0,
            t_stat: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.sma.reset();
        self.residuals.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.last_close = None;
        self.phi = 0.0;
        self.t_stat = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> (f64, f64) {
        // OLS against SMA is effectively residual = close - SMA(close)
        let m = self.sma.update_bar(open, high, low, close, volume);
        let resid = close - m;
        self.residuals[self.idx] = resid;
        self.idx = (self.idx + 1) % self.window;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            self.update_stats();
        }
        (self.phi, self.t_stat)
    }

    fn update_stats(&mut self) {
        let n = self.window;
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut sxx = 0.0;
        let mut sxy = 0.0;
        let mut count = 0.0;
        for i in 1..n {
            let y = self.residuals[(self.idx + i) % n];
            let x = self.residuals[(self.idx + i - 1) % n];
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
        let mut se_sum = 0.0;
        for i in 1..n {
            let y = self.residuals[(self.idx + i) % n];
            let x = self.residuals[(self.idx + i - 1) % n];
            let e = y - self.phi * x;
            se_sum += e * e;
        }
        let var = se_sum.max(1e-12) / (count - 1.0).max(1.0);
        let sxx_adj = (sxx - sx * sx / count).max(1e-12);
        let se_phi = (var / sxx_adj).sqrt();
        self.t_stat = if se_phi > 0.0 {
            (self.phi - 1.0) / se_phi
        } else {
            0.0
        };
    }

    pub fn value(&self) -> crate::bar_indicators::indicator_value::IndicatorValue {
        crate::bar_indicators::indicator_value::IndicatorValue::Double(self.phi, self.t_stat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engle_granger_adf_proxy_creation() {
        let egap = EngleGrangerAdfProxy::new(50, 20);
        assert!(!egap.is_ready());
        assert_eq!(egap.phi, 0.0);
        assert_eq!(egap.t_stat, 0.0);
    }

    #[test]
    fn test_engle_granger_adf_proxy_warmup() {
        let mut egap = EngleGrangerAdfProxy::new(50, 20);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            egap.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(egap.is_ready());
    }

    #[test]
    fn test_engle_granger_adf_proxy_values() {
        let mut egap = EngleGrangerAdfProxy::new(50, 20);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (phi, t_stat) = egap.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(phi.is_finite(), "Phi should be finite");
            assert!(t_stat.is_finite(), "T-stat should be finite");
        }
    }

    #[test]
    fn test_engle_granger_adf_proxy_reset() {
        let mut egap = EngleGrangerAdfProxy::new(50, 20);
        for i in 0..60 {
            egap.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        egap.reset();
        assert!(!egap.is_ready());
        assert_eq!(egap.phi, 0.0);
        assert_eq!(egap.t_stat, 0.0);
    }
}
