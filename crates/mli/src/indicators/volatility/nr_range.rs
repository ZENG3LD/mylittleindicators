// Normalized Range and NR flags (NR4/NR7) plus percentile of range

#[derive(Debug, Clone)]
pub struct NrRange {
    window: usize,
    ranges: Vec<f64>,
    idx: usize,
    filled: bool,
    // outputs
    pub range: f64,
    pub percentile: f64, // [0..1]
    pub is_nr4: bool,
    pub is_nr7: bool,
}

impl NrRange {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            ranges: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            range: 0.0,
            percentile: 0.0,
            is_nr4: false,
            is_nr7: false,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ranges.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.range = 0.0;
        self.percentile = 0.0;
        self.is_nr4 = false;
        self.is_nr7 = false;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed the resolved input lanes — `[high, low]` (the factory resolves the fixed
    /// High/Low slice). Knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, bool, bool) {
        let high = lanes[0];
        let low = lanes[1];
        let current_range = (high - low).max(0.0);
        self.range = current_range;
        self.ranges[self.idx] = current_range;

        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        // Percentile of current range among window
        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut count_le = 0usize;
            for i in 0..len {
                if self.ranges[i] <= current_range {
                    count_le += 1;
                }
            }
            self.percentile = count_le as f64 / len as f64;
        } else {
            self.percentile = 0.0;
        }

        // NR4 / NR7 flags
        self.is_nr4 = false;
        self.is_nr7 = false;
        if len >= 4 {
            let start = len - 4;
            let mut min_r = f64::INFINITY;
            for i in start..len {
                if self.ranges[i] < min_r {
                    min_r = self.ranges[i];
                }
            }
            self.is_nr4 = current_range <= min_r + 1e-12;
        }
        if len >= 7 {
            let start = len - 7;
            let mut min_r = f64::INFINITY;
            for i in start..len {
                if self.ranges[i] < min_r {
                    min_r = self.ranges[i];
                }
            }
            self.is_nr7 = current_range <= min_r + 1e-12;
        }

        (self.range, self.percentile, self.is_nr4, self.is_nr7)
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.range
    }

}

impl Default for NrRange {
    fn default() -> Self {
        Self::new(20)
    }
}

// -- contract -----------------------------------------------------------------

use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct NrConfig {
    pub period: Param<usize>,
}

impl Indicator for NrRange {
    const ID: IndicatorId = IndicatorId::Nr;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Nr)];
    type Config = NrConfig;
    type Runtime = NrRange;

    fn create(cfg: NrConfig) -> NrRange {
        NrRange::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for NrConfig {
    fn defaults() -> Self {
        NrConfig { period: Param::Solo(7) }
    }
    fn machine_defaults() -> Self {
        // period: Class A usize — auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
}


impl Render for NrRange {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Nr, "Narrow Range", Color::hex(0xFF9800))
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nr_range_creation() {
        let nr = NrRange::new(20);
        assert!(!nr.is_ready());
        assert_eq!(nr.value(), 0.0);
    }

    #[test]
    fn test_nr_range_warmup() {
        let mut nr = NrRange::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            nr.feed(&[price + 1.0, price - 1.0]);
        }
        assert!(nr.is_ready());
    }

    #[test]
    fn test_nr_range_percentile() {
        let mut nr = NrRange::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            let (_, pct, _, _) = nr.feed(&[price + 2.0, price - 2.0]);
            assert!(pct >= 0.0 && pct <= 1.0);
        }
    }

    #[test]
    fn test_nr_range_reset() {
        let mut nr = NrRange::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            nr.feed(&[price + 1.0, price - 1.0]);
        }
        nr.reset();
        assert!(!nr.is_ready());
        assert_eq!(nr.value(), 0.0);
    }
}
