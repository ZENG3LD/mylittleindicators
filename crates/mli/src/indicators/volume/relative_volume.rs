// Relative Volume (RVOL) and percentile variant

#[derive(Debug, Clone)]
pub struct RelativeVolume {
    window: usize,
    vol_buffer: Vec<f64>,
    idx: usize,
    filled: bool,
    rvol: f64,
    rvol_percentile: f64,
}

impl RelativeVolume {
    pub fn new(window: usize) -> Self {
        Self {
            window,
            vol_buffer: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            rvol: 0.0,
            rvol_percentile: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.vol_buffer.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.rvol = 0.0;
        self.rvol_percentile = 0.0;
    }

    /// Feed resolved lane `[volume]`.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let volume = lanes[0];
        self.vol_buffer[self.idx] = volume.max(0.0);
        self.idx = (self.idx + 1) % self.window.max(1);
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }

        let len = if self.filled { self.window } else { self.idx };
        if len == 0 {
            self.rvol = 0.0;
            self.rvol_percentile = 0.0;
            return (self.rvol, self.rvol_percentile);
        }

        // Mean volume
        let mut sum = 0.0;
        for i in 0..len {
            sum += self.vol_buffer[i];
        }
        let mean = if sum > 0.0 { sum / (len as f64) } else { 0.0 };
        self.rvol = if mean > 0.0 {
            self.vol_buffer[(self.idx + self.window - 1) % self.window] / mean
        } else {
            0.0
        };

        // Percentile rank of current vol
        let curr = self.vol_buffer[(self.idx + self.window - 1) % self.window];
        let mut count = 0usize;
        for i in 0..len {
            if self.vol_buffer[i] <= curr {
                count += 1;
            }
        }
        self.rvol_percentile = (count as f64) / (len as f64);
        (self.rvol, self.rvol_percentile)
    }


    /// Named output: brace `rvol`.
    #[inline]
    pub fn rvol(&self) -> f64 { self.rvol }
    /// Named output: brace `percentile`.
    #[inline]
    pub fn percentile(&self) -> f64 { self.rvol_percentile }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relative_volume_creation() {
        let rvol = RelativeVolume::new(20);
        assert!(!rvol.is_ready());
        assert_eq!(rvol.rvol(), 0.0);
        assert_eq!(rvol.percentile(), 0.0);
    }

    #[test]
    fn test_relative_volume_warmup() {
        let mut rvol = RelativeVolume::new(20);
        for i in 0..25 {
            let volume = 1000.0 + (i as f64 * 50.0);
            rvol.feed(&[volume]);
        }
        assert!(rvol.is_ready());
    }

    #[test]
    fn test_relative_volume_values() {
        let mut rvol = RelativeVolume::new(10);
        for i in 0..15 {
            let volume = 1000.0 + (i as f64 * 0.2).sin() * 500.0;
            let (rv, percentile) = rvol.feed(&[volume]);
            assert!(rv >= 0.0, "RVOL should be non-negative");
            assert!(percentile >= 0.0 && percentile <= 1.0, "Percentile in [0, 1]");
        }
    }

    #[test]
    fn test_relative_volume_reset() {
        let mut rvol = RelativeVolume::new(10);
        for i in 0..15 {
            rvol.feed(&[1000.0 + i as f64 * 100.0]);
        }
        rvol.reset();
        assert!(!rvol.is_ready());
        assert_eq!(rvol.rvol(), 0.0);
        assert_eq!(rvol.percentile(), 0.0);
    }
}

impl Default for RelativeVolume {
    fn default() -> Self {
        Self::new(50)
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity, Store, StoreKind};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`RelativeVolume`] — lookback window size.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RvolConfig {
    pub period: Param<usize>,
}

impl Indicator for RelativeVolume {
    const ID: IndicatorId = IndicatorId::Rvol;
    /// Standalone relative volume measure — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed volume lane only.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::Volume]));
    const NEEDS_VOLUME: bool = true;
    /// O(period) scan per bar — rescans window for mean + percentile rank.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[
        Output::ratio(IndicatorOutputId::RvolRvol),
        Output::percent(IndicatorOutputId::RvolPercentile),
    ];
    type Config = RvolConfig;
    type Runtime = RelativeVolume;

    fn create(cfg: RvolConfig) -> RelativeVolume {
        RelativeVolume::new(cfg.period.resolved().max(1))
    }
}

impl crate::contract::Config for RvolConfig {
    fn defaults() -> Self {
        RvolConfig { period: Param::Solo(50) }
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


impl Render for RelativeVolume {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::RvolRvol, "RVOL", Color::hex(0x00BCD4))
            .precision(4)
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
        let mut f = IndicatorOrder::Rvol(<<RelativeVolume as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Fill window with uniform volume, then one spike
        for _ in 0..50 {
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: 9999.0, volume: 1000.0 });
        }
        // Spike bar: volume 5000 → RVOL should be ~5
        f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: 9999.0, volume: 5000.0 });
        let v = f.read(IndicatorOutputId::RvolRvol);
        assert!(v > 1.0, "spike bar should produce RVOL > 1.0, got {v}");
    }
}
