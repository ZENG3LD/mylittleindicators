// Percentile of Spectral Crest over rolling window

use crate::indicators::signal_processing::spectral_crest::SpectralCrest;

#[derive(Debug, Clone)]
pub struct SpectralCrestPercentile {
    inner: SpectralCrest,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralCrestPercentile {
    pub fn new(fft_window: usize, pct_window: usize) -> Self {
        let w = pct_window.max(30);
        Self {
            inner: SpectralCrest::new(fft_window),
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 50.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 50.0;
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
        if self.filled {
            let mut less = 0usize;
            for &t in &self.buf {
                if t <= x {
                    less += 1;
                }
            }
            self.value = 100.0 * (less as f64) / (self.window as f64);
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

impl Default for SpectralCrestPercentile {
    /// Factory default: fft_window=32, pct_window=100.
    fn default() -> Self {
        Self::new(32, 100)
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

/// Typed config for [`SpectralCrestPercentile`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ScrestpConfig {
    /// FFT window fed to the inner SpectralCrest.
    pub fft_window: Param<usize>,
    /// Rolling window over which the percentile rank is computed.
    pub pct_window: Param<usize>,
}

impl Indicator for SpectralCrestPercentile {
    const ID: IndicatorId = IndicatorId::Screstp;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Screst, &[IndicatorOutputId::Screst])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Screstp)];
    type Config = ScrestpConfig;
    type Runtime = SpectralCrestPercentile;

    fn create(cfg: ScrestpConfig) -> SpectralCrestPercentile {
        SpectralCrestPercentile::new(cfg.fft_window.resolved(), cfg.pct_window.resolved())
    }

    fn source_fields(cfg: &ScrestpConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for ScrestpConfig {
    fn defaults() -> Self {
        ScrestpConfig { fft_window: Param::Solo(32), pct_window: Param::Solo(100) }
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


impl Render for SpectralCrestPercentile {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Screstp, "Spectral Crest %", Color::hex(0x9C27B0))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_crest_percentile_creation() {
        let scp = SpectralCrestPercentile::new(64, 50);
        assert!(!scp.is_ready());
        assert_eq!(scp.value(), 50.0);
        assert_eq!(scp.window(), 50);
    }

    #[test]
    fn test_spectral_crest_percentile_warmup() {
        let mut scp = SpectralCrestPercentile::new(64, 50);
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            scp.feed(price);
        }
        assert!(scp.is_ready());
    }

    #[test]
    fn test_spectral_crest_percentile_range() {
        let mut scp = SpectralCrestPercentile::new(64, 50);
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = scp.feed(price);
            assert!(value >= 0.0 && value <= 100.0, "Percentile should be in [0, 100], got {}", value);
        }
    }

    #[test]
    fn test_spectral_crest_percentile_reset() {
        let mut scp = SpectralCrestPercentile::new(64, 50);
        for i in 0..120 {
            scp.feed(100.0 + i as f64);
        }
        scp.reset();
        assert!(!scp.is_ready());
        assert_eq!(scp.value(), 50.0);
    }

    /// Factory resolves close (not wild 9999 high) and feeds it through the inner crest.
    #[test]
    fn factory_feeds_resolved_screstp() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralCrestPercentile as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Screstp(cfg).build_solo().unwrap();
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
