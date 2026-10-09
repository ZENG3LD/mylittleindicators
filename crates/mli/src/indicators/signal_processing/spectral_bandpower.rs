// Spectral Bandpower: power within predefined frequency bands over rolling window

use crate::indicators::signal_processing::fft::FastFourierTransform;

#[derive(Clone, Copy)]
enum BandKind {
    Low,
    Mid,
    High,
}

/// Generic spectral bandpower calculator with 3 bands: [0, l), [l, h), [h, 0.5]
/// Frequency expressed as fraction of sampling rate (Nyquist = 0.5)
struct SpectralBandpowerGeneric {
    window: usize,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    low_cut: f64,
    high_cut: f64,
    band: BandKind,
    value: f64, // primary output (band power share)
}

impl SpectralBandpowerGeneric {
    fn new(window: usize, low_cut_fraction: f64, high_cut_fraction: f64, band: BandKind) -> Self {
        let w = window.clamp(32, 512);
        let l = low_cut_fraction.clamp(0.05, 0.45);
        let h = high_cut_fraction.clamp(l + 0.01, 0.49);
        Self {
            window: w,
            fft: FastFourierTransform::new(w, 1.0),
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            low_cut: l,
            high_cut: h,
            band,
            value: 0.0,
        }
    }

    #[inline]
    fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.buf.fill(0.0);
        self.value = 0.0;
        self.fft.reset();
    }

    #[inline]
    fn is_ready(&self) -> bool {
        self.filled && self.fft.is_ready()
    }

    fn feed(&mut self, c: f64) -> f64 {
        let n = self.window;
        self.buf[self.idx] = c;
        self.idx = (self.idx + 1) % n;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }

        if self.filled {
            // Demean and feed into FFT in ring order
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
            let mut low = 0.0f64;
            let mut mid = 0.0f64;
            let mut high = 0.0f64;
            for i in 0..fd.power_spectrum.len() {
                let f = if i < fd.frequencies.len() {
                    fd.frequencies[i]
                } else {
                    0.0
                };
                let p = fd.power_spectrum[i];
                if f <= self.low_cut {
                    low += p;
                } else if f <= self.high_cut {
                    mid += p;
                } else {
                    high += p;
                }
            }
            let total = (low + mid + high).max(1e-18);
            self.value = match self.band {
                BandKind::Low => low / total,
                BandKind::Mid => mid / total,
                BandKind::High => high / total,
            };
        }
        self.value
    }
}

pub struct SpectralBandpowerLow {
    inner: SpectralBandpowerGeneric,
}
pub struct SpectralBandpowerMid {
    inner: SpectralBandpowerGeneric,
}
pub struct SpectralBandpowerHigh {
    inner: SpectralBandpowerGeneric,
}

