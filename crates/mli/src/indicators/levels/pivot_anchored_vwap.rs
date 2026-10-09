// Pivot-Anchored VWAP: anchor at last detected pivot high/low (simple HH/LL)

#[derive(Clone, Debug)]
pub struct PivotAnchoredVwap {
    lookback: usize,
    highs: Vec<f64>,
    lows: Vec<f64>,
    idx: usize,
    filled: bool,
    // accumulators since last pivot
    pv: f64,
    vv: f64,
    value: f64,
}

impl PivotAnchoredVwap {
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback: lookback.max(3),
            highs: vec![0.0; lookback.max(3)],
            lows: vec![0.0; lookback.max(3)],
            idx: 0,
            filled: false,
            pv: 0.0,
            vv: 0.0,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.highs.fill(0.0);
        self.lows.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.pv = 0.0;
        self.vv = 0.0;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    fn recompute(&mut self, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        // detect simple pivot: if current high is new max or current low new min over window-1 previous
        self.highs[self.idx] = high;
        self.lows[self.idx] = low;
        let len = self.lookback;
        let mut prev_max = f64::MIN;
        let mut prev_min = f64::MAX;
        for k in 1..len {
            let i = (self.idx + len - k) % len;
            prev_max = prev_max.max(self.highs[i]);
            prev_min = prev_min.min(self.lows[i]);
        }
        let is_pivot_high = high >= prev_max && self.filled;
        let is_pivot_low = low <= prev_min && self.filled;

        if is_pivot_high || is_pivot_low {
            self.pv = 0.0;
            self.vv = 0.0;
        }
        // accumulate
        self.pv += close * volume;
        self.vv += volume.max(1e-12);
        self.value = self.pv / self.vv;

        // advance ring
        self.idx = (self.idx + 1) % len;
        if self.idx == 0 {
            self.filled = true;
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

}

impl Default for PivotAnchoredVwap {
    /// Factory default: `new(20)` — lookback = `unwrap_or(20)`.
    fn default() -> Self {
        Self::new(20)
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

/// Own config for [`PivotAnchoredVwap`] — lookback period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PivavwapConfig {
    pub period: Param<usize>,
}

impl Indicator for PivotAnchoredVwap {
    const ID: IndicatorId = IndicatorId::Pivavwap;
    /// No family — VWAP anchored at detected pivot high/low, a position-level producer.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads H/L for pivot detection + close*volume for VWAP accumulation.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close, OhlcvField::Volume]));
    /// O(lookback) per bar — rescans the ring to find prev max/min. Two period-deep Vecs.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Pivavwap)];
    type Config = PivavwapConfig;
    type Runtime = PivotAnchoredVwap;

    fn create(cfg: PivavwapConfig) -> PivotAnchoredVwap {
        PivotAnchoredVwap::new(cfg.period.resolved())
    }
}

impl PivotAnchoredVwap {
    /// Feed resolved `[high, low, close, volume]` lanes.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let volume = lanes[3];
        self.recompute(high, low, close, volume)
    }
}

impl crate::contract::Config for PivavwapConfig {
    fn defaults() -> Self {
        PivavwapConfig { period: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // period: Class A lookback for pivot detection — auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for PivotAnchoredVwap {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(
                IndicatorOutputId::Pivavwap,
                "Pivot AVWAP",
                Color::hex(0x2196F3),
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
    fn factory_feeds_resolved_lanes() {
        let mut f = IndicatorOrder::Pivavwap(<<PivotAnchoredVwap as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=25 {
            let base = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: base + 1.0,
                low: base - 1.0,
                close: base,
                volume: 1000.0,
            });
        }
        assert!(f.is_ready());
        let v = f.primary();
        assert!(v > 0.0, "pivot avwap must be positive, got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pivot_anchored_vwap_creation() {
        let pav = PivotAnchoredVwap::new(10);
        assert!(!pav.is_ready());
        assert_eq!(pav.value(), 0.0);
    }

    #[test]
    fn test_pivot_anchored_vwap_warmup() {
        let mut pav = PivotAnchoredVwap::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pav.recompute(price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pav.is_ready());
    }

    #[test]
    fn test_pivot_anchored_vwap_positive() {
        let mut pav = PivotAnchoredVwap::new(10);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            let value = pav.recompute(price + 1.0, price - 1.0, price, 1000.0);
            assert!(value > 0.0, "VWAP should be positive");
        }
    }

    #[test]
    fn test_pivot_anchored_vwap_reset() {
        let mut pav = PivotAnchoredVwap::new(10);
        for _i in 0..15 {
            pav.recompute(101.0, 99.0, 100.0, 1000.0);
        }
        pav.reset();
        assert!(!pav.is_ready());
        assert_eq!(pav.value(), 0.0);
    }
}
