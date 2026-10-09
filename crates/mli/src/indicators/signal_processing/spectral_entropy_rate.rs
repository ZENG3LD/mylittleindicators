// Spectral Entropy Rate: smoothed first difference of spectral entropy

use crate::indicators::signal_processing::spectral_entropy::SpectralEntropy;

#[derive(Debug, Clone)]
pub struct SpectralEntropyRate {
    inner: SpectralEntropy,
    alpha: f64, // EMA smoothing
    prev_entropy: f64,
    is_prev_set: bool,
    pub value: f64, // smoothed dH
}

impl SpectralEntropyRate {
    pub fn new(window: usize, smoothing_alpha: f64) -> Self {
        Self {
            inner: SpectralEntropy::new(window),
            alpha: smoothing_alpha.clamp(0.01, 1.0),
            prev_entropy: 0.0,
            is_prev_set: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.prev_entropy = 0.0;
        self.is_prev_set = false;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready() && self.is_prev_set
    }

    pub fn feed(&mut self, c: f64) -> f64 {
        let h_now = self.inner.feed(c);
        if !self.is_prev_set {
            self.prev_entropy = h_now;
            self.is_prev_set = true;
            return self.value;
        }
        let diff = h_now - self.prev_entropy;
        self.prev_entropy = h_now;
        // EMA smoothing of rate
        self.value = self.alpha * diff + (1.0 - self.alpha) * self.value;
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }
}

impl Default for SpectralEntropyRate {
    /// Factory default: window=128, smoothing_alpha=0.2.
    fn default() -> Self {
        Self::new(128, 0.2)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity,
};
use crate::contract::axis::sweep_f64;
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`SpectralEntropyRate`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SentrConfig {
    /// FFT window fed to the inner SpectralEntropy.
    pub fft_window: Param<usize>,
    /// EMA smoothing factor for the rate (0.01–1.0).
    pub alpha: Param<f64>,
}

impl Indicator for SpectralEntropyRate {
    const ID: IndicatorId = IndicatorId::Sentr;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Sent, &[IndicatorOutputId::Sent])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Sentr)];
    type Config = SentrConfig;
    type Runtime = SpectralEntropyRate;

    fn create(cfg: SentrConfig) -> SpectralEntropyRate {
        SpectralEntropyRate::new(cfg.fft_window.resolved(), cfg.alpha.resolved())
    }

    fn source_fields(cfg: &SentrConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for SentrConfig {
    fn defaults() -> Self {
        SentrConfig { fft_window: Param::Solo(128), alpha: Param::Solo(0.2) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // fft_window → range(2,4048,1)
        s.alpha = Param::many(sweep_f64(0.01, 0.99, 0.01)); // Class E alpha/decay
        s
    }
}


impl Render for SpectralEntropyRate {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Sentr, "Spectral Entropy Rate", Color::hex(0xE91E63))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_entropy_rate_creation() {
        let ser = SpectralEntropyRate::new(64, 0.2);
        assert!(!ser.is_ready());
        assert_eq!(ser.value(), 0.0);
        assert!((ser.alpha() - 0.2).abs() < 1e-9);
    }

    #[test]
    fn test_spectral_entropy_rate_warmup() {
        let mut ser = SpectralEntropyRate::new(64, 0.2);
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ser.feed(price);
        }
        assert!(ser.is_ready());
    }

    #[test]
    fn test_spectral_entropy_rate_finite() {
        let mut ser = SpectralEntropyRate::new(64, 0.2);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ser.feed(price);
            assert!(value.is_finite(), "Entropy rate should be finite");
        }
    }

    #[test]
    fn test_spectral_entropy_rate_reset() {
        let mut ser = SpectralEntropyRate::new(64, 0.2);
        for i in 0..80 {
            ser.feed(100.0 + i as f64);
        }
        ser.reset();
        assert!(!ser.is_ready());
        assert_eq!(ser.value(), 0.0);
    }

    /// Factory resolves close (not wild 9999 high) and feeds the entropy rate.
    #[test]
    fn factory_feeds_resolved_sentr() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralEntropyRate as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Sentr(cfg).build_solo().unwrap();
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
