// ATR Percentile Trend: EMA-detrended ATR percentile

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::volatility::atr_percentile::AtrPercentile;

pub struct AtrPercentileTrend {
    inner: AtrPercentile,
    alpha: f64,
    ema: f64,
    init: bool,
    pub value: f64,
}

impl AtrPercentileTrend {
    pub fn new(
        atr_period: usize,
        ma_type: MovingAverageType,
        pct_window: usize,
        alpha: f64,
    ) -> Self {
        Self {
            inner: AtrPercentile::new(atr_period, ma_type, pct_window),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atr_percentile_trend_creation() {
        let apt = AtrPercentileTrend::new(14, MovingAverageType::RMA, 50, 0.1);
        assert!(!apt.is_ready());
        assert_eq!(apt.value, 0.0);
    }

    #[test]
    fn test_atr_percentile_trend_warmup() {
        let mut apt = AtrPercentileTrend::new(14, MovingAverageType::RMA, 50, 0.1);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            apt.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(apt.is_ready());
    }

    #[test]
    fn test_atr_percentile_trend_values() {
        let mut apt = AtrPercentileTrend::new(14, MovingAverageType::RMA, 50, 0.1);
        for i in 0..60 {
            let price = 100.0 + i as f64;
            let value = apt.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_atr_percentile_trend_reset() {
        let mut apt = AtrPercentileTrend::new(14, MovingAverageType::RMA, 50, 0.1);
        for i in 0..60 {
            apt.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        apt.reset();
        assert!(!apt.is_ready());
        assert_eq!(apt.value, 0.0);
    }
}
