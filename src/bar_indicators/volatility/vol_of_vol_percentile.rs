// Percentile of Volatility-of-Volatility over rolling window

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::volatility::vol_of_vol::{VoVSource, VolOfVol};

#[derive(Clone)]
pub struct VolOfVolPercentile {
    vov: VolOfVol,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl VolOfVolPercentile {
    pub fn new(
        source: Option<(usize, MovingAverageType)>,
        vov_window: usize,
        percentile_window: usize,
    ) -> Self {
        let vov = match source {
            Some((p, t)) => VolOfVol::new(VoVSource::Atr(p, t), vov_window),
            None => VolOfVol::new(VoVSource::AbsReturn, vov_window),
        };
        let w = percentile_window.max(1);
        Self {
            vov,
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.vov.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let cur = self.vov.update_bar(o, h, l, c, v).abs();
        let _old = self.buf[self.idx];
        self.buf[self.idx] = cur;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut le = 0usize;
            for i in 0..len {
                if self.buf[i] <= cur {
                    le += 1;
                }
            }
            self.value = le as f64 / len as f64;
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vol_of_vol_percentile_creation() {
        let vovp = VolOfVolPercentile::new(Some((14, MovingAverageType::RMA)), 20, 50);
        assert!(!vovp.is_ready());
        assert_eq!(vovp.value(), 0.0);
    }

    #[test]
    fn test_vol_of_vol_percentile_abs_return() {
        let vovp = VolOfVolPercentile::new(None, 20, 50);
        assert!(!vovp.is_ready());
    }

    #[test]
    fn test_vol_of_vol_percentile_warmup() {
        let mut vovp = VolOfVolPercentile::new(Some((14, MovingAverageType::RMA)), 20, 50);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vovp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vovp.is_ready());
    }

    #[test]
    fn test_vol_of_vol_percentile_range() {
        let mut vovp = VolOfVolPercentile::new(Some((14, MovingAverageType::RMA)), 20, 50);
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vovp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0, "Percentile should be in [0, 1]");
        }
    }

    #[test]
    fn test_vol_of_vol_percentile_reset() {
        let mut vovp = VolOfVolPercentile::new(Some((14, MovingAverageType::RMA)), 20, 50);
        for i in 0..70 {
            vovp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        vovp.reset();
        assert!(!vovp.is_ready());
        assert_eq!(vovp.value(), 0.0);
    }
}
