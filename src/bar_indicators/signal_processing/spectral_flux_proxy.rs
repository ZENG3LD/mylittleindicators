// Spectral Flux proxy via absolute change of Spectral Rolloff

use crate::bar_indicators::signal_processing::spectral_rolloff::SpectralRolloff;
use crate::bar_indicators::indicator_value::IndicatorValue;

pub struct SpectralFluxProxy {
    rolloff: SpectralRolloff,
    alpha: f64,
    prev: Option<f64>,
    pub value: f64,
}

impl SpectralFluxProxy {
    pub fn new(fft_window: usize, rolloff_percent: f64, ema_alpha: f64) -> Self {
        Self {
            rolloff: SpectralRolloff::new(fft_window, rolloff_percent),
            alpha: ema_alpha.clamp(0.0, 1.0),
            prev: None,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.rolloff.reset();
        self.prev = None;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rolloff.is_ready() && self.prev.is_some()
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let r = self.rolloff.update_bar(o, h, l, c, v);
        if let Some(p) = self.prev {
            let flux = (r - p).abs();
            self.value = self.alpha * flux + (1.0 - self.alpha) * self.value;
        }
        self.prev = Some(r);
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_flux_creation() {
        let sfp = SpectralFluxProxy::new(64, 0.85, 0.2);
        assert!(!sfp.is_ready());
        assert_eq!(sfp.value().main(), 0.0);
        assert!((sfp.alpha() - 0.2).abs() < 1e-9);
    }

    #[test]
    fn test_spectral_flux_warmup() {
        let mut sfp = SpectralFluxProxy::new(64, 0.85, 0.2);
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sfp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sfp.is_ready());
    }

    #[test]
    fn test_spectral_flux_finite() {
        let mut sfp = SpectralFluxProxy::new(64, 0.85, 0.2);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sfp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Flux should be finite");
        }
    }

    #[test]
    fn test_spectral_flux_reset() {
        let mut sfp = SpectralFluxProxy::new(64, 0.85, 0.2);
        for i in 0..80 {
            sfp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        sfp.reset();
        assert!(!sfp.is_ready());
        assert_eq!(sfp.value().main(), 0.0);
    }
}