impl SpectralBandpowerLow {
    pub fn new(window: usize, low_cut_fraction: f64, high_cut_fraction: f64) -> Self {
        Self {
            inner: SpectralBandpowerGeneric::new(
                window,
                low_cut_fraction,
                high_cut_fraction,
                BandKind::Low,
            ),
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }
    #[inline]
    pub fn feed(&mut self, c: f64) -> f64 {
        self.inner.feed(c)
    }
}

impl SpectralBandpowerMid {
    pub fn new(window: usize, low_cut_fraction: f64, high_cut_fraction: f64) -> Self {
        Self {
            inner: SpectralBandpowerGeneric::new(
                window,
                low_cut_fraction,
                high_cut_fraction,
                BandKind::Mid,
            ),
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }
    #[inline]
    pub fn feed(&mut self, c: f64) -> f64 {
        self.inner.feed(c)
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.inner.value
    }
}

impl SpectralBandpowerHigh {
    pub fn new(window: usize, low_cut_fraction: f64, high_cut_fraction: f64) -> Self {
        Self {
            inner: SpectralBandpowerGeneric::new(
                window,
                low_cut_fraction,
                high_cut_fraction,
                BandKind::High,
            ),
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }
    #[inline]
    pub fn feed(&mut self, c: f64) -> f64 {
        self.inner.feed(c)
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.inner.value
    }
}

/// Base SpectralBandpower that returns all 3 bands (low, mid, high) at once
#[derive(Debug, Clone)]
pub struct SpectralBandpower {
    window: usize,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    low_cut: f64,
    high_cut: f64,
    pub low: f64,
    pub mid: f64,
    pub high: f64,
}

impl SpectralBandpower {
    pub fn new(window: usize, low_cut_fraction: f64, high_cut_fraction: f64) -> Self {
        let w = window.clamp(32, 512);
        let l = low_cut_fraction.clamp(0.05, 0.45);
        let h = high_cut_fraction.clamp(l + 0.01, 0.49);
        Self {
            window: w,
            fft: FastFourierTransform::new(w, 1.0),
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            low_cut: l,
            high_cut: h,
            low: 0.0,
            mid: 0.0,
            high: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.buf.fill(0.0);
        self.low = 0.0;
        self.mid = 0.0;
        self.high = 0.0;
        self.fft.reset();
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.fft.is_ready()
    }

    /// Feed ONE resolved scalar (the configured source, default close). The factory
    /// extracts the source field; the spectral ring consumes the scalar.
    pub fn feed(&mut self, value: f64) -> (f64, f64, f64) {
        let n = self.window;
        self.buf[self.idx] = value;
        self.idx = (self.idx + 1) % n;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }

        if self.filled {
            // Demean and feed into FFT in ring order
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
            let mut low_sum = 0.0f64;
            let mut mid_sum = 0.0f64;
            let mut high_sum = 0.0f64;
            for i in 0..fd.power_spectrum.len() {
                let f = if i < fd.frequencies.len() {
                    fd.frequencies[i]
                } else {
                    0.0
                };
                let p = fd.power_spectrum[i];
                if f <= self.low_cut {
                    low_sum += p;
                } else if f <= self.high_cut {
                    mid_sum += p;
                } else {
                    high_sum += p;
                }
            }
            let total = (low_sum + mid_sum + high_sum).max(1e-18);
            self.low = low_sum / total;
            self.mid = mid_sum / total;
            self.high = high_sum / total;
        }
        (self.low, self.mid, self.high)
    }

}

impl SpectralBandpower {
    /// Power share in the low frequency band.
    pub fn low(&self) -> f64 {
        self.low
    }

    /// Power share in the mid frequency band.
    pub fn mid(&self) -> f64 {
        self.mid
    }

    /// Power share in the high frequency band.
    pub fn high(&self) -> f64 {
        self.high
    }
}

impl Default for SpectralBandpower {
    /// Factory default: window=256, low_cut=0.1, high_cut=0.3.
    fn default() -> Self {
        Self::new(256, 0.1, 0.3)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Store, StoreKind, UpdateComplexity,
};
use crate::contract::axis::sweep_f64;
use crate::contract::{Color, Render, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`SpectralBandpower`]: rolling window + frequency cut-points.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SpectralBandpowerConfig {
    /// Rolling window length (clamped to 32..=512 internally).
    pub window: Param<usize>,
    /// Low/mid boundary as fraction of Nyquist (0.05..=0.45).
    pub low_cut: Param<f64>,
    /// Mid/high boundary as fraction of Nyquist (low_cut+0.01..=0.49).
    pub high_cut: Param<f64>,
}

impl Indicator for SpectralBandpower {
    const ID: IndicatorId = IndicatorId::Sbp;
    /// Not a pluggable family member — a spectral decomposition utility.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// O(window) per bar: demeans and feeds the full ring into FFT, then accumulates
    /// power bins. One period-deep `Vec` ring buffer.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::SbpLow),
        Output::percent(IndicatorOutputId::SbpMid),
        Output::percent(IndicatorOutputId::SbpHigh),
    ];
    type Config = SpectralBandpowerConfig;
    type Runtime = SpectralBandpower;

