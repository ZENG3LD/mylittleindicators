// Return Autocorrelation at lag L over rolling window N

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct Autocorr {
    lag: usize,
    window: usize,
    // store last window+lag closes to compute returns
    closes: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl Autocorr {
    pub fn new(lag: usize, window: usize) -> Self {
        let lag = lag.max(1);
        let window = window.max(2);
        Self {
            lag,
            window,
            closes: vec![0.0; window + lag + 1],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.closes.fill(0.0);
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
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> f64 {
        // push close
        self.closes[self.idx] = close;
        self.idx = (self.idx + 1) % self.closes.len();
        if self.idx == 0 {
            self.filled = true;
        }

        // compute autocorr over window on log returns with lag
        if !self.filled {
            return self.value;
        }
        let len = self.closes.len();
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_x2 = 0.0;
        let mut sum_y2 = 0.0;
        let mut sum_xy = 0.0;
        let mut n = 0.0;
        // walk last `window` pairs
        for k in 0..self.window {
            // indices from newest backwards
            let i_curr = (self.idx + len + len - 1 - k) % len;
            let i_prev = (i_curr + len - 1) % len;
            let i_lag_curr = (i_curr + len - self.lag) % len;
            let i_lag_prev = (i_lag_curr + len - 1) % len;
            let c1 = self.closes[i_prev].max(1e-12);
            let c2 = self.closes[i_curr].max(1e-12);
            let c3 = self.closes[i_lag_prev].max(1e-12);
            let c4 = self.closes[i_lag_curr].max(1e-12);
            let r_t = (c2 / c1).ln();
            let r_tlag = (c4 / c3).ln();
            sum_x += r_t;
            sum_y += r_tlag;
            sum_x2 += r_t * r_t;
            sum_y2 += r_tlag * r_tlag;
            sum_xy += r_t * r_tlag;
            n += 1.0;
        }
        if n >= 2.0 {
            let mx = sum_x / n;
            let my = sum_y / n;
            let cov = (sum_xy / n) - mx * my;
            let vx = (sum_x2 / n) - mx * mx;
            let vy = (sum_y2 / n) - my * my;
            let denom = (vx.max(0.0) * vy.max(0.0)).sqrt();
            self.value = if denom > 1e-12 { cov / denom } else { 0.0 };
        } else {
            self.value = 0.0;
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn lag(&self) -> usize {
        self.lag
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autocorr_creation() {
        let ac = Autocorr::new(5, 20);
        assert!(!ac.is_ready());
        assert_eq!(ac.value().main(), 0.0);
        assert_eq!(ac.lag(), 5);
        assert_eq!(ac.window(), 20);
    }

    #[test]
    fn test_autocorr_range() {
        let mut ac = Autocorr::new(5, 20);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let value = ac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if ac.is_ready() {
                assert!(value >= -1.0 && value <= 1.0, "Autocorr should be in [-1, 1], got {}", value);
            }
        }
        assert!(ac.is_ready());
    }

    #[test]
    fn test_autocorr_reset() {
        let mut ac = Autocorr::new(5, 20);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            ac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ac.is_ready());
        ac.reset();
        assert!(!ac.is_ready());
        assert_eq!(ac.value().main(), 0.0);
    }
}
