// Spectral Crest Factor: max magnitude / RMS magnitude of spectrum

use crate::indicators::signal_processing::fft::FastFourierTransform;

#[derive(Debug, Clone)]
pub struct SpectralCrest {
    window: usize,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralCrest {
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
            let mut mean = 0.0;
            for i in 0..n {
                mean += self.buf[i];
            }
            mean /= n as f64;
            for i in 0..n {
                self.fft.update(self.buf[(self.idx + i) % n] - mean);
            }
            let fd = self.fft.frequency_domain();
            let mut max_mag = 0.0;
            let mut sum_pow = 0.0;
            let mut count = 0.0;
            for i in 0..fd.magnitudes.len() {
                let m = fd.magnitudes[i];
                if m > max_mag {
                    max_mag = m;
                }
            }
            for i in 0..fd.power_spectrum.len() {
                sum_pow += fd.power_spectrum[i];
                count += 1.0;
            }
            let rms = if count > 0.0 {
                (sum_pow / count).sqrt()
            } else {
                0.0
            };
            self.value = if rms > 0.0 { max_mag / rms } else { 0.0 };
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

impl Default for SpectralCrest {
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

/// Own config for [`SpectralCrest`] — period-only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ScrestConfig {
    pub period: Param<usize>,
}

impl Indicator for SpectralCrest {
    const ID: IndicatorId = IndicatorId::Screst;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Fft, &[IndicatorOutputId::Fft])],
    };
    const OUTPUTS: &'static [Output] = &[Output::ratio(IndicatorOutputId::Screst)];
    type Config = ScrestConfig;
    type Runtime = SpectralCrest;

    fn create(cfg: ScrestConfig) -> SpectralCrest {
        SpectralCrest::new(cfg.period.resolved())
    }

    fn source_fields(cfg: &ScrestConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for ScrestConfig {
    fn defaults() -> Self {
        ScrestConfig { period: Param::Solo(128) }
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


impl Render for SpectralCrest {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Screst, "Spectral Crest", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

// Transitional bridge: un-contracted composites still call update_bar; delegates to the
// contracted feed (default source = close). Removed when each caller is Port-wired.
impl SpectralCrest {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_crest_creation() {
        let sc = SpectralCrest::new(64);
        assert!(!sc.is_ready());
        assert_eq!(sc.value(), 0.0);
        assert_eq!(sc.window(), 64);
    }

    #[test]
    fn test_spectral_crest_warmup() {
        let mut sc = SpectralCrest::new(64);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sc.feed(price);
        }
        assert!(sc.is_ready());
    }

    #[test]
    fn test_spectral_crest_finite() {
        let mut sc = SpectralCrest::new(64);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sc.feed(price);
            assert!(value.is_finite(), "Crest should be finite");
        }
    }

    #[test]
    fn test_spectral_crest_reset() {
        let mut sc = SpectralCrest::new(64);
        for i in 0..70 {
            sc.feed(100.0 + i as f64);
        }
        sc.reset();
        assert!(!sc.is_ready());
        assert_eq!(sc.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_screst() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralCrest as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Screst(cfg).build_solo().unwrap();
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
