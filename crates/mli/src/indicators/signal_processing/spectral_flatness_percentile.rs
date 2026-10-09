// Percentile rank of Spectral Flatness over window

use crate::indicators::signal_processing::spectral_flatness::SpectralFlatness;

#[derive(Debug, Clone)]
pub struct SpectralFlatnessPercentile {
    inner: SpectralFlatness,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralFlatnessPercentile {
    pub fn new(fft_window: usize, pct_window: usize) -> Self {
        let w = pct_window.max(10);
        Self {
            inner: SpectralFlatness::new(fft_window),
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.inner.is_ready()
    }
    pub fn feed(&mut self, c: f64) -> f64 {
        let x = self.inner.feed(c);
        self.buf[self.idx] = x;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut le = 0usize;
            for i in 0..len {
                if self.buf[i] <= x {
                    le += 1;
                }
            }
            self.value = (le as f64) / (len as f64);
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

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`SpectralFlatnessPercentile`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SflatpConfig {
    /// FFT window fed to the inner SpectralFlatness.
    pub fft_window: Param<usize>,
    /// Rolling window over which the percentile rank is computed.
    pub pct_window: Param<usize>,
}

impl Indicator for SpectralFlatnessPercentile {
    const ID: IndicatorId = IndicatorId::Sflatp;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Sflat, &[IndicatorOutputId::Sflat])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Sflatp)];
    type Config = SflatpConfig;
    type Runtime = SpectralFlatnessPercentile;

    fn create(cfg: SflatpConfig) -> SpectralFlatnessPercentile {
        SpectralFlatnessPercentile::new(cfg.fft_window.resolved(), cfg.pct_window.resolved())
    }

    fn source_fields(cfg: &SflatpConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for SflatpConfig {
    fn defaults() -> Self {
        SflatpConfig { fft_window: Param::Solo(128), pct_window: Param::Solo(100) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // fft_window / pct_window: Class A usize → auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
}


impl Render for SpectralFlatnessPercentile {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Sflatp, "Spectral Flatness %", Color::hex(0x009688))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_flatness_pct_creation() {
        let sfp = SpectralFlatnessPercentile::new(64, 30);
        assert!(!sfp.is_ready());
        assert_eq!(sfp.value(), 0.0);
        assert_eq!(sfp.window(), 30);
    }

    #[test]
    fn test_spectral_flatness_pct_warmup() {
        let mut sfp = SpectralFlatnessPercentile::new(64, 30);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sfp.feed(price);
        }
        assert!(sfp.is_ready());
    }

    #[test]
    fn test_spectral_flatness_pct_range() {
        let mut sfp = SpectralFlatnessPercentile::new(64, 30);
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sfp.feed(price);
            assert!(value >= 0.0 && value <= 1.0, "Percentile should be in [0, 1], got {}", value);
        }
    }

    #[test]
    fn test_spectral_flatness_pct_reset() {
        let mut sfp = SpectralFlatnessPercentile::new(64, 30);
        for i in 0..100 {
            sfp.feed(100.0 + i as f64);
        }
        sfp.reset();
        assert!(!sfp.is_ready());
        assert_eq!(sfp.value(), 0.0);
    }

    /// Factory resolves close (not wild 9999 high) and feeds flatness percentile.
    #[test]
    fn factory_feeds_resolved_sflatp() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralFlatnessPercentile as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Sflatp(cfg).build_solo().unwrap();
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
