// Spectral Rolloff: frequency where cumulative power reaches target fraction (e.g., 85%)

use crate::indicators::signal_processing::fft::FastFourierTransform;

#[derive(Debug, Clone)]
pub struct SpectralRolloff {
    window: usize,
    target: f64,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64, // frequency in [0, Nyquist]
}

impl SpectralRolloff {
    pub fn new(window: usize, target_fraction: f64) -> Self {
        let w = window.clamp(16, 256);
        let t = target_fraction.clamp(0.01, 0.99);
        Self {
            window: w,
            target: t,
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
            let mut total = 0.0;
            for i in 0..fd.power_spectrum.len() {
                total += fd.power_spectrum[i];
            }
            if total > 0.0 {
                let mut cum = 0.0;
                let mut f = 0.0;
                for i in 0..fd.power_spectrum.len() {
                    cum += fd.power_spectrum[i];
                    if cum / total >= self.target {
                        f = if i < fd.frequencies.len() {
                            fd.frequencies[i]
                        } else {
                            0.0
                        };
                        break;
                    }
                }
                self.value = f;
            } else {
                self.value = 0.0;
            }
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

impl Default for SpectralRolloff {
    /// Factory default: window=128, target_fraction=0.85.
    fn default() -> Self {
        Self::new(128, 0.85)
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

/// Typed config for [`SpectralRolloff`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SrollConfig {
    pub window: Param<usize>,
    pub target: Param<f64>,
}

impl Indicator for SpectralRolloff {
    const ID: IndicatorId = IndicatorId::Sroll;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Fft, &[IndicatorOutputId::Fft])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Sroll)];
    type Config = SrollConfig;
    type Runtime = SpectralRolloff;

    fn create(cfg: SrollConfig) -> SpectralRolloff {
        SpectralRolloff::new(cfg.window.resolved(), cfg.target.resolved())
    }

    fn source_fields(cfg: &SrollConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for SrollConfig {
    fn defaults() -> Self {
        SrollConfig { window: Param::Solo(128), target: Param::Solo(0.85) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // window → range(2,4048,1)
        // target: Class G rolloff percentile — meaningful range 0.70..0.99
        s.target = Param::many(sweep_f64(0.70, 0.99, 0.01));
        s
    }
}


impl Render for SpectralRolloff {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Sroll, "Spectral Rolloff", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

// Transitional bridge: un-contracted composites still call update_bar; delegates to the
// contracted feed (default source = close). Removed when each caller is Port-wired.
impl SpectralRolloff {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_rolloff_creation() {
        let sr = SpectralRolloff::new(64, 0.85);
        assert!(!sr.is_ready());
        assert_eq!(sr.value(), 0.0);
        assert_eq!(sr.window(), 64);
    }

    #[test]
    fn test_spectral_rolloff_warmup() {
        let mut sr = SpectralRolloff::new(64, 0.85);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sr.feed(price);
        }
        assert!(sr.is_ready());
    }

    #[test]
    fn test_spectral_rolloff_finite() {
        let mut sr = SpectralRolloff::new(64, 0.85);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sr.feed(price);
            assert!(value.is_finite(), "Rolloff should be finite");
        }
    }

    #[test]
    fn test_spectral_rolloff_reset() {
        let mut sr = SpectralRolloff::new(64, 0.85);
        for i in 0..70 {
            sr.feed(100.0 + i as f64);
        }
        sr.reset();
        assert!(!sr.is_ready());
        assert_eq!(sr.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_sroll() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralRolloff as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Sroll(cfg).build_solo().unwrap();
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
