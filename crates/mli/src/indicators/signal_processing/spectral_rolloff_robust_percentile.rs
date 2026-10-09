// Robust percentile of Spectral Rolloff using winsorized window

use crate::indicators::signal_processing::spectral_rolloff::SpectralRolloff;
use crate::indicators::utils::math::percentile::quickselect_nth;

#[derive(Debug, Clone)]
pub struct SpectralRolloffRobustPercentile {
    inner: SpectralRolloff,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralRolloffRobustPercentile {
    pub fn new(fft_window: usize, pct_window: usize, rolloff_percent: f64) -> Self {
        let w = pct_window.max(50);
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
        let x = self.inner.feed(c);
        self.buf[self.idx] = x;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            // Optimized: Use O(n) quickselect for 5th and 95th percentiles instead of O(n log n) full sort
            let mut w = self.buf.clone();
            let k = (0.05 * (self.window as f64)) as usize;

            // Get 5th percentile using quickselect
            let low = quickselect_nth(&mut w.clone(), k);
            // Get 95th percentile using quickselect
            let high = quickselect_nth(&mut w, self.window - 1 - k);

            let mut less = 0usize;
            for &t in &self.buf {
                let tt = t.max(low).min(high);
                if tt <= x {
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

/// Typed config for [`SpectralRolloffRobustPercentile`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SrollrpConfig {
    /// FFT window fed to the inner SpectralRolloff.
    pub fft_window: Param<usize>,
    /// Rolling window over which the winsorized percentile is computed.
    pub pct_window: Param<usize>,
    /// Rolloff threshold fraction (e.g. 0.85).
    pub rolloff_percent: Param<f64>,
}

impl Indicator for SpectralRolloffRobustPercentile {
    const ID: IndicatorId = IndicatorId::Srollrp;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Sroll, &[IndicatorOutputId::Sroll])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Srollrp)];
    type Config = SrollrpConfig;
    type Runtime = SpectralRolloffRobustPercentile;

    fn create(cfg: SrollrpConfig) -> SpectralRolloffRobustPercentile {
        SpectralRolloffRobustPercentile::new(cfg.fft_window.resolved(), cfg.pct_window.resolved(), cfg.rolloff_percent.resolved())
    }

    fn source_fields(cfg: &SrollrpConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for SrollrpConfig {
    fn defaults() -> Self {
        SrollrpConfig { fft_window: Param::Solo(32), pct_window: Param::Solo(100), rolloff_percent: Param::Solo(0.85) }
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


impl Render for SpectralRolloffRobustPercentile {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Srollrp, "Spectral Rolloff RP", Color::hex(0x3F51B5))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_rolloff_robust_creation() {
        let srr = SpectralRolloffRobustPercentile::new(64, 50, 0.85);
        assert!(!srr.is_ready());
        assert_eq!(srr.value(), 50.0);
        assert_eq!(srr.window(), 50);
    }

    #[test]
    fn test_spectral_rolloff_robust_warmup() {
        let mut srr = SpectralRolloffRobustPercentile::new(64, 50, 0.85);
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            srr.feed(price);
        }
        assert!(srr.is_ready());
    }

    #[test]
    fn test_spectral_rolloff_robust_range() {
        let mut srr = SpectralRolloffRobustPercentile::new(64, 50, 0.85);
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = srr.feed(price);
            assert!(value >= 0.0 && value <= 100.0, "Percentile should be in [0, 100], got {}", value);
        }
    }

    #[test]
    fn test_spectral_rolloff_robust_reset() {
        let mut srr = SpectralRolloffRobustPercentile::new(64, 50, 0.85);
        for i in 0..120 {
            srr.feed(100.0 + i as f64);
        }
        srr.reset();
        assert!(!srr.is_ready());
        assert_eq!(srr.value(), 50.0);
    }

    /// Factory resolves close (not wild 9999 high) and feeds robust rolloff percentile.
    #[test]
    fn factory_feeds_resolved_srollrp() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralRolloffRobustPercentile as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Srollrp(cfg).build_solo().unwrap();
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
