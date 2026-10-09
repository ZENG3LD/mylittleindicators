// Spectral Flux proxy via absolute change of Spectral Rolloff

use crate::indicators::signal_processing::spectral_rolloff::SpectralRolloff;

#[derive(Debug, Clone)]
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
    pub fn feed(&mut self, c: f64) -> f64 {
        let r = self.rolloff.feed(c);
        if let Some(p) = self.prev {
            let flux = (r - p).abs();
            self.value = self.alpha * flux + (1.0 - self.alpha) * self.value;
        }
        self.prev = Some(r);
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity,
};
use crate::contract::axis::sweep_f64;
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`SpectralFluxProxy`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SfluxConfig {
    /// FFT window fed to the inner SpectralRolloff.
    pub fft_window: Param<usize>,
    /// Rolloff threshold fraction (e.g. 0.85).
    pub rolloff_percent: Param<f64>,
    /// EMA smoothing factor for the flux (0.0–1.0).
    pub alpha: Param<f64>,
}

impl Indicator for SpectralFluxProxy {
    const ID: IndicatorId = IndicatorId::Sflux;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Sroll, &[IndicatorOutputId::Sroll])],
    };
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Sflux)];
    type Config = SfluxConfig;
    type Runtime = SpectralFluxProxy;

    fn create(cfg: SfluxConfig) -> SpectralFluxProxy {
        SpectralFluxProxy::new(cfg.fft_window.resolved(), cfg.rolloff_percent.resolved(), cfg.alpha.resolved())
    }

    fn source_fields(cfg: &SfluxConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for SfluxConfig {
    fn defaults() -> Self {
        SfluxConfig { fft_window: Param::Solo(64), rolloff_percent: Param::Solo(0.85), alpha: Param::Solo(0.2) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // fft_window → range(2,4048,1)
        // rolloff_percent: Class G target — rolloff percentile fraction, meaningful range 0.70..0.99
        s.rolloff_percent = Param::many(sweep_f64(0.70, 0.99, 0.01));
        // alpha: Class E alpha/decay
        s.alpha = Param::many(sweep_f64(0.01, 0.99, 0.01));
        s
    }
}


impl Render for SpectralFluxProxy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Sflux, "Spectral Flux", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_flux_creation() {
        let sfp = SpectralFluxProxy::new(64, 0.85, 0.2);
        assert!(!sfp.is_ready());
        assert_eq!(sfp.value(), 0.0);
        assert!((sfp.alpha() - 0.2).abs() < 1e-9);
    }

    #[test]
    fn test_spectral_flux_warmup() {
        let mut sfp = SpectralFluxProxy::new(64, 0.85, 0.2);
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sfp.feed(price);
        }
        assert!(sfp.is_ready());
    }

    #[test]
    fn test_spectral_flux_finite() {
        let mut sfp = SpectralFluxProxy::new(64, 0.85, 0.2);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sfp.feed(price);
            assert!(value.is_finite(), "Flux should be finite");
        }
    }

    #[test]
    fn test_spectral_flux_reset() {
        let mut sfp = SpectralFluxProxy::new(64, 0.85, 0.2);
        for i in 0..80 {
            sfp.feed(100.0 + i as f64);
        }
        sfp.reset();
        assert!(!sfp.is_ready());
        assert_eq!(sfp.value(), 0.0);
    }

    /// Factory resolves close (not wild 9999 high) and feeds the flux proxy.
    #[test]
    fn factory_feeds_resolved_sflux() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralFluxProxy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Sflux(cfg).build_solo().unwrap();
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
