// Rolling entropy of spectral entropy (volatility of entropy)

use crate::indicators::signal_processing::spectral_entropy::SpectralEntropy;

#[derive(Debug, Clone)]
pub struct SpectralEntropyOfEntropy {
    inner: SpectralEntropy,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralEntropyOfEntropy {
    pub fn new(fft_window: usize, window: usize) -> Self {
        let w = window.max(20);
        Self {
            inner: SpectralEntropy::new(fft_window),
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
        let e = self.inner.feed(c);
        self.buf[self.idx] = e;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut m = 0.0;
            for &x in &self.buf {
                m += x;
            }
            m /= self.window as f64;
            let mut s = 0.0;
            for &x in &self.buf {
                let d = x - m;
                s += d * d;
            }
            self.value = (s / (self.window as f64)).sqrt();
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

/// Typed config for [`SpectralEntropyOfEntropy`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SententConfig {
    /// FFT window fed to the inner SpectralEntropy.
    pub fft_window: Param<usize>,
    /// Rolling window over which the std-dev of entropy is computed.
    pub window: Param<usize>,
}

impl Indicator for SpectralEntropyOfEntropy {
    const ID: IndicatorId = IndicatorId::Sentent;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Sent, &[IndicatorOutputId::Sent])],
    };
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Sentent)];
    type Config = SententConfig;
    type Runtime = SpectralEntropyOfEntropy;

    fn create(cfg: SententConfig) -> SpectralEntropyOfEntropy {
        SpectralEntropyOfEntropy::new(cfg.fft_window.resolved(), cfg.window.resolved())
    }

    fn source_fields(cfg: &SententConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for SententConfig {
    fn defaults() -> Self {
        SententConfig { fft_window: Param::Solo(64), window: Param::Solo(30) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // fft_window / window: Class A usize → auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
}


impl Render for SpectralEntropyOfEntropy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Sentent, "Spectral EoE", Color::hex(0xE91E63))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_eoe_creation() {
        let eoe = SpectralEntropyOfEntropy::new(64, 30);
        assert!(!eoe.is_ready());
        assert_eq!(eoe.value(), 0.0);
        assert_eq!(eoe.window(), 30);
    }

    #[test]
    fn test_spectral_eoe_warmup() {
        let mut eoe = SpectralEntropyOfEntropy::new(64, 30);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            eoe.feed(price);
        }
        assert!(eoe.is_ready());
    }

    #[test]
    fn test_spectral_eoe_finite() {
        let mut eoe = SpectralEntropyOfEntropy::new(64, 30);
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = eoe.feed(price);
            assert!(value.is_finite(), "EoE should be finite");
        }
    }

    #[test]
    fn test_spectral_eoe_reset() {
        let mut eoe = SpectralEntropyOfEntropy::new(64, 30);
        for i in 0..100 {
            eoe.feed(100.0 + i as f64);
        }
        eoe.reset();
        assert!(!eoe.is_ready());
        assert_eq!(eoe.value(), 0.0);
    }

    /// Factory resolves close (not wild 9999 high) and feeds entropy-of-entropy.
    #[test]
    fn factory_feeds_resolved_sentent() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralEntropyOfEntropy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Sentent(cfg).build_solo().unwrap();
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
