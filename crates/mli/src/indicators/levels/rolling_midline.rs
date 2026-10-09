// Rolling Midline: average of High/Low over window

#[derive(Clone, Debug)]
pub struct RollingMidline {
    window: usize,
    sum: f64,
    buffer: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl RollingMidline {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            sum: 0.0,
            buffer: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.sum = 0.0;
        self.buffer.fill(0.0);
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

}

impl Default for RollingMidline {
    /// Factory default: `new(14)` — uses shared `period = unwrap_or(14).clamp(2, 512)`.
    fn default() -> Self {
        Self::new(14)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderOutput, RenderSpec, SourceAxis,
    Store, StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`RollingMidline`] — window period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RmidConfig {
    pub period: Param<usize>,
}

impl Indicator for RollingMidline {
    const ID: IndicatorId = IndicatorId::Rmid;
    /// No family — structural midline level, not a pluggable oscillator/MA.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L — midline is (high + low) / 2, not configurable.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// O(1): running sum — add new mid, subtract evicted. One period-deep Vec buffer.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Rmid)];
    type Config = RmidConfig;
    type Runtime = RollingMidline;

    fn create(cfg: RmidConfig) -> RollingMidline {
        RollingMidline::new(cfg.period.resolved())
    }
}

impl RollingMidline {
    /// Feed the resolved H/L lanes (`[high, low]`). Replaces `update_bar` for the contract path.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let mid = 0.5 * (high + low);
        self.sum += mid - self.buffer[self.idx];
        self.buffer[self.idx] = mid;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        let denom = if self.filled {
            self.window as f64
        } else {
            self.idx as f64
        };
        self.value = if denom > 0.0 { self.sum / denom } else { 0.0 };
        self.value
    }
}

impl crate::contract::Config for RmidConfig {
    fn defaults() -> Self {
        RmidConfig { period: Param::Solo(14) }
    }
    fn machine_defaults() -> Self {
        // period: Class A period — auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for RollingMidline {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(
                IndicatorOutputId::Rmid,
                "Range Mid",
                Color::hex(0x9E9E9E),
                1.0,
            ))
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
    fn factory_feeds_resolved_hl() {
        let mut f = IndicatorOrder::Rmid(<<RollingMidline as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=20 {
            let base = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: base + 1.0,
                low: base - 1.0,
                close: 9999.0,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        // midline of (high+low)/2 = base, close/open are wild and must NOT affect result
        let v = f.primary();
        assert!(v > 100.0 && v < 122.0, "mid out of range: {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_midline_creation() {
        let rm = RollingMidline::new(20);
        assert!(!rm.is_ready());
        assert_eq!(rm.value(), 0.0);
    }

    #[test]
    fn test_rolling_midline_warmup() {
        let mut rm = RollingMidline::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rm.feed(&[price + 1.0, price - 1.0]);
        }
        assert!(rm.is_ready());
    }

    #[test]
    fn test_rolling_midline_positive() {
        let mut rm = RollingMidline::new(20);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let value = rm.feed(&[price + 2.0, price - 2.0]);
            assert!(value > 0.0, "Midline should be positive");
        }
    }

    #[test]
    fn test_rolling_midline_reset() {
        let mut rm = RollingMidline::new(20);
        for _i in 0..25 {
            rm.feed(&[101.0, 99.0]);
        }
        rm.reset();
        assert!(!rm.is_ready());
        assert_eq!(rm.value(), 0.0);
    }
}
