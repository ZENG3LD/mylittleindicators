// Spectral Energy Ratio: low-band vs high-band power ratio

use crate::indicators::signal_processing::fft::FastFourierTransform;

#[derive(Debug, Clone)]
pub struct SpectralEnergyRatio {
    window: usize,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub ratio: f64, // low/(low+high)
    low_cut: f64,
}

impl SpectralEnergyRatio {
    pub fn new(window: usize, low_cut_fraction: f64) -> Self {
        let w = window.clamp(32, 256);
        Self {
            window: w,
            fft: FastFourierTransform::new(w, 1.0),
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            ratio: 0.0,
            low_cut: low_cut_fraction.clamp(0.05, 0.95),
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.fft.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.ratio = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn feed(&mut self, c: f64) -> f64 {
        self.buf[self.idx] = c;
        self.idx = (self.idx + 1) % self.window;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let n = self.window;
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
            let mut low = 0.0;
            let mut high = 0.0;
            let split_freq = self.low_cut * 0.5;
            for i in 0..fd.power_spectrum.len() {
                let f = if i < fd.frequencies.len() {
                    fd.frequencies[i]
                } else {
                    0.0
                };
                let p = fd.power_spectrum[i];
                if f <= split_freq {
                    low += p;
                } else {
                    high += p;
                }
            }
            let total = low + high;
            self.ratio = if total > 0.0 { low / total } else { 0.0 };
        }
        self.ratio
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.ratio
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

impl Default for SpectralEnergyRatio {
    /// Factory default: window=128, low_cut_fraction=0.1.
    fn default() -> Self {
        Self::new(128, 0.1)
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

/// Typed config for [`SpectralEnergyRatio`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SerConfig {
    pub window: Param<usize>,
    pub low_cut: Param<f64>,
}

impl Indicator for SpectralEnergyRatio {
    const ID: IndicatorId = IndicatorId::Ser;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Fft, &[IndicatorOutputId::Fft])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Ser)];
    type Config = SerConfig;
    type Runtime = SpectralEnergyRatio;

    fn create(cfg: SerConfig) -> SpectralEnergyRatio {
        SpectralEnergyRatio::new(cfg.window.resolved(), cfg.low_cut.resolved())
    }

    fn source_fields(cfg: &SerConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for SerConfig {
    fn defaults() -> Self {
        SerConfig { window: Param::Solo(128), low_cut: Param::Solo(0.1) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // window → range(2,4048,1)
        // low_cut: Class G Nyquist fraction 0..0.5
        s.low_cut = Param::many(sweep_f64(0.01, 0.49, 0.01));
        s
    }
}


impl Render for SpectralEnergyRatio {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Ser, "Spectral Energy Ratio", Color::hex(0x4CAF50))
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    }
}

// Transitional bridge: un-contracted composites still call update_bar; delegates to the
// contracted feed (default source = close). Removed when each caller is Port-wired.
impl SpectralEnergyRatio {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_energy_ratio_creation() {
        let ser = SpectralEnergyRatio::new(64, 0.25);
        assert!(!ser.is_ready());
        assert_eq!(ser.value(), 0.0);
        assert_eq!(ser.window(), 64);
    }

    #[test]
    fn test_spectral_energy_ratio_warmup() {
        let mut ser = SpectralEnergyRatio::new(64, 0.25);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ser.feed(price);
        }
        assert!(ser.is_ready());
    }

    #[test]
    fn test_spectral_energy_ratio_range() {
        let mut ser = SpectralEnergyRatio::new(64, 0.25);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ser.feed(price);
            if ser.is_ready() {
                assert!(value >= 0.0 && value <= 1.0, "Ratio should be in [0, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_spectral_energy_ratio_reset() {
        let mut ser = SpectralEnergyRatio::new(64, 0.25);
        for i in 0..70 {
            ser.feed(100.0 + i as f64);
        }
        ser.reset();
        assert!(!ser.is_ready());
        assert_eq!(ser.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_ser() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralEnergyRatio as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Ser(cfg).build_solo().unwrap();
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
