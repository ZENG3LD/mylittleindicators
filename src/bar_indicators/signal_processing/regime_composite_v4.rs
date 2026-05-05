// RegimeCompositeV4: combine Hurst/DFA + SpectralSlope + SpectralEnergyRatio + VolOfVolPercentile + ATR Percentile trend

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::chaos::{dfa::Dfa, hurst_exponent::HurstExponent};
use crate::bar_indicators::signal_processing::{SpectralEnergyRatio, SpectralSlope};
use crate::bar_indicators::volatility::{
    atr_percentile::AtrPercentile, vol_of_vol_percentile::VolOfVolPercentile,
};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct RegimeCompositeV4 {
    hurst: HurstExponent,
    dfa: Dfa,
    slope: SpectralSlope,
    ser: SpectralEnergyRatio,
    vovp: VolOfVolPercentile,
    atrp: AtrPercentile,
    pub value: f64,
}

impl RegimeCompositeV4 {
    pub fn new(
        hurst_window: usize,
        dfa_scales: [usize; 4],
        fft_window: usize,
        ser_low_cut: f64,
        vov_window: usize,
        perc_window: usize,
        atr_period: usize,
    ) -> Self {
        Self {
            hurst: HurstExponent::new(hurst_window),
            dfa: Dfa::new(dfa_scales),
            slope: SpectralSlope::new(fft_window),
            ser: SpectralEnergyRatio::new(fft_window, ser_low_cut),
            vovp: VolOfVolPercentile::new(None, vov_window, perc_window),
            atrp: AtrPercentile::new(atr_period, MovingAverageType::RMA, perc_window),
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.hurst.reset();
        self.dfa.reset();
        self.slope.reset();
        self.ser.reset();
        self.vovp.reset();
        self.atrp.reset();
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.hurst.is_ready() && self.dfa.is_ready() && self.slope.is_ready() && self.ser.is_ready()
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let hval = self.hurst.update(c);
        let dval = self.dfa.update_bar(o, h, l, c, v);
        let sval = self.slope.update_bar(o, h, l, c, v);
        let ser = self.ser.update_bar(o, h, l, c, v);
        let vovp = self.vovp.update_bar(o, h, l, c, v);
        let atrp = self.atrp.update_bar(o, h, l, c, v);
        // normalize components to [-1,1] where feasible and combine
        let trendiness = (hval - 0.5) * 2.0; // H in (0,1) -> (-1,1)
        let persistence = (1.0 - dval).clamp(0.0, 1.0) * 2.0 - 1.0; // inverse DFA as proxy
        let spectral_trend = (-sval).tanh(); // negative slope => trend
        let low_band_bias = (ser * 2.0 - 1.0).clamp(-1.0, 1.0);
        let vov_state = (vovp * 2.0 - 1.0).clamp(-1.0, 1.0);
        let atr_state = (atrp * 2.0 - 1.0).clamp(-1.0, 1.0);
        self.value = 0.25 * trendiness
            + 0.15 * persistence
            + 0.2 * spectral_trend
            + 0.15 * low_band_bias
            + 0.15 * vov_state
            + 0.1 * atr_state;
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
    fn test_regime_composite_v4_creation() {
        let rc = RegimeCompositeV4::new(100, [8, 16, 32, 64], 64, 0.1, 20, 100, 14);
        assert!(!rc.is_ready());
        assert_eq!(rc.value().main(), 0.0);
    }

    #[test]
    fn test_regime_composite_v4_warmup() {
        let mut rc = RegimeCompositeV4::new(100, [8, 16, 32, 64], 64, 0.1, 20, 100, 14);
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rc.is_ready());
    }

    #[test]
    fn test_regime_composite_v4_finite() {
        let mut rc = RegimeCompositeV4::new(100, [8, 16, 32, 64], 64, 0.1, 20, 100, 14);
        for i in 0..250 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let value = rc.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "Score should be finite, got {}", value);
        }
    }

    #[test]
    fn test_regime_composite_v4_reset() {
        let mut rc = RegimeCompositeV4::new(100, [8, 16, 32, 64], 64, 0.1, 20, 100, 14);
        for i in 0..200 {
            rc.update_bar(100.0 + i as f64, 101.0 + i as f64, 99.0 + i as f64, 100.0 + i as f64, 1000.0);
        }
        rc.reset();
        assert!(!rc.is_ready());
        assert_eq!(rc.value().main(), 0.0);
    }
}
