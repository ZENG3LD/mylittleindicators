// Composite regime score: combine KalmanRegimeScore with ATR/VoV percentiles

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::kalman::kalman_regime_score::KalmanRegimeScore;
use crate::bar_indicators::volatility::atr_percentile::AtrPercentile;
use crate::bar_indicators::volatility::close_to_close_vol_percentile::CloseVolPercentile;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct KalmanRegimeComposite {
    regime: KalmanRegimeScore,
    atrp: AtrPercentile,
    cvp: CloseVolPercentile,
    w_regime: f64,
    w_atr: f64,
    w_vov: f64,
    pub value: f64,
}

impl KalmanRegimeComposite {
    pub fn new(
        dt: f64,
        q: f64,
        r: f64,
        k_window: usize,
        decay: f64,
        atr_period: usize,
        atr_ma: MovingAverageType,
        atr_pct_window: usize,
        vov_vol_window: usize,
        vov_pct_window: usize,
        w_regime: f64,
        w_atr: f64,
        w_vov: f64,
    ) -> Self {
        Self {
            regime: KalmanRegimeScore::new(dt, q, r, k_window, decay),
            atrp: AtrPercentile::new(atr_period, atr_ma, atr_pct_window),
            cvp: CloseVolPercentile::new(vov_vol_window, vov_pct_window),
            w_regime,
            w_atr,
            w_vov,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.regime.reset();
        self.atrp.reset();
        self.cvp.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.atrp.is_ready() && self.cvp.is_ready() && self.regime.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let r = self.regime.update_bar(o, h, l, c, v);
        let a = self.atrp.update_bar(o, h, l, c, v);
        let (_vv, _pct) = self.cvp.update_bar(o, h, l, c, v);
        let vv = _pct;
        self.value = (self.w_regime * r + self.w_atr * (1.0 - a) + self.w_vov * (1.0 - vv))
            / (self.w_regime + self.w_atr + self.w_vov).max(1e-9);
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_regime_composite_creation() {
        let krc = KalmanRegimeComposite::new(
            1.0, 0.1, 1.0, 20, 0.94,
            14, MovingAverageType::EMA, 50,
            20, 50,
            0.4, 0.3, 0.3
        );
        assert!(!krc.is_ready());
        assert_eq!(krc.value, 0.0);
    }

    #[test]
    fn test_kalman_regime_composite_values_finite() {
        let mut krc = KalmanRegimeComposite::new(
            1.0, 0.1, 1.0, 20, 0.94,
            14, MovingAverageType::EMA, 50,
            20, 50,
            0.4, 0.3, 0.3
        );
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = krc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_kalman_regime_composite_reset() {
        let mut krc = KalmanRegimeComposite::new(
            1.0, 0.1, 1.0, 20, 0.94,
            14, MovingAverageType::EMA, 50,
            20, 50,
            0.4, 0.3, 0.3
        );
        for i in 0..100 {
            krc.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        krc.reset();
        assert!(!krc.is_ready());
        assert_eq!(krc.value, 0.0);
    }
}
