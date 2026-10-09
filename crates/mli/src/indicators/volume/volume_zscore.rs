// Volume Z-Score over rolling window

#[derive(Debug, Clone)]
pub struct VolumeZscore {
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    sum: f64,
    sumsq: f64,
    z: f64,
}

impl VolumeZscore {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            buf: vec![0.0; window.max(2)],
            idx: 0,
            filled: false,
            sum: 0.0,
            sumsq: 0.0,
            z: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.sum = 0.0;
        self.sumsq = 0.0;
        self.z = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed resolved lanes `[volume]`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let volume = lanes[0];
        let old = self.buf[self.idx];
        self.buf[self.idx] = volume;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        self.sum += volume - old;
        self.sumsq += volume * volume - old * old;
        let n = if self.filled {
            self.window as f64
        } else {
            self.idx as f64
        };
        if n >= 2.0 {
            let mean = self.sum / n;
            let var = (self.sumsq / n) - mean * mean;
            let std = if var > 0.0 { var.sqrt() } else { 0.0 };
            self.z = if std > 1e-12 {
                (volume - mean) / std
            } else {
                0.0
            };
        }
        self.z
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_zscore_creation() {
        let vz = VolumeZscore::new(20);
        assert!(!vz.is_ready());
        assert_eq!(vz.value(), 0.0);
    }

    #[test]
    fn test_volume_zscore_warmup() {
        let mut vz = VolumeZscore::new(20);
        for i in 0..25 {
            let volume = 1000.0 + (i as f64 * 0.1).sin() * 100.0;
            vz.feed(&[volume]);
        }
        assert!(vz.is_ready());
    }

    #[test]
    fn test_volume_zscore_values() {
        let mut vz = VolumeZscore::new(20);
        for i in 0..30 {
            let volume = 1000.0 + i as f64 * 50.0;
            let value = vz.feed(&[volume]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_volume_zscore_reset() {
        let mut vz = VolumeZscore::new(20);
        for i in 0..30 {
            vz.feed(&[1000.0 + i as f64 * 10.0]);
        }
        vz.reset();
        assert!(!vz.is_ready());
        assert_eq!(vz.value(), 0.0);
    }
}

impl Default for VolumeZscore {
    fn default() -> Self {
        Self::new(100)
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity, Store, StoreKind};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec, ReferenceLine};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`VolumeZscore`] — rolling window for the z-score calculation.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VzConfig {
    pub period: Param<usize>,
}

impl Indicator for VolumeZscore {
    const ID: IndicatorId = IndicatorId::Vz;
    /// Standalone volume z-score — not a pluggable oscillator family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed volume lane only — pure volume statistic, no price.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::Volume]));
    const NEEDS_VOLUME: bool = true;
    /// O(period) per bar — maintains a rolling ring buffer for online mean/var.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Vz)];
    type Config = VzConfig;
    type Runtime = VolumeZscore;

    fn create(cfg: VzConfig) -> VolumeZscore {
        VolumeZscore::new(cfg.period.resolved().max(2))
    }
}

impl crate::contract::Config for VzConfig {
    fn defaults() -> Self {
        VzConfig { period: Param::Solo(100) }
    }
    fn machine_defaults() -> Self {
        // period: Class A period/window — auto gives range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VolumeZscore {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vz, "Vol Z", Color::hex(0xFF9800))
            .reference_line(ReferenceLine::new(0.0, Color::hex(0x9E9E9E)))
            .reference_line(ReferenceLine::new(2.0, Color::hex(0xFF5722)))
            .reference_line(ReferenceLine::new(-2.0, Color::hex(0x4CAF50)))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_lanes() {
        let mut f = IndicatorOrder::Vz(<<VolumeZscore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Feed uniform volume — after warmup z-score should be near 0
        for _ in 0..105 {
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: 9999.0, volume: 1000.0 });
        }
        let v = f.read(IndicatorOutputId::Vz);
        assert!(v.is_finite(), "VZ must be finite, got {v}");
        // Uniform volume → std ≈ 0 → z ≈ 0
        assert!(v.abs() < 1e-6, "uniform volume should give z≈0, got {v}");
    }
}
