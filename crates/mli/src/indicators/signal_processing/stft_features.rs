// STFT Band Energy Ratio — streaming high-vs-low frequency energy proxy.
//
// True STFT requires FFT (not available without external crates). This implementation
// approximates spectral band energy by comparing variance of the oldest quarter of the
// rolling window (low-frequency proxy) against the newest quarter (high-frequency proxy).
//
// Interpretation:
//   ratio > 1 → recent price changes noisier than slower baseline (high-freq dominant)
//   ratio < 1 → slow trend dominates over short-term noise
//   ratio ≈ 1 → uniform energy across timescales
//
// `window` = total ring-buffer size (4–1024), `split` = fraction divisor (1–8).

#[derive(Debug, Clone)]
pub struct StftBandEnergyRatio {
    window: usize,
    split: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl StftBandEnergyRatio {
    pub fn new(window: usize, split: usize) -> Self {
        Self {
            window: window.clamp(4, 1024),
            split: split.clamp(1, 8),
            buf: Vec::with_capacity(window.clamp(4, 1024)),
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn feed(&mut self, c: f64) -> f64 {
        if self.buf.len() < self.window {
            self.buf.push(c);
            if self.buf.len() == self.window {
                self.filled = true;
            }
        } else {
            self.buf[self.idx] = c;
        }
        self.idx = (self.idx + 1) % self.window;
        if self.is_ready() {
            let part = self.window / self.split.max(1);
            let mut low_var = 0.0;
            let mut high_var = 0.0;
            for i in 0..part {
                let x = self.buf[i] - c;
                low_var += x * x;
            }
            for i in (self.window - part)..self.window {
                let x = self.buf[i] - c;
                high_var += x * x;
            }
            // Add epsilon to avoid division by zero
            // Returns ratio of high-frequency to low-frequency energy
            let eps = 1e-10;
            self.value = (high_var + eps) / (low_var + eps);
        }
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
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`StftBandEnergyRatio`]: ring-buffer window + spectral split divisor.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct StftConfig {
    pub window: Param<usize>,
    pub split: Param<usize>,
}

impl Indicator for StftBandEnergyRatio {
    const ID: IndicatorId = IndicatorId::Stft;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(window): two partial-variance passes over the ring buffer each bar.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[crate::contract::Output::ratio(IndicatorOutputId::Stft)];
    type Config = StftConfig;
    type Runtime = StftBandEnergyRatio;

    fn create(cfg: StftConfig) -> StftBandEnergyRatio {
        StftBandEnergyRatio::new(cfg.window.resolved(), cfg.split.resolved())
    }

    fn source_fields(cfg: &StftConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for StftConfig {
    fn defaults() -> Self {
        StftConfig { window: Param::Solo(64), split: Param::Solo(4) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // window → range(2,4048,1)
        // split: spectral band divisor — small bounded (2..=16); must be < window/2
        s.split = Param::range(2, 16, 1);
        s
    }
}


impl Render for StftBandEnergyRatio {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Stft, "STFT Band Ratio", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

impl Default for StftBandEnergyRatio {
    /// Factory default: window=64, split=4.
    fn default() -> Self {
        Self::new(64, 4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stft_band_energy_creation() {
        let stft = StftBandEnergyRatio::new(64, 4);
        assert!(!stft.is_ready());
        assert_eq!(stft.value(), 0.0);
        assert_eq!(stft.window(), 64);
    }

    #[test]
    fn test_stft_band_energy_warmup() {
        let mut stft = StftBandEnergyRatio::new(64, 4);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            stft.feed(price);
        }
        assert!(stft.is_ready());
    }

    #[test]
    fn test_stft_band_energy_finite() {
        let mut stft = StftBandEnergyRatio::new(64, 4);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = stft.feed(price);
            assert!(value.is_finite(), "STFT value should be finite");
        }
    }

    #[test]
    fn test_stft_band_energy_reset() {
        let mut stft = StftBandEnergyRatio::new(64, 4);
        for i in 0..70 {
            stft.feed(100.0 + i as f64);
        }
        stft.reset();
        assert!(!stft.is_ready());
        assert_eq!(stft.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_stft() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<StftBandEnergyRatio as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Stft(cfg).build_solo().unwrap();
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite());
    }
}
