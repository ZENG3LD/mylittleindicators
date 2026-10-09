// Range Percentile over rolling window using (High-Low)

#[derive(Debug, Clone)]
pub struct RangePercentile {
    window: usize,
    buffer: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
    percentile: f64,
}

impl RangePercentile {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            buffer: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            value: 0.0,
            percentile: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
        self.percentile = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed the resolved input lanes — `[high, low]` (the factory resolves the fixed
    /// High/Low slice). Knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        self.value = (high - low).max(0.0);
        self.buffer[self.idx] = self.value;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut count_le = 0usize;
            for i in 0..len {
                if self.buffer[i] <= self.value {
                    count_le += 1;
                }
            }
            self.percentile = count_le as f64 / len as f64;
        } else {
            self.percentile = 0.0;
        }
        (self.value, self.percentile)
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    #[inline]
    pub fn percentile(&self) -> f64 {
        self.percentile
    }

}

impl Default for RangePercentile {
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
pub struct RpConfig {
    pub period: Param<usize>,
}

impl Indicator for RangePercentile {
    const ID: IndicatorId = IndicatorId::Rp;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Rp)];
    type Config = RpConfig;
    type Runtime = RangePercentile;

    fn create(cfg: RpConfig) -> RangePercentile {
        RangePercentile::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for RpConfig {
    fn defaults() -> Self {
        RpConfig { period: Param::Solo(50) }
    }
    fn machine_defaults() -> Self {
        // period: Class A usize — auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
}


impl Render for RangePercentile {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Rp, "Range Percentile", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_percentile_creation() {
        let rp = RangePercentile::new(20);
        assert!(!rp.is_ready());
        assert_eq!(rp.value(), 0.0);
    }

    #[test]
    fn test_range_percentile_warmup() {
        let mut rp = RangePercentile::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rp.feed(&[price + 1.0, price - 1.0]);
        }
        assert!(rp.is_ready());
    }

    #[test]
    fn test_range_percentile_range() {
        let mut rp = RangePercentile::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            let (_, pct) = rp.feed(&[price + 2.0, price - 2.0]);
            assert!(pct >= 0.0 && pct <= 1.0);
        }
    }

    #[test]
    fn test_range_percentile_reset() {
        let mut rp = RangePercentile::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            rp.feed(&[price + 1.0, price - 1.0]);
        }
        rp.reset();
        assert!(!rp.is_ready());
        assert_eq!(rp.value(), 0.0);
    }
}
