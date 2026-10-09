// Spectral Entropy using normalized FFT power on rolling window of closes

use crate::indicators::signal_processing::fft::FastFourierTransform;

#[derive(Debug, Clone)]
pub struct SpectralEntropy {
    window: usize,
    fft: FastFourierTransform,
    closes: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl SpectralEntropy {
    pub fn new(window: usize) -> Self {
        let w = window.clamp(16, 256);
        // sampling_rate: 1.0 bar^-1
        Self {
            window: w,
            fft: FastFourierTransform::new(w, 1.0),
            closes: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.closes.fill(0.0);
        self.value = 0.0;
        self.fft.reset();
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.fft.is_ready()
    }

    pub fn feed(&mut self, close: f64) -> f64 {
        let n = self.window;
        self.closes[self.idx] = close;
        self.idx = (self.idx + 1) % n;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }
        // feed normalized demeaned close to FFT
        if self.filled {
            let mut mean = 0.0;
            for i in 0..n {
                mean += self.closes[i];
            }
            mean /= n as f64;
            for i in 0..n {
                self.fft.update(self.closes[(self.idx + i) % n] - mean);
            }
            let fd = self.fft.frequency_domain();
            let mut ps_sum = 0.0;
            for i in 0..fd.power_spectrum.len() {
                ps_sum += fd.power_spectrum[i];
            }
            if ps_sum > 1e-12 {
                // Избегаем деления на очень малые числа
                let mut h = 0.0;
                let n = fd.power_spectrum.len() as f64;
                let epsilon = 1e-15; // Минимальное значение для избежания ln(0)

                for i in 0..fd.power_spectrum.len() {
                    let p = (fd.power_spectrum[i] / ps_sum).max(epsilon);
                    if p > epsilon && p.is_finite() {
                        h -= p * p.ln();
                    }
                }

                // Проверяем на NaN и нормализуем
                if h.is_finite() && h >= 0.0 && n > 1.0 {
                    self.value = (h / n.ln()).clamp(0.0, 1.0);
                } else {
                    self.value = 0.5; // Средняя энтропия при проблемах
                }
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

impl Default for SpectralEntropy {
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

/// Own config for [`SpectralEntropy`] — period-only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SentConfig {
    pub period: Param<usize>,
}

impl Indicator for SpectralEntropy {
    const ID: IndicatorId = IndicatorId::Sent;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Fft, &[IndicatorOutputId::Fft])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Sent)];
    type Config = SentConfig;
    type Runtime = SpectralEntropy;

    fn create(cfg: SentConfig) -> SpectralEntropy {
        SpectralEntropy::new(cfg.period.resolved())
    }

    fn source_fields(cfg: &SentConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for SentConfig {
    fn defaults() -> Self {
        SentConfig { period: Param::Solo(128) }
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


impl Render for SpectralEntropy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Sent, "Spectral Entropy", Color::hex(0xE91E63))
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    }
}

// Transitional bridge: un-contracted composites still call update_bar; delegates to the
// contracted feed (default source = close). Removed when each caller is Port-wired.
impl SpectralEntropy {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_entropy_creation() {
        let se = SpectralEntropy::new(64);
        assert!(!se.is_ready());
        assert_eq!(se.value(), 0.0);
        assert_eq!(se.window(), 64);
    }

    #[test]
    fn test_spectral_entropy_warmup() {
        let mut se = SpectralEntropy::new(64);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            se.feed(price);
        }
        assert!(se.is_ready());
    }

    #[test]
    fn test_spectral_entropy_range() {
        let mut se = SpectralEntropy::new(64);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = se.feed(price);
            if se.is_ready() {
                assert!(value >= 0.0 && value <= 1.0, "Entropy should be in [0, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_spectral_entropy_reset() {
        let mut se = SpectralEntropy::new(64);
        for i in 0..70 {
            se.feed(100.0 + i as f64);
        }
        se.reset();
        assert!(!se.is_ready());
        assert_eq!(se.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_sent() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralEntropy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Sent(cfg).build_solo().unwrap();
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