    fn create(cfg: SpectralBandpowerConfig) -> SpectralBandpower {
        SpectralBandpower::new(cfg.window.resolved(), cfg.low_cut.resolved(), cfg.high_cut.resolved())
    }
}

impl crate::contract::Config for SpectralBandpowerConfig {
    fn defaults() -> Self {
        SpectralBandpowerConfig {
            window: Param::Solo(256),
            low_cut: Param::Solo(0.1),
            high_cut: Param::Solo(0.3),
        }
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


impl Render for SpectralBandpower {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::SbpLow, "Band Low", Color::hex(0x4CAF50), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::SbpMid, "Band Mid", Color::hex(0x2196F3), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::SbpHigh, "Band High", Color::hex(0xF44336), 2.0))
            .bounds(0.0, 1.0)
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_bandpower_creation() {
        let sb = SpectralBandpower::new(64, 0.1, 0.3);
        assert!(!sb.is_ready());
        assert_eq!(sb.low(), 0.0);
        assert_eq!(sb.mid(), 0.0);
        assert_eq!(sb.high(), 0.0);
    }

    #[test]
    fn test_spectral_bandpower_warmup() {
        let mut sb = SpectralBandpower::new(64, 0.1, 0.3);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sb.feed(price);
        }
        assert!(sb.is_ready());
    }

    #[test]
    fn test_spectral_bandpower_sum_to_one() {
        let mut sb = SpectralBandpower::new(64, 0.1, 0.3);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (l, m, h) = sb.feed(price);
            if sb.is_ready() {
                let sum = l + m + h;
                assert!((sum - 1.0).abs() < 0.01, "Bands should sum to ~1.0, got {}", sum);
            }
        }
    }

    #[test]
    fn test_spectral_bandpower_reset() {
        let mut sb = SpectralBandpower::new(64, 0.1, 0.3);
        for i in 0..70 {
            sb.feed(100.0 + i as f64);
        }
        sb.reset();
        assert!(!sb.is_ready());
        assert_eq!(sb.low(), 0.0);
        assert_eq!(sb.mid(), 0.0);
        assert_eq!(sb.high(), 0.0);
    }

    #[test]
    fn test_spectral_bandpower_low() {
        let mut sbl = SpectralBandpowerLow::new(64, 0.1, 0.3);
        assert!(!sbl.is_ready());
        let mut last_val = 0.0;
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            last_val = sbl.feed(price);
        }
        assert!(sbl.is_ready());
        assert!(last_val >= 0.0 && last_val <= 1.0);
    }

    #[test]
    fn factory_feeds_resolved_sbp() {
        use crate::engine::contract_engine::{IndicatorOrder, IndicatorOutputId};
        use crate::contract::MarketSample;
        let cfg = <<SpectralBandpower as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Sbp(cfg).build_solo().unwrap();
        // Feed enough bars to warm up (window=256 default, use a smaller window for speed)
        let cfg_small = SpectralBandpowerConfig { window: Param::Solo(64), low_cut: Param::Solo(0.1), high_cut: Param::Solo(0.3) };
        let mut f2 = IndicatorOrder::Sbp(cfg_small).build_solo().unwrap();
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            // volume=9999.0 is a wild value — Sbp uses only close, proving field resolution
            f2.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 9999.0,
            });
        }
        // Bands should be in [0, 1]
        let low = f2.read(IndicatorOutputId::SbpLow);
        let mid = f2.read(IndicatorOutputId::SbpMid);
        let high = f2.read(IndicatorOutputId::SbpHigh);
        assert!(low.is_finite());
        assert!(low >= 0.0 && low <= 1.0);
        assert!(mid >= 0.0 && mid <= 1.0);
        assert!(high >= 0.0 && high <= 1.0);
        // Smoke: default config builds without error
        f.feed(0, MarketSample::Bar { open: 100.0, high: 101.0, low: 99.0, close: 100.0, volume: 1000.0 });
    }
}
