// Distance-to-Levels: distances to rolling_midline and percentile_channels mid

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::channels::percentile_channels::{
    PercentileBasis, PercentileChannels,
};
use crate::bar_indicators::levels::rolling_midline::RollingMidline;

#[derive(Clone)]
pub struct DistanceToLevels {
    mid: RollingMidline,
    pct: PercentileChannels,
    last_dist_mid: f64,
    last_dist_mid_pct: f64,
}

impl DistanceToLevels {
    pub fn new(mid_window: usize, pct_window: usize, low_pct: f64, high_pct: f64) -> Self {
        Self {
            mid: RollingMidline::new(mid_window),
            pct: PercentileChannels::new(pct_window, PercentileBasis::Close, low_pct, high_pct),
            last_dist_mid: 0.0,
            last_dist_mid_pct: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.mid.reset();
        self.pct.reset();
        self.last_dist_mid = 0.0;
        self.last_dist_mid_pct = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.mid.is_ready()
    }

    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> (f64, f64) {
        let mid = self.mid.update_bar(open, high, low, close, volume);
        let (_lower, mid_pct, _upper) = self.pct.update_bar(open, high, low, close, volume);
        let dist_mid = if mid != 0.0 {
            (close - mid) / mid.abs().max(1e-9)
        } else {
            0.0
        };
        let dist_mid_pct = if mid_pct != 0.0 {
            (close - mid_pct) / mid_pct.abs().max(1e-9)
        } else {
            0.0
        };
        self.last_dist_mid = dist_mid;
        self.last_dist_mid_pct = dist_mid_pct;
        (dist_mid, dist_mid_pct)
    }

    #[inline]
    pub fn values(&self) -> (f64, f64) {
        (self.last_dist_mid, self.last_dist_mid_pct)
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.last_dist_mid, self.last_dist_mid_pct)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_to_levels_creation() {
        let dtl = DistanceToLevels::new(20, 20, 0.25, 0.75);
        assert!(!dtl.is_ready());
    }

    #[test]
    fn test_distance_to_levels_warmup() {
        let mut dtl = DistanceToLevels::new(20, 20, 0.25, 0.75);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            dtl.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(dtl.is_ready());
    }

    #[test]
    fn test_distance_to_levels_values() {
        let mut dtl = DistanceToLevels::new(20, 20, 0.25, 0.75);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let (d1, d2) = dtl.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(d1.is_finite(), "Distance should be finite");
            assert!(d2.is_finite(), "Distance should be finite");
        }
    }

    #[test]
    fn test_distance_to_levels_reset() {
        let mut dtl = DistanceToLevels::new(20, 20, 0.25, 0.75);
        for i in 0..25 {
            dtl.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0, 1000.0);
        }
        dtl.reset();
        assert!(!dtl.is_ready());
    }
}
