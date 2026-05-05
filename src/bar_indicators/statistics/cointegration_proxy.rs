// Cointegration Proxy: residual stationarity via AR(1) t-stat on (close - SMA(window)) residuals

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::average::sma::Sma;

#[derive(Clone)]
pub struct CointegrationProxy {
    window: usize,
    sma: Sma,
    residuals: Vec<f64>,
    idx: usize,
    filled: bool,
    pub phi: f64,
    pub t_stat: f64,
}

impl CointegrationProxy {
    pub fn new(window: usize) -> Self {
        let w = window.max(20);
        Self {
            window: w,
            sma: Sma::new(w),
            residuals: vec![0.0; w],
            idx: 0,
            filled: false,
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

    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> (f64, f64) {
        let m = self.sma.update_bar(open, high, low, close, volume);
        let r = close - m;
        self.residuals[self.idx] = r;
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
        // residual variance and std error of phi
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cointegration_proxy_creation() {
        let cp = CointegrationProxy::new(50);
        assert!(!cp.is_ready());
        assert_eq!(cp.phi, 0.0);
        assert_eq!(cp.t_stat, 0.0);
    }

    #[test]
    fn test_cointegration_proxy_warmup() {
        let mut cp = CointegrationProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            cp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(cp.is_ready());
    }

    #[test]
    fn test_cointegration_proxy_values() {
        let mut cp = CointegrationProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (phi, t_stat) = cp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(phi.is_finite(), "Phi should be finite");
            assert!(t_stat.is_finite(), "T-stat should be finite");
        }
    }

    #[test]
    fn test_cointegration_proxy_reset() {
        let mut cp = CointegrationProxy::new(50);
        for i in 0..60 {
            cp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        cp.reset();
        assert!(!cp.is_ready());
        assert_eq!(cp.phi, 0.0);
        assert_eq!(cp.t_stat, 0.0);
    }
}
