// Spectral Centroid feature from FFT

use crate::indicators::signal_processing::fft::FastFourierTransform;

#[derive(Debug, Clone)]
pub struct SpectralCentroidFeature {
    fft: FastFourierTransform,
    pub value: f64,
}

impl SpectralCentroidFeature {
    pub fn new(window: usize) -> Self {
        Self {
            fft: FastFourierTransform::new(window, 1.0),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.fft.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.fft.is_ready()
    }
    pub fn feed(&mut self, c: f64) -> f64 {
        let fd = self.fft.update(c);
        self.value = fd.spectral_centroid;
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

}

impl Default for SpectralCentroidFeature {
    /// Factory default: window=128.
    fn default() -> Self {
        Self::new(128)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`SpectralCentroidFeature`] — period-only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ScfConfig {
    pub period: Param<usize>,
}

impl Indicator for SpectralCentroidFeature {
    const ID: IndicatorId = IndicatorId::Scf;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Fft, &[IndicatorOutputId::Fft])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Scf)];
    type Config = ScfConfig;
    type Runtime = SpectralCentroidFeature;

    fn create(cfg: ScfConfig) -> SpectralCentroidFeature {
        SpectralCentroidFeature::new(cfg.period.resolved())
    }

    fn source_fields(cfg: &ScfConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for ScfConfig {
    fn defaults() -> Self {
        ScfConfig { period: Param::Solo(128) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A usize → auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
}


impl Render for SpectralCentroidFeature {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Scf, "Spectral Centroid", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_centroid_creation() {
        let sc = SpectralCentroidFeature::new(64);
        assert!(!sc.is_ready());
        assert_eq!(sc.value(), 0.0);
    }

    #[test]
    fn test_spectral_centroid_warmup() {
        let mut sc = SpectralCentroidFeature::new(64);
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sc.feed(price);
        }
        assert!(sc.is_ready());
    }

    #[test]
    fn test_spectral_centroid_finite() {
        let mut sc = SpectralCentroidFeature::new(64);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sc.feed(price);
            assert!(value.is_finite(), "Centroid should be finite");
        }
    }

    #[test]
    fn test_spectral_centroid_reset() {
        let mut sc = SpectralCentroidFeature::new(64);
        for i in 0..70 {
            sc.feed(100.0 + i as f64);
        }
        sc.reset();
        assert!(!sc.is_ready());
        assert_eq!(sc.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_scf() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralCentroidFeature as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Scf(cfg).build_solo().unwrap();
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
