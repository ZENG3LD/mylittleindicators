// Z-score of Spectral Slope over rolling window

use crate::indicators::signal_processing::spectral_slope::SpectralSlope;

#[derive(Debug, Clone)]
pub struct SpectralSlopeZscore {
    inner: SpectralSlope,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralSlopeZscore {
    pub fn new(fft_window: usize, z_window: usize) -> Self {
        let w = z_window.clamp(20, 2048);
        Self {
            inner: SpectralSlope::new(fft_window),
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
        let slope = self.inner.feed(c);
        let n = self.window;
        self.buf[self.idx] = slope;
        self.idx = (self.idx + 1) % n;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut mean = 0.0;
            for i in 0..n {
                mean += self.buf[i];
            }
            mean /= n as f64;
            let mut var = 0.0;
            for i in 0..n {
                let d = self.buf[i] - mean;
                var += d * d;
            }
            let std = (var / (n as f64)).sqrt().max(1e-9);
            self.value = (slope - mean) / std;
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

impl Default for SpectralSlopeZscore {
    /// Factory default: fft_window=256, z_window=256.
    fn default() -> Self {
        Self::new(256, 256)
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

/// Typed config for [`SpectralSlopeZscore`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SslopezConfig {
    /// FFT window fed to the inner SpectralSlope.
    pub fft_window: Param<usize>,
    /// Rolling window over which the z-score is computed.
    pub z_window: Param<usize>,
}

impl Indicator for SpectralSlopeZscore {
    const ID: IndicatorId = IndicatorId::Sslopez;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Sslope, &[IndicatorOutputId::Sslope])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Sslopez)];
    type Config = SslopezConfig;
    type Runtime = SpectralSlopeZscore;

    fn create(cfg: SslopezConfig) -> SpectralSlopeZscore {
        SpectralSlopeZscore::new(cfg.fft_window.resolved(), cfg.z_window.resolved())
    }

    fn source_fields(cfg: &SslopezConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for SslopezConfig {
    fn defaults() -> Self {
        SslopezConfig { fft_window: Param::Solo(256), z_window: Param::Solo(256) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // fft_window / z_window: Class A usize → auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
}


impl Render for SpectralSlopeZscore {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Sslopez, "Spectral Slope Z", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_slope_zscore_creation() {
        let ssz = SpectralSlopeZscore::new(64, 30);
        assert!(!ssz.is_ready());
        assert_eq!(ssz.value(), 0.0);
        assert_eq!(ssz.window(), 30);
    }

    #[test]
    fn test_spectral_slope_zscore_warmup() {
        let mut ssz = SpectralSlopeZscore::new(64, 30);
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ssz.feed(price);
        }
        assert!(ssz.is_ready());
    }

    #[test]
    fn test_spectral_slope_zscore_finite() {
        let mut ssz = SpectralSlopeZscore::new(64, 30);
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ssz.feed(price);
            assert!(value.is_finite(), "Z-score should be finite");
        }
    }

    #[test]
    fn test_spectral_slope_zscore_reset() {
        let mut ssz = SpectralSlopeZscore::new(64, 30);
        for i in 0..200 {
            ssz.feed(100.0 + i as f64);
        }
        ssz.reset();
        assert!(!ssz.is_ready());
        assert_eq!(ssz.value(), 0.0);
    }

    /// Factory resolves close (not wild 9999 high) and feeds slope z-score.
    #[test]
    fn factory_feeds_resolved_sslopez() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralSlopeZscore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Sslopez(cfg).build_solo().unwrap();
        for i in 0..300 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
