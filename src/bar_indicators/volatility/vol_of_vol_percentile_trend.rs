// Vol-of-Vol Percentile Trend: EMA-detrended percentile of VolOfVol

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::volatility::vol_of_vol_percentile::VolOfVolPercentile;

#[derive(Clone)]
pub struct VolOfVolPercentileTrend {
    inner: VolOfVolPercentile,
    alpha: f64,
    ema: f64,
    init: bool,
    pub value: f64,
}

impl VolOfVolPercentileTrend {
    pub fn new(
        source: Option<(
            usize,
            crate::bar_indicators::average::MovingAverageType,
        )>,
        vov_window: usize,
        pct_window: usize,
        alpha: f64,
    ) -> Self {
        Self {
            inner: VolOfVolPercentile::new(source, vov_window, pct_window),
            alpha: alpha.clamp(0.01, 1.0),
            ema: 0.0,
            init: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.ema = 0.0;
        self.init = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let p = self.inner.update_bar(o, h, l, c, v);
        if !self.init {
            self.ema = p;
            self.init = true;
        }
        self.ema = self.alpha * p + (1.0 - self.alpha) * self.ema;
        self.value = p - self.ema;
        self.value
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar_indicators::average::MovingAverageType;

    #[test]
    fn test_vol_of_vol_percentile_trend_creation() {
        let vovpt = VolOfVolPercentileTrend::new(Some((14, MovingAverageType::RMA)), 20, 50, 0.1);
        assert!(!vovpt.is_ready());
        assert_eq!(vovpt.value, 0.0);
    }

    #[test]
    fn test_vol_of_vol_percentile_trend_warmup() {
        let mut vovpt = VolOfVolPercentileTrend::new(Some((14, MovingAverageType::RMA)), 20, 50, 0.1);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vovpt.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vovpt.is_ready());
    }

    #[test]
    fn test_vol_of_vol_percentile_trend_values() {
        let mut vovpt = VolOfVolPercentileTrend::new(Some((14, MovingAverageType::RMA)), 20, 50, 0.1);
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vovpt.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Trend value should be finite");
        }
    }

    #[test]
    fn test_vol_of_vol_percentile_trend_reset() {
        let mut vovpt = VolOfVolPercentileTrend::new(Some((14, MovingAverageType::RMA)), 20, 50, 0.1);
        for i in 0..70 {
            vovpt.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        vovpt.reset();
        assert!(!vovpt.is_ready());
        assert_eq!(vovpt.value, 0.0);
    }
}
