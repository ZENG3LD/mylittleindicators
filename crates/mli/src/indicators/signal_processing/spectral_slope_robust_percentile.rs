// Robust percentile of Spectral Slope using winsorized window

use crate::indicators::signal_processing::spectral_slope::SpectralSlope;
use crate::indicators::utils::math::percentile::quickselect_nth;

#[derive(Debug, Clone)]
pub struct SpectralSlopeRobustPercentile {
    inner: SpectralSlope,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralSlopeRobustPercentile {
    pub fn new(fft_window: usize, pct_window: usize) -> Self {
        let w = pct_window.max(50);
        Self {
            inner: SpectralSlope::new(fft_window),
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
            // Optimized: Use O(n) quickselect for 5th and 95th percentiles instead of O(n log n) full sort
            let mut w: Vec<f64> = self.buf.clone();
            let k = (0.05 * (self.window as f64)) as usize;

            // Get 5th percentile using quickselect
            let low = quickselect_nth(&mut w.clone(), k);
            // Get 95th percentile using quickselect
            let high = quickselect_nth(&mut w, self.window - 1 - k);

            let mut count = 0usize;
            let mut less = 0usize;
            for &t in &self.buf {
                let tt = t.max(low).min(high);
                if tt <= x {
                    less += 1;
                }
                count += 1;
            }
            self.value = 100.0 * (less as f64) / (count as f64);
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

impl Default for SpectralSlopeRobustPercentile {
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

/// Typed config for [`SpectralSlopeRobustPercentile`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SsloperpConfig {
    /// FFT window fed to the inner SpectralSlope.
    pub fft_window: Param<usize>,
    /// Rolling window over which the winsorized percentile is computed.
    pub pct_window: Param<usize>,
}

impl Indicator for SpectralSlopeRobustPercentile {
    const ID: IndicatorId = IndicatorId::Ssloperp;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Sslope, &[IndicatorOutputId::Sslope])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Ssloperp)];
    type Config = SsloperpConfig;
    type Runtime = SpectralSlopeRobustPercentile;

    fn create(cfg: SsloperpConfig) -> SpectralSlopeRobustPercentile {
        SpectralSlopeRobustPercentile::new(cfg.fft_window.resolved(), cfg.pct_window.resolved())
    }

    fn source_fields(cfg: &SsloperpConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for SsloperpConfig {
    fn defaults() -> Self {
        SsloperpConfig { fft_window: Param::Solo(32), pct_window: Param::Solo(100) }
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


impl Render for SpectralSlopeRobustPercentile {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Ssloperp, "Spectral Slope RP", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_slope_robust_pct_creation() {
        let ssr = SpectralSlopeRobustPercentile::new(64, 60);
        assert!(!ssr.is_ready());
        assert_eq!(ssr.value(), 50.0);
        assert_eq!(ssr.window(), 60);
    }

    #[test]
    fn test_spectral_slope_robust_pct_warmup() {
        let mut ssr = SpectralSlopeRobustPercentile::new(64, 60);
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ssr.feed(price);
        }
        assert!(ssr.is_ready());
    }

    #[test]
    fn test_spectral_slope_robust_pct_range() {
        let mut ssr = SpectralSlopeRobustPercentile::new(64, 60);
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ssr.feed(price);
            assert!(value >= 0.0 && value <= 100.0, "Percentile should be in [0, 100], got {}", value);
        }
    }

    #[test]
    fn test_spectral_slope_robust_pct_reset() {
        let mut ssr = SpectralSlopeRobustPercentile::new(64, 60);
        for i in 0..200 {
            ssr.feed(100.0 + i as f64);
        }
        ssr.reset();
        assert!(!ssr.is_ready());
        assert_eq!(ssr.value(), 50.0);
    }

    /// Factory resolves close (not wild 9999 high) and feeds robust slope percentile.
    #[test]
    fn factory_feeds_resolved_ssloperp() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralSlopeRobustPercentile as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Ssloperp(cfg).build_solo().unwrap();
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
