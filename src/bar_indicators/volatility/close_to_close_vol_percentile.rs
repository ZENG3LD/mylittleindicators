// Close-to-Close Volatility Percentile over rolling window

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct CloseVolPercentile {
    #[allow(dead_code)]
    vol_window: usize,
    ret_prev_close: f64,
    // ring for recent vol values
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
    percentile: f64,
}

impl CloseVolPercentile {
    pub fn new(vol_window: usize, percentile_window: usize) -> Self {
        Self {
            vol_window: vol_window.max(2),
            ret_prev_close: 0.0,
            window: percentile_window.max(1),
            buf: vec![0.0; percentile_window.max(1)],
            idx: 0,
            filled: false,
            value: 0.0,
            percentile: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ret_prev_close = 0.0;
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
        self.percentile = 0.0;
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
    ) -> (f64, f64) {
        if self.ret_prev_close <= 0.0 {
            self.ret_prev_close = close.max(1e-12);
            return (self.value, self.percentile);
        }
        let r = (close / self.ret_prev_close).ln();
        self.ret_prev_close = close.max(1e-12);

        // EWMA-like vol proxy over vol_window using simple rolling RMS of returns
        self.value = r.abs(); // use absolute return as cheap proxy; median filtering can be added later

        let _old = self.buf[self.idx];
        self.buf[self.idx] = self.value;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut count_le = 0usize;
            for i in 0..len {
                if self.buf[i] <= self.value {
                    count_le += 1;
                }
            }
            self.percentile = count_le as f64 / len as f64;
        } else {
            self.percentile = 0.0;
        }
        (self.value, self.percentile)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.percentile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_close_vol_percentile_creation() {
        let cvp = CloseVolPercentile::new(20, 50);
        assert!(!cvp.is_ready());
        assert_eq!(cvp.value().main(), 0.0);
    }

    #[test]
    fn test_close_vol_percentile_warmup() {
        let mut cvp = CloseVolPercentile::new(20, 50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            cvp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(cvp.is_ready());
    }

    #[test]
    fn test_close_vol_percentile_range() {
        let mut cvp = CloseVolPercentile::new(20, 50);
        for i in 0..60 {
            let price = 100.0 + i as f64;
            let (_, pct) = cvp.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(pct >= 0.0 && pct <= 1.0, "Percentile should be in [0, 1]");
        }
    }

    #[test]
    fn test_close_vol_percentile_reset() {
        let mut cvp = CloseVolPercentile::new(20, 50);
        for i in 0..60 {
            cvp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        cvp.reset();
        assert!(!cvp.is_ready());
        assert_eq!(cvp.value().main(), 0.0);
    }
}
