// Realized Volatility Z-Score: zscore of rolling close-to-close volatility

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct RealizedVolZscore {
    // for vol estimation over returns
    vol_period: usize,
    r_buf: Vec<f64>,
    r_idx: usize,
    r_filled: bool,
    sum_r: f64,
    sum_r2: f64,
    prev_close: f64,
    curr_vol: f64,

    // for zscore of vol values
    zs_window: usize,
    v_buf: Vec<f64>,
    v_idx: usize,
    v_filled: bool,
    sum_v: f64,
    sum_v2: f64,
    zscore: f64,
}

impl RealizedVolZscore {
    pub fn new(vol_period: usize, zscore_window: usize) -> Self {
        let vp = vol_period.max(2);
        let zw = zscore_window.max(2);
        Self {
            vol_period: vp,
            r_buf: vec![0.0; vp],
            r_idx: 0,
            r_filled: false,
            sum_r: 0.0,
            sum_r2: 0.0,
            prev_close: 0.0,
            curr_vol: 0.0,
            zs_window: zw,
            v_buf: vec![0.0; zw],
            v_idx: 0,
            v_filled: false,
            sum_v: 0.0,
            sum_v2: 0.0,
            zscore: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.r_buf.fill(0.0);
        self.r_idx = 0;
        self.r_filled = false;
        self.sum_r = 0.0;
        self.sum_r2 = 0.0;
        self.prev_close = 0.0;
        self.curr_vol = 0.0;
        self.v_buf.fill(0.0);
        self.v_idx = 0;
        self.v_filled = false;
        self.sum_v = 0.0;
        self.sum_v2 = 0.0;
        self.zscore = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.v_filled
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> (f64, f64) {
        // update return
        if self.prev_close <= 0.0 {
            self.prev_close = close.max(1e-12);
            return (self.curr_vol, self.zscore);
        }
        let r = (close / self.prev_close).ln();
        self.prev_close = close.max(1e-12);

        // rolling stats for returns
        let old_r = self.r_buf[self.r_idx];
        self.r_buf[self.r_idx] = r;
        self.r_idx = (self.r_idx + 1) % self.vol_period;
        if self.r_idx == 0 {
            self.r_filled = true;
        }
        self.sum_r += r - old_r;
        self.sum_r2 += r * r - old_r * old_r;

        // compute current vol
        let n = if self.r_filled {
            self.vol_period as f64
        } else {
            self.r_idx as f64
        };
        if n >= 2.0 {
            let mean_r = self.sum_r / n;
            let var_r = (self.sum_r2 / n) - mean_r * mean_r;
            self.curr_vol = if var_r > 0.0 { var_r.sqrt() } else { 0.0 };
        } else {
            self.curr_vol = 0.0;
        }

        // feed vol into zscore ring
        let old_v = self.v_buf[self.v_idx];
        self.v_buf[self.v_idx] = self.curr_vol;
        self.v_idx = (self.v_idx + 1) % self.zs_window;
        if self.v_idx == 0 {
            self.v_filled = true;
        }
        self.sum_v += self.curr_vol - old_v;
        self.sum_v2 += self.curr_vol * self.curr_vol - old_v * old_v;
        let m = if self.v_filled {
            self.zs_window as f64
        } else {
            self.v_idx as f64
        };
        if m >= 2.0 {
            let mean_v = self.sum_v / m;
            let var_v = (self.sum_v2 / m) - mean_v * mean_v;
            let std_v = if var_v > 0.0 { var_v.sqrt() } else { 0.0 };
            self.zscore = if std_v > 1e-12 {
                (self.curr_vol - mean_v) / std_v
            } else {
                0.0
            };
        } else {
            self.zscore = 0.0;
        }

        (self.curr_vol, self.zscore)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.zscore)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_realized_vol_zscore_creation() {
        let rvz = RealizedVolZscore::new(20, 50);
        assert!(!rvz.is_ready());
        assert_eq!(rvz.value().main(), 0.0);
    }

    #[test]
    fn test_realized_vol_zscore_warmup() {
        let mut rvz = RealizedVolZscore::new(20, 50);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rvz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rvz.is_ready());
    }

    #[test]
    fn test_realized_vol_zscore_values() {
        let mut rvz = RealizedVolZscore::new(20, 50);
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (vol, zscore) = rvz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(vol >= 0.0, "Volatility should be non-negative");
            assert!(zscore.is_finite(), "Z-score should be finite");
        }
    }

    #[test]
    fn test_realized_vol_zscore_reset() {
        let mut rvz = RealizedVolZscore::new(20, 50);
        for i in 0..70 {
            rvz.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        rvz.reset();
        assert!(!rvz.is_ready());
        assert_eq!(rvz.value().main(), 0.0);
    }
}
