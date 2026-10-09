// Ratio of spectral bandpower: high / low

use crate::indicators::signal_processing::fft::FastFourierTransform;

#[derive(Debug, Clone)]
pub struct SpectralBandpowerRatioHL {
    window: usize,
    low_cut: f64,
    high_cut: f64,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralBandpowerRatioHL {
    pub fn new(window: usize, low_cut_fraction: f64, high_cut_fraction: f64) -> Self {
        let w = window.clamp(32, 1024);
        let lc = low_cut_fraction.clamp(1e-6, 0.49);
        let hc = high_cut_fraction.clamp(lc + 1e-6, 0.499);
        Self {
            window: w,
            low_cut: lc,
            high_cut: hc,
            fft: FastFourierTransform::new(w, 1.0),
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 1.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.fft.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 1.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn feed(&mut self, c: f64) -> f64 {
        let sr = self.fft.sampling_rate();
        let fd = self.fft.update(c);
        let freqs = &fd.frequencies;
        let power = &fd.power_spectrum;
        if freqs.is_empty() || power.is_empty() {
            return self.value;
        }
        let mut low = 0.0;
        let mut high = 0.0;
        let nyq = 0.5;
        for (i, &f) in freqs.iter().enumerate() {
            let frac = f / (sr.max(1e-9));
            let p = *power.get(i).unwrap_or(&0.0);
            if frac < self.low_cut * nyq {
                low += p;
            } else if frac >= self.high_cut * nyq {
                high += p;
            }
        }
        self.value = if low > 0.0 { high / low } else { self.value };
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn window(&self) -> usize {
        self.window
    }

}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::axis::sweep_f64;
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`SpectralBandpowerRatioHL`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SbprhlConfig {
    pub window: Param<usize>,
    pub low_cut: Param<f64>,
    pub high_cut: Param<f64>,
}

impl Indicator for SpectralBandpowerRatioHL {
    const ID: IndicatorId = IndicatorId::Sbprhl;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Fft, &[IndicatorOutputId::Fft])],
    };
    const OUTPUTS: &'static [Output] = &[Output::ratio(IndicatorOutputId::Sbprhl)];
    type Config = SbprhlConfig;
    type Runtime = SpectralBandpowerRatioHL;

    fn create(cfg: SbprhlConfig) -> SpectralBandpowerRatioHL {
        SpectralBandpowerRatioHL::new(cfg.window.resolved(), cfg.low_cut.resolved(), cfg.high_cut.resolved())
    }

    fn source_fields(cfg: &SbprhlConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for SbprhlConfig {
    fn defaults() -> Self {
        SbprhlConfig { window: Param::Solo(64), low_cut: Param::Solo(0.2), high_cut: Param::Solo(0.6) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // window → range(2,4048,1)
        // low_cut / high_cut: Class G Nyquist fraction 0..0.5
        s.low_cut = Param::many(sweep_f64(0.01, 0.49, 0.01));
        s.high_cut = Param::many(sweep_f64(0.01, 0.49, 0.01));
        s
    }
}


impl Render for SpectralBandpowerRatioHL {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Sbprhl, "Band Pass RHL", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_bandpower_ratio_creation() {
        let sbr = SpectralBandpowerRatioHL::new(64, 0.2, 0.6);
        assert!(!sbr.is_ready());
        assert_eq!(sbr.value(), 1.0);
        assert_eq!(sbr.window(), 64);
    }

    #[test]
    fn test_spectral_bandpower_ratio_finite() {
        let mut sbr = SpectralBandpowerRatioHL::new(64, 0.2, 0.6);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sbr.feed(price);
            assert!(value.is_finite(), "Ratio should be finite, got {}", value);
        }
    }

    #[test]
    fn test_spectral_bandpower_ratio_reset() {
        let mut sbr = SpectralBandpowerRatioHL::new(64, 0.2, 0.6);
        for i in 0..70 {
            sbr.feed(100.0 + i as f64);
        }
        sbr.reset();
        assert!(!sbr.is_ready());
        assert_eq!(sbr.value(), 1.0);
    }

    #[test]
    fn factory_feeds_resolved_sbprhl() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralBandpowerRatioHL as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Sbprhl(cfg).build_solo().unwrap();
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
