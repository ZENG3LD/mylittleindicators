// Engle–Granger Trend Proxy: OLS with trend term (time index) before residual ADF proxy

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct EngleGrangerTrendProxy {
    window: usize,
    // rolling buffers for close and time
    closes: Vec<f64>,
    times: Vec<f64>,
    idx: usize,
    filled: bool,
    pub t_stat: f64,
}

impl EngleGrangerTrendProxy {
    pub fn new(window: usize) -> Self {
        let w = window.max(32);
        Self {
            window: w,
            closes: vec![0.0; w],
            times: (0..w).map(|i| i as f64).collect(),
            idx: 0,
            filled: false,
            t_stat: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.closes.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.t_stat = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.t_stat)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        self.closes[self.idx] = c;
        self.idx = (self.idx + 1) % self.window;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            self.update_stats();
        }
        self.t_stat
    }

    fn update_stats(&mut self) {
        let n = self.window;
        let mut _sx = 0.0;
        let mut st = 0.0;
        let mut _sxx = 0.0;
        let mut stt = 0.0;
        let mut _sxt = 0.0;
        let mut sy = 0.0;
        let mut syt = 0.0;
        let mut _sxy = 0.0;
        let mut count = 0.0;
        // multiple regression y ~ a + b*x + c*t, here x is SMA proxy via de-meaning (omit for simplicity) -> use (t) trend only with intercept
        for i in 0..n {
            let y = self.closes[(self.idx + i) % n];
            let t = self.times[i];
            _sx += 1.0;
            st += t;
            _sxx += 1.0;
            stt += t * t;
            _sxt += t;
            sy += y;
            syt += y * t;
            _sxy += y;
            count += 1.0;
        }
        // Solve for c (trend coef) in normal equations [[n, sum t],[sum t, sum t2]] * [a,c] = [sum y, sum y t]
        let denom = count * stt - st * st;
        let c = if denom.abs() > 1e-12 {
            (count * syt - st * sy) / denom
        } else {
            0.0
        };
        // residuals = y - (a + c*t); compute AR(1) t-like stat on residuals
        let a = if count > 0.0 {
            (sy - c * st) / count
        } else {
            0.0
        };
        let mut resid = vec![0.0; n];
        for (i, (slot, &t)) in resid.iter_mut().zip(self.times[..n].iter()).enumerate() {
            let y = self.closes[(self.idx + i) % n];
            *slot = y - (a + c * t);
        }
        let mut rx = 0.0;
        let mut ry = 0.0;
        let mut rxx = 0.0;
        let mut rxy = 0.0;
        let mut rc = 0.0;
        for i in 1..n {
            let y = resid[i];
            let x = resid[i - 1];
            rx += x;
            ry += y;
            rxx += x * x;
            rxy += x * y;
            rc += 1.0;
        }
        let d = rc * rxx - rx * rx;
        let phi = if d.abs() > 1e-12 {
            (rc * rxy - rx * ry) / d
        } else {
            0.0
        };
        let mut se = 0.0;
        for i in 1..n {
            let y = resid[i];
            let x = resid[i - 1];
            let e = y - phi * x;
            se += e * e;
        }
        let var = se.max(1e-12) / (rc - 1.0).max(1.0);
        let se_phi = (var / (rxx - rx * rx / rc).max(1e-12)).sqrt();
        self.t_stat = if se_phi > 0.0 {
            (phi - 1.0) / se_phi
        } else {
            0.0
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engle_granger_trend_proxy_creation() {
        let egtp = EngleGrangerTrendProxy::new(50);
        assert!(!egtp.is_ready());
        assert_eq!(egtp.t_stat, 0.0);
    }

    #[test]
    fn test_engle_granger_trend_proxy_warmup() {
        let mut egtp = EngleGrangerTrendProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            egtp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(egtp.is_ready());
    }

    #[test]
    fn test_engle_granger_trend_proxy_values() {
        let mut egtp = EngleGrangerTrendProxy::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = egtp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "T-stat should be finite");
        }
    }

    #[test]
    fn test_engle_granger_trend_proxy_reset() {
        let mut egtp = EngleGrangerTrendProxy::new(50);
        for i in 0..60 {
            egtp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        egtp.reset();
        assert!(!egtp.is_ready());
        assert_eq!(egtp.t_stat, 0.0);
    }
}
