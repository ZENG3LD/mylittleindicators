// Regime Composite v2: combines spectral energy ratio, volatility percentile, and entropy into [0..1]

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::entropy::shannon_entropy::ShannonEntropy;
use crate::bar_indicators::signal_processing::spectral_flatness::SpectralFlatness;
use crate::bar_indicators::signal_processing::spectral_rolloff::SpectralRolloff;
use crate::bar_indicators::volatility::atr_percentile::AtrPercentile;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct RegimeCompositeV2 {
    flat: SpectralFlatness,
    roll: SpectralRolloff,
    atrp: AtrPercentile,
    sent: ShannonEntropy,
    pub value: f64,
}

impl RegimeCompositeV2 {
    pub fn new(
        window: usize,
        roll_target: f64,
        atr_period: usize,
        atr_ma: MovingAverageType,
        atr_pct_window: usize,
        shannon_bins: usize,
    ) -> Self {
        Self {
            flat: SpectralFlatness::new(window),
            roll: SpectralRolloff::new(window, roll_target),
            atrp: AtrPercentile::new(atr_period, atr_ma, atr_pct_window),
            sent: ShannonEntropy::new(window, shannon_bins),
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.flat.reset();
        self.roll.reset();
        self.atrp.reset();
        self.sent.reset();
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.flat.is_ready()
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let f = self.flat.update_bar(open, high, low, close, volume); // 0..1
        let r = self.roll.update_bar(open, high, low, close, volume); // freq ∈ [0, 0.5]
        let a = self.atrp.update_bar(open, high, low, close, volume); // 0..100
        let e = self.sent.update_bar(open, high, low, close, volume); // unnormalized
                                                                      // normalize proxies
        let roll_norm = (r * 2.0).min(1.0); // scale Nyquist=0.5 -> 1.0
        let atr_norm = (a / 100.0).min(1.0);
        let ent_norm = (e / 5.0).min(1.0); // empirical cap
                                           // weights
        let w_f = 0.35;
        let w_r = 0.25;
        let w_a = 0.25;
        let w_e = 0.15;
        self.value = (w_f * f + w_r * roll_norm + w_a * atr_norm + w_e * ent_norm).clamp(0.0, 1.0);
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regime_composite_v2_creation() {
        let rc = RegimeCompositeV2::new(32, 0.85, 14, MovingAverageType::RMA, 100, 10);
        assert!(!rc.is_ready());
        assert_eq!(rc.value().main(), 0.0);
    }

    #[test]
    fn test_regime_composite_v2_warmup() {
        let mut rc = RegimeCompositeV2::new(32, 0.85, 14, MovingAverageType::RMA, 100, 10);
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rc.is_ready());
    }

    #[test]
    fn test_regime_composite_v2_range() {
        let mut rc = RegimeCompositeV2::new(32, 0.85, 14, MovingAverageType::RMA, 100, 10);
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let value = rc.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            if rc.is_ready() {
                assert!(value >= 0.0 && value <= 1.0, "Score should be in [0, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_regime_composite_v2_reset() {
        let mut rc = RegimeCompositeV2::new(32, 0.85, 14, MovingAverageType::RMA, 100, 10);
        for i in 0..150 {
            rc.update_bar(100.0 + i as f64, 101.0 + i as f64, 99.0 + i as f64, 100.0 + i as f64, 1000.0);
        }
        rc.reset();
        assert!(!rc.is_ready());
        assert_eq!(rc.value().main(), 0.0);
    }
}
