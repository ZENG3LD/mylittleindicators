// Spectral Flatness (Wiener entropy): geometric mean / arithmetic mean of power spectrum

use crate::indicators::signal_processing::fft::FastFourierTransform;

#[derive(Debug, Clone)]
pub struct SpectralFlatness {
    window: usize,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralFlatness {
    pub fn new(window: usize) -> Self {
        let w = window.clamp(16, 256);
        Self {
            window: w,
            fft: FastFourierTransform::new(w, 1.0),
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.buf.fill(0.0);
        self.value = 0.0;
        self.fft.reset();
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.fft.is_ready()
    }

    pub fn feed(&mut self, c: f64) -> f64 {
        let n = self.window;
        self.buf[self.idx] = c;
        self.idx = (self.idx + 1) % n;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            // demean and feed
            let mut mean = 0.0;
            for i in 0..n {
                mean += self.buf[i];
            }
            mean /= n as f64;
            for i in 0..n {
                let x = self.buf[(self.idx + i) % n] - mean;
                self.fft.update(x);
            }
            let fd = self.fft.frequency_domain();
            let k = fd.power_spectrum.len().max(1);
            let mut am = 0.0;
            let mut lg = 0.0;
            for i in 0..fd.power_spectrum.len() {
                let p = fd.power_spectrum[i].max(1e-18);
                am += p;
                lg += p.ln();
            }
            let amean = am / k as f64;
            let gmean = (lg / k as f64).exp();
            self.value = if amean > 0.0 {
                (gmean / amean).clamp(0.0, 1.0)
            } else {
                0.0
            };
        }
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

impl Default for SpectralFlatness {
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

/// Own config for [`SpectralFlatness`] — period-only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SflatConfig {
    pub period: Param<usize>,
}

impl Indicator for SpectralFlatness {
    const ID: IndicatorId = IndicatorId::Sflat;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Fft, &[IndicatorOutputId::Fft])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Sflat)];
    type Config = SflatConfig;
    type Runtime = SpectralFlatness;

    fn create(cfg: SflatConfig) -> SpectralFlatness {
        SpectralFlatness::new(cfg.period.resolved())
    }

    fn source_fields(cfg: &SflatConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for SflatConfig {
    fn defaults() -> Self {
        SflatConfig { period: Param::Solo(128) }
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


impl Render for SpectralFlatness {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Sflat, "Spectral Flatness", Color::hex(0x009688))
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    }
}

// Transitional bridge: un-contracted composites still call update_bar; delegates to the
// contracted feed (default source = close). Removed when each caller is Port-wired.
impl SpectralFlatness {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_flatness_creation() {
        let sf = SpectralFlatness::new(64);
        assert!(!sf.is_ready());
        assert_eq!(sf.value(), 0.0);
        assert_eq!(sf.window(), 64);
    }

    #[test]
    fn test_spectral_flatness_warmup() {
        let mut sf = SpectralFlatness::new(64);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sf.feed(price);
        }
        assert!(sf.is_ready());
    }

    #[test]
    fn test_spectral_flatness_range() {
        let mut sf = SpectralFlatness::new(64);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sf.feed(price);
            if sf.is_ready() {
                assert!(value >= 0.0 && value <= 1.0, "Flatness should be in [0, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_spectral_flatness_reset() {
        let mut sf = SpectralFlatness::new(64);
        for i in 0..70 {
            sf.feed(100.0 + i as f64);
        }
        sf.reset();
        assert!(!sf.is_ready());
        assert_eq!(sf.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_sflat() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralFlatness as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Sflat(cfg).build_solo().unwrap();
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
