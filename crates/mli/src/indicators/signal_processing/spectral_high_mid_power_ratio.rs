// Ratio of spectral power: high band / mid band

use crate::indicators::signal_processing::fft::FastFourierTransform;

#[derive(Debug, Clone)]
pub struct SpectralHighMidPowerRatio {
    window: usize,
    low_cut: f64,
    high_cut: f64,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralHighMidPowerRatio {
    pub fn new(window: usize, low_cut_fraction: f64, high_cut_fraction: f64) -> Self {
        let w = window.clamp(32, 1024);
        let lc = low_cut_fraction.clamp(1e-6, 0.49);
        let hc = high_cut_fraction.clamp(lc + 1e-6, 0.499);
        Self {
            window: w,
            low_cut: lc,
            high_cut: hc,
            fft: FastFourierTransform::new(w, 1.0),
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
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
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
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
            let mut mid = 0.0;
            let mut high = 0.0;
            for i in 0..fd.power_spectrum.len() {
                if i >= fd.frequencies.len() {
                    break;
                }
                let f = fd.frequencies[i];
                let p = fd.power_spectrum[i].max(1e-18);
                if f <= self.low_cut {
                    continue;
                } else if f <= self.high_cut {
                    mid += p;
                } else {
                    high += p;
                }
            }
            self.value = if mid > 0.0 { high / mid } else { 0.0 };
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

impl Default for SpectralHighMidPowerRatio {
    /// Factory default: window=256, low_cut=0.1, high_cut=0.25.
    fn default() -> Self {
        Self::new(256, 0.1, 0.25)
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

/// Typed config for [`SpectralHighMidPowerRatio`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ShmprConfig {
    pub window: Param<usize>,
    pub low_cut: Param<f64>,
    pub high_cut: Param<f64>,
}

impl Indicator for SpectralHighMidPowerRatio {
    const ID: IndicatorId = IndicatorId::Shmpr;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Fft, &[IndicatorOutputId::Fft])],
    };
    const OUTPUTS: &'static [Output] = &[Output::ratio(IndicatorOutputId::Shmpr)];
    type Config = ShmprConfig;
    type Runtime = SpectralHighMidPowerRatio;

    fn create(cfg: ShmprConfig) -> SpectralHighMidPowerRatio {
        SpectralHighMidPowerRatio::new(cfg.window.resolved(), cfg.low_cut.resolved(), cfg.high_cut.resolved())
    }

    fn source_fields(cfg: &ShmprConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for ShmprConfig {
    fn defaults() -> Self {
        ShmprConfig { window: Param::Solo(256), low_cut: Param::Solo(0.1), high_cut: Param::Solo(0.25) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // window → range(2,4048,1)
        // low_cut / high_cut: Class G Nyquist fraction 0..0.5
        s.low_cut = Param::many(sweep_f64(0.01, 0.49, 0.01));
        s.high_cut = Param::many(sweep_f64(0.01, 0.49, 0.01));
        s
    }
}


impl Render for SpectralHighMidPowerRatio {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Shmpr, "High/Mid MPR", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_hm_ratio_creation() {
        let shm = SpectralHighMidPowerRatio::new(64, 0.15, 0.35);
        assert!(!shm.is_ready());
        assert_eq!(shm.value(), 0.0);
        assert_eq!(shm.window(), 64);
    }

    #[test]
    fn test_spectral_hm_ratio_warmup() {
        let mut shm = SpectralHighMidPowerRatio::new(64, 0.15, 0.35);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            shm.feed(price);
        }
        assert!(shm.is_ready());
    }

    #[test]
    fn test_spectral_hm_ratio_finite() {
        let mut shm = SpectralHighMidPowerRatio::new(64, 0.15, 0.35);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = shm.feed(price);
            assert!(value.is_finite(), "Ratio should be finite");
        }
    }

    #[test]
    fn test_spectral_hm_ratio_reset() {
        let mut shm = SpectralHighMidPowerRatio::new(64, 0.15, 0.35);
        for i in 0..70 {
            shm.feed(100.0 + i as f64);
        }
        shm.reset();
        assert!(!shm.is_ready());
        assert_eq!(shm.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_shmpr() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SpectralHighMidPowerRatio as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Shmpr(cfg).build_solo().unwrap();
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
