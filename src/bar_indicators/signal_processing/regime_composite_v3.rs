// Regime Composite v3: blends spectral energy ratio, vol-of-vol, DFA (if available) and Shannon entropy

use crate::bar_indicators::entropy::shannon_entropy::ShannonEntropy;
use crate::bar_indicators::signal_processing::spectral_energy_ratio::SpectralEnergyRatio;
use crate::bar_indicators::volatility::vol_of_vol::{VoVSource, VolOfVol};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct RegimeCompositeV3 {
    ser: SpectralEnergyRatio,
    vov: VolOfVol,
    ent: ShannonEntropy,
    pub value: f64,
}

impl RegimeCompositeV3 {
    pub fn new(
        window: usize,
        low_cut_fraction: f64,
        vov_window: usize,
        entropy_bins: usize,
    ) -> Self {
        Self {
            ser: SpectralEnergyRatio::new(window, low_cut_fraction),
            vov: VolOfVol::new(VoVSource::AbsReturn, vov_window),
            ent: ShannonEntropy::new(window, entropy_bins),
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ser.reset();
        self.vov.reset();
        self.ent.reset();
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ser.is_ready()
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let s = self.ser.update_bar(o, h, l, c, v); // 0..1 low/(low+high)
        let vv = self.vov.update_bar(o, h, l, c, v).abs(); // normalize later
        let e = self.ent.update_bar(o, h, l, c, v).min(1.0); // 0..1
        let vvn = (vv / 0.05).min(1.0); // heuristic normalization
                                        // weights: emphasize spectral structure and stability (low vov, lower entropy)
        let w_s = 0.4;
        let w_v = 0.3;
        let w_e = 0.3;
        self.value = (w_s * s + w_v * (1.0 - vvn) + w_e * (1.0 - e)).clamp(0.0, 1.0);
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
    fn test_regime_composite_v3_creation() {
        let rc = RegimeCompositeV3::new(32, 0.1, 20, 10);
        assert!(!rc.is_ready());
        assert_eq!(rc.value().main(), 0.0);
    }

    #[test]
    fn test_regime_composite_v3_warmup() {
        let mut rc = RegimeCompositeV3::new(32, 0.1, 20, 10);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rc.is_ready());
    }

    #[test]
    fn test_regime_composite_v3_range() {
        let mut rc = RegimeCompositeV3::new(32, 0.1, 20, 10);
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let value = rc.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            if rc.is_ready() {
                assert!(value >= 0.0 && value <= 1.0, "Score should be in [0, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_regime_composite_v3_reset() {
        let mut rc = RegimeCompositeV3::new(32, 0.1, 20, 10);
        for i in 0..100 {
            rc.update_bar(100.0 + i as f64, 101.0 + i as f64, 99.0 + i as f64, 100.0 + i as f64, 1000.0);
        }
        rc.reset();
        assert!(!rc.is_ready());
        assert_eq!(rc.value().main(), 0.0);
    }
}
