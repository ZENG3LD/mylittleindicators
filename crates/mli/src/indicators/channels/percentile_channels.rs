// Percentile Channels: upper/lower as rolling percentiles of close price.
// The contract path operates on a pre-extracted scalar (Source flavor, default Close).

use crate::indicators::utils::math::percentile::quickselect_nth;

/// Source field selector for un-contracted embedders that call the legacy 4-arg constructor.
/// All variants map to close in the current scalar-feed implementation.
// transitional bridge for un-contracted embedders; remove when they convert
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PercentileBasis {
    Close,
    Typical,
    HighLow,
}

#[derive(Debug, Clone)]
pub struct PercentileChannels {
    window: usize,
    upper_q: f64,
    lower_q: f64,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

impl Default for PercentileChannels {
    fn default() -> Self {
        Self::new(14, 0.25, 0.75)
    }
}

impl PercentileChannels {
    /// 4-arg overload for un-contracted embedders that supply a `PercentileBasis`.
    /// The basis is ignored; close is always used by the scalar feed path.
    // transitional bridge for un-contracted embedders; remove when they convert
    pub fn new_with_basis(window: usize, _basis: PercentileBasis, lower_q: f64, upper_q: f64) -> Self {
        Self::new(window, lower_q, upper_q)
    }

    pub fn new(window: usize, lower_q: f64, upper_q: f64) -> Self {
        Self {
            window,
            upper_q: upper_q.clamp(0.0, 1.0),
            lower_q: lower_q.clamp(0.0, 1.0),
            buf: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }


    pub fn upper(&self) -> f64 { self.upper }
    pub fn middle(&self) -> f64 { self.middle }
    pub fn lower(&self) -> f64 { self.lower }

    /// Feed one pre-extracted scalar price — the contract feed path.
    pub fn feed(&mut self, price: f64) -> (f64, f64, f64) {
        self.buf[self.idx] = price;
        self.idx = (self.idx + 1) % self.window.max(1);
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }
        let len = if self.filled { self.window } else { self.idx };
        if len == 0 {
            return (self.lower, self.middle, self.upper);
        }
        self.lower = percentile_of(&self.buf, len, self.lower_q);
        self.upper = percentile_of(&self.buf, len, self.upper_q);
        self.middle = 0.5 * (self.lower + self.upper);
        (self.lower, self.middle, self.upper)
    }

}

// ---- Indicator contract ----

use crate::contract::{Param, sweep_f64};
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`PercentileChannels`]. Defaults: 14 bars, Q25/Q75 bounds.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PercentilechConfig {
    pub period: Param<usize>,
    pub lower_q: Param<f64>,
    pub upper_q: Param<f64>,
}

impl Indicator for PercentileChannels {
    const ID: IndicatorId = IndicatorId::Percentilech;
    /// Channel family — rolling-percentile price channel.
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::Field { default: crate::engine::ohlcv_field::OhlcvField::Close });
    /// O(period) per bar: quickselect rescan. One period-deep Vec buffer.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::PercentilechUpper),
        Output::price(IndicatorOutputId::PercentilechMiddle),
        Output::price(IndicatorOutputId::PercentilechLower),
    ];
    type Config = PercentilechConfig;
    type Runtime = PercentileChannels;

    fn create(cfg: PercentilechConfig) -> PercentileChannels {
        PercentileChannels::new(cfg.period.resolved(), cfg.lower_q.resolved(), cfg.upper_q.resolved())
    }
}

impl crate::contract::Config for PercentilechConfig {
    fn defaults() -> Self {
        PercentilechConfig {
            period: Param::Solo(14),
            lower_q: Param::Solo(0.25),
            upper_q: Param::Solo(0.75),
        }
    }
    fn valid_params(&self) -> Result<(), String> {
        let lo = self.lower_q.resolved();
        let hi = self.upper_q.resolved();
        if lo >= hi {
            return Err(format!("lower_q({lo}) >= upper_q({hi})"));
        }
        Ok(())
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1)
        // lower_q / upper_q: Class D ratio/quantile — BOTH were swept over the SAME
        // sweep_f64(0.0,1.0,0.05), so both resolved to the same min (0.0), failing this
        // config's OWN `valid_params` (lower_q < upper_q) at the min corner (2026-07-03 fix).
        // Split into disjoint sub-ranges of the original [0.0, 1.0] quantile bounds so
        // `resolved()` stays ordered.
        s.lower_q = Param::many(sweep_f64(0.0, 0.45, 0.05));
        s.upper_q = Param::many(sweep_f64(0.5, 1.0, 0.05));
        s
    }
}


impl Render for PercentileChannels {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::PercentilechUpper, "Upper", Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::PercentilechMiddle, "Middle", Color::hex(0x9E9E9E), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::PercentilechLower, "Lower", Color::hex(0x4CAF50), 1.0))
            .precision(4)
            .build()
    }
}

#[inline]
fn percentile_of(buf: &[f64], len: usize, q: f64) -> f64 {
    // 🚀 O(n) quickselect instead of O(n log n) sorting
    if len == 0 {
        return 0.0;
    }
    let mut tmp = Vec::with_capacity(len);
    tmp.extend_from_slice(&buf[..len]);
    let pos = (q.clamp(0.0, 1.0) * (len as f64 - 1.0)).round() as usize;
    quickselect_nth(&mut tmp, pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_channels_creation() {
        let pc = PercentileChannels::new(20, 0.1, 0.9);
        assert!(!pc.is_ready());
        assert_eq!(pc.upper, 0.0);
        assert_eq!(pc.lower, 0.0);
    }

    #[test]
    fn test_percentile_channels_warmup() {
        let mut pc = PercentileChannels::new(20, 0.1, 0.9);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pc.feed(price);
        }
        assert!(pc.is_ready());
    }

    #[test]
    fn test_percentile_channels_values() {
        let mut pc = PercentileChannels::new(20, 0.1, 0.9);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            pc.feed(price);
        }
        assert!(pc.upper >= pc.middle);
        assert!(pc.middle >= pc.lower);
    }

    #[test]
    fn test_percentile_channels_reset() {
        let mut pc = PercentileChannels::new(20, 0.1, 0.9);
        for i in 0..25 {
            pc.feed(100.0 + i as f64);
        }
        pc.reset();
        assert!(!pc.is_ready());
        assert_eq!(pc.upper, 0.0);
        assert_eq!(pc.lower, 0.0);
    }

    /// Factory resolves configurable source (default close); wild high/low are ignored.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<PercentileChannels as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Percentilech(cfg).build_solo().unwrap();
        for i in 0..20usize {
            let close = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 9999.0, close, volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite(), "upper should be finite, got {}", f.primary());
    }
}
