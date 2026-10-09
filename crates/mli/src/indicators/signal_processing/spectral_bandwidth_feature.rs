// Spectral Bandwidth feature from FFT

use crate::indicators::signal_processing::fft::FastFourierTransform;

#[derive(Debug, Clone)]
pub struct SpectralBandwidthFeature {
    fft: FastFourierTransform,
    pub value: f64,
}

impl SpectralBandwidthFeature {
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
        self.value = fd.spectral_bandwidth;
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
}

impl Default for SpectralBandwidthFeature {
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

/// Own config for [`SpectralBandwidthFeature`] — period-only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SbwfConfig {
    pub period: Param<usize>,
}

impl Indicator for SpectralBandwidthFeature {
    const ID: IndicatorId = IndicatorId::Sbwf;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Fft, &[IndicatorOutputId::Fft])],
    };
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Sbwf)];
    type Config = SbwfConfig;
    type Runtime = SpectralBandwidthFeature;

    fn create(cfg: SbwfConfig) -> SpectralBandwidthFeature {
        SpectralBandwidthFeature::new(cfg.period.resolved())
    }

    fn source_fields(cfg: &SbwfConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for SbwfConfig {
    fn defaults() -> Self {
        SbwfConfig { period: Param::Solo(128) }
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


impl Render for SpectralBandwidthFeature {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Sbwf, "Spectral Bandwidth", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_bandwidth_creation() {
        let sbw = SpectralBandwidthFeature::new(64);
        assert!(!sbw.is_ready());
        assert_eq!(sbw.value(), 0.0);
    }

    #[test]
    fn test_spectral_bandwidth_warmup() {
        let mut sbw = SpectralBandwidthFeature::new(64);
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sbw.feed(price);
        }
        assert!(sbw.is_ready());
    }

    #[test]
    fn test_spectral_bandwidth_finite() {
        let mut sbw = SpectralBandwidthFeature::new(64);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sbw.feed(price);
            assert!(value.is_finite(), "Bandwidth should be finite");
        }
    }

    #[test]
    fn test_spectral_bandwidth_reset() {
        let mut sbw = SpectralBandwidthFeature::new(64);
        for i in 0..70 {
            sbw.feed(100.0 + i as f64);
        }
        sbw.reset();
        assert!(!sbw.is_ready());
        assert_eq!(sbw.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_sbwf() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralBandwidthFeature as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Sbwf(cfg).build_solo().unwrap();
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
