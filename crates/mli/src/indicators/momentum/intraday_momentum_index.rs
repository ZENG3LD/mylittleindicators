// Intraday Momentum Index (IMI)
// Placeholder implementation: windowed ratio of up-momentum to total momentum over period

use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct IntradayMomentumIndex {
    period: usize,
    ups: VecDeque<f64>,
    downs: VecDeque<f64>,
    sum_up: f64,
    sum_down: f64,
    value: f64,
}

impl IntradayMomentumIndex {
    pub fn new(period: usize) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            ups: VecDeque::with_capacity(p + 1),
            downs: VecDeque::with_capacity(p + 1),
            sum_up: 0.0,
            sum_down: 0.0,
            value: 50.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.ups.clear();
        self.downs.clear();
        self.sum_up = 0.0;
        self.sum_down = 0.0;
        self.value = 50.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ups.len() >= self.period
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn period(&self) -> usize {
        self.period
    }

    /// Feed resolved `[open, high, low, close]` lanes — contract input (SOURCE = KlineSlice[O,H,L,C]).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let o = lanes[0];
        let h = lanes[1];
        let l = lanes[2];
        let c = lanes[3];
        let range = (h - l).abs().max(1e-12);
        let delta = c - o;
        let up = if delta > 0.0 { delta / range } else { 0.0 };
        let down = if delta < 0.0 { (-delta) / range } else { 0.0 };

        self.ups.push_back(up);
        self.sum_up += up;
        self.downs.push_back(down);
        self.sum_down += down;
        if self.ups.len() > self.period {
            if let Some(x) = self.ups.pop_front() {
                self.sum_up -= x;
            }
        }
        if self.downs.len() > self.period {
            if let Some(x) = self.downs.pop_front() {
                self.sum_down -= x;
            }
        }

        let denom = (self.sum_up + self.sum_down).max(1e-12);
        self.value = 100.0 * (self.sum_up / denom);
        self.value
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`IntradayMomentumIndex`] — period only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ImiConfig {
    pub period: Param<usize>,
}

impl Indicator for IntradayMomentumIndex {
    const ID: IndicatorId = IndicatorId::Imi;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// IMI uses Open, High, Low, Close — all fixed.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Imi)];
    /// O(1): VecDeque running sums, period-bounded deques.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque), Store::window(StoreKind::Deque)],
    );

    type Config = ImiConfig;
    type Runtime = IntradayMomentumIndex;

    fn create(cfg: ImiConfig) -> IntradayMomentumIndex {
        IntradayMomentumIndex::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for ImiConfig {
    fn defaults() -> Self {
        ImiConfig { period: Param::Solo(14) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
}


impl Render for IntradayMomentumIndex {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::Imi,
                "IMI",
                Color::hex(0x009688),
                1.5,
            ))
            .bounds(0.0, 100.0)
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
        let cfg = <<IntradayMomentumIndex as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.period.resolved(), 14);
        let mut f = IndicatorOrder::Imi(cfg)
            .build_solo()
            .unwrap();
        for i in 1..=30 {
            let open = 100.0 + i as f64;
            let close = open + 2.0;
            f.feed(0, MarketSample::Bar {
                open,
                high: close + 0.5,
                low: open - 0.5,
                close,
                volume: 9999.0, // not used
            });
        }
        let v = f.primary();
        assert!(v >= 0.0 && v <= 100.0, "IMI must be in [0,100], got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_imi_creation() {
        let imi = IntradayMomentumIndex::new(14);
        assert!(!imi.is_ready());
        assert_eq!(imi.value(), 50.0); // default neutral
        assert_eq!(imi.period(), 14);
    }

    #[test]
    fn test_imi_uptrend() {
        let mut imi = IntradayMomentumIndex::new(10);
        for i in 1..=30 {
            let open = 100.0 + i as f64;
            let close = open + 2.0;
            imi.feed(&[open, close + 0.5, open - 0.5, close]);
        }
        assert!(imi.is_ready());
        assert!(imi.value() > 50.0, "IMI should be > 50 in uptrend, got {}", imi.value());
    }

    #[test]
    fn test_imi_downtrend() {
        let mut imi = IntradayMomentumIndex::new(10);
        for i in 1..=30 {
            let open = 200.0 - i as f64;
            let close = open - 2.0;
            imi.feed(&[open, open + 0.5, close - 0.5, close]);
        }
        assert!(imi.is_ready());
        assert!(imi.value() < 50.0, "IMI should be < 50 in downtrend, got {}", imi.value());
    }

    #[test]
    fn test_imi_range() {
        let mut imi = IntradayMomentumIndex::new(10);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = imi.feed(&[price - 1.0, price + 2.0, price - 2.0, price + 1.0]);
            assert!(value >= 0.0 && value <= 100.0, "IMI should be in [0, 100], got {}", value);
        }
    }

    #[test]
    fn test_imi_reset() {
        let mut imi = IntradayMomentumIndex::new(10);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            imi.feed(&[price, price + 2.0, price - 2.0, price + 1.0]);
        }
        assert!(imi.is_ready());
        imi.reset();
        assert!(!imi.is_ready());
        assert_eq!(imi.value(), 50.0);
    }
}

impl Default for IntradayMomentumIndex {
    fn default() -> Self {
        Self::new(14)
    }
}
