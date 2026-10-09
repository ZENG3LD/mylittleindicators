// Percentile of Spectral Rolloff over rolling window

use crate::indicators::signal_processing::spectral_rolloff::SpectralRolloff;

#[derive(Debug, Clone)]
pub struct SpectralRolloffPercentile {
    inner: SpectralRolloff,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralRolloffPercentile {
    pub fn new(fft_window: usize, pct_window: usize, rolloff_percent: f64) -> Self {
        let w = pct_window.clamp(30, 4096);
        Self {
            inner: SpectralRolloff::new(fft_window, rolloff_percent),
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
        let r = self.inner.feed(c);
        self.buf[self.idx] = r;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut less = 0usize;
            for &x in &self.buf {
                if x <= r {
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

impl Default for SpectralRolloffPercentile {
    /// Factory default: fft_window=32, pct_window=100, rolloff_percent=0.85.
    fn default() -> Self {
        Self::new(32, 100, 0.85)
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

/// Typed config for [`SpectralRolloffPercentile`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SrollpConfig {
    /// FFT window fed to the inner SpectralRolloff.
    pub fft_window: Param<usize>,
    /// Rolling window over which the percentile rank is computed.
    pub pct_window: Param<usize>,
    /// Rolloff threshold fraction (e.g. 0.85).
    pub rolloff_percent: Param<f64>,
}

impl Indicator for SpectralRolloffPercentile {
    const ID: IndicatorId = IndicatorId::Srollp;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Sroll, &[IndicatorOutputId::Sroll])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Srollp)];
    type Config = SrollpConfig;
    type Runtime = SpectralRolloffPercentile;

    fn create(cfg: SrollpConfig) -> SpectralRolloffPercentile {
        SpectralRolloffPercentile::new(cfg.fft_window.resolved(), cfg.pct_window.resolved(), cfg.rolloff_percent.resolved())
    }

    fn source_fields(cfg: &SrollpConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for SrollpConfig {
    fn defaults() -> Self {
        SrollpConfig { fft_window: Param::Solo(32), pct_window: Param::Solo(100), rolloff_percent: Param::Solo(0.85) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // fft_window / pct_window → range(2,4048,1)
        // rolloff_percent: Class G rolloff percentile — meaningful range 0.70..0.99
        s.rolloff_percent = Param::many(sweep_f64(0.70, 0.99, 0.01));
        s
    }
}


impl Render for SpectralRolloffPercentile {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Srollp, "Spectral Rolloff %", Color::hex(0x9C27B0))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_rolloff_pct_creation() {
        let srp = SpectralRolloffPercentile::new(64, 50, 0.85);
        assert!(!srp.is_ready());
        assert_eq!(srp.value(), 50.0);
        assert_eq!(srp.window(), 50);
    }

    #[test]
    fn test_spectral_rolloff_pct_warmup() {
        let mut srp = SpectralRolloffPercentile::new(64, 50, 0.85);
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            srp.feed(price);
        }
        assert!(srp.is_ready());
    }

    #[test]
    fn test_spectral_rolloff_pct_range() {
        let mut srp = SpectralRolloffPercentile::new(64, 50, 0.85);
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = srp.feed(price);
            assert!(value >= 0.0 && value <= 100.0, "Percentile should be in [0, 100], got {}", value);
        }
    }

    #[test]
    fn test_spectral_rolloff_pct_reset() {
        let mut srp = SpectralRolloffPercentile::new(64, 50, 0.85);
        for i in 0..120 {
            srp.feed(100.0 + i as f64);
        }
        srp.reset();
        assert!(!srp.is_ready());
        assert_eq!(srp.value(), 50.0);
    }

    /// Factory resolves close (not wild 9999 high) and feeds rolloff percentile.
    #[test]
    fn factory_feeds_resolved_srollp() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralRolloffPercentile as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Srollp(cfg).build_solo().unwrap();
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
