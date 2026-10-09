// Kaufman's Efficiency Ratio (ER) on close: direction/volatility over window


#[derive(Debug, Clone)]
pub struct EfficiencyRatio {
    window: usize,
    closes: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    pub value: f64,
}

impl EfficiencyRatio {
    pub fn new(window: usize) -> Self {
        let w = window.max(2);
        Self {
            window: w,
            closes: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.last_close = None;
        self.closes.fill(0.0);
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, close: f64) -> f64 {
        let n = self.window;
        self.closes[self.idx] = close;
        self.idx = (self.idx + 1) % n;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }

        if self.filled {
            let oldest_idx = self.idx;
            let newest_idx = (self.idx + n - 1) % n;
            let direction = (self.closes[newest_idx] - self.closes[oldest_idx]).abs();
            let mut volatility = 0.0;
            for i in 0..n - 1 {
                let a = (self.idx + i) % n;
                let b = (self.idx + i + 1) % n;
                volatility += (self.closes[b] - self.closes[a]).abs();
            }
            self.value = if volatility > 0.0 {
                (direction / volatility).clamp(0.0, 1.0)
            } else {
                0.0
            };
        }
        self.value
    }

}

impl Default for EfficiencyRatio {
    fn default() -> Self {
        Self::new(14)
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Own config for [`EfficiencyRatio`] — period only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct EfficiencyRatioConfig {
    pub period: Param<usize>,
}

impl Indicator for EfficiencyRatio {
    const ID: IndicatorId = IndicatorId::TrEr;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(N): scans the full window on each update to sum |step| changes.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::TrEr)];
    type Config = EfficiencyRatioConfig;
    type Runtime = EfficiencyRatio;

    fn create(cfg: EfficiencyRatioConfig) -> EfficiencyRatio {
        EfficiencyRatio::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for EfficiencyRatioConfig {
    fn defaults() -> Self {
        EfficiencyRatioConfig { period: Param::Solo(10) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A — auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
}


impl Render for EfficiencyRatio {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::TrEr, "Trend/ER", Color::hex(0x009688))
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efficiency_ratio_creation() {
        let er = EfficiencyRatio::new(10);
        assert!(!er.is_ready());
        assert_eq!(er.value, 0.0);
    }

    #[test]
    fn test_efficiency_ratio_warmup() {
        let mut er = EfficiencyRatio::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            er.feed(price);
        }
        assert!(er.is_ready());
    }

    #[test]
    fn test_efficiency_ratio_range() {
        let mut er = EfficiencyRatio::new(10);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            let value = er.feed(price);
            assert!(value >= 0.0 && value <= 1.0, "ER should be in [0, 1]");
        }
    }

    #[test]
    fn test_efficiency_ratio_reset() {
        let mut er = EfficiencyRatio::new(10);
        for i in 0..15 {
            er.feed(100.0 + i as f64);
        }
        er.reset();
        assert!(!er.is_ready());
        assert_eq!(er.value, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_scalar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::TrEr(<<EfficiencyRatio as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..15 {
            let close = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: close, high: close + 9999.0, low: close - 9999.0,
                close, volume: 0.0,
            });
        }
        let v = f.primary();
        assert!(v >= 0.0 && v <= 1.0, "ER should be in [0,1], got {v}");
    }
}
