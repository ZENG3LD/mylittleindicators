// Hurst exponent percentile over rolling window

use crate::indicators::chaos::hurst_exponent::HurstExponent;

#[derive(Debug, Clone)]
pub struct HurstPercentile {
    inner: HurstExponent,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl Default for HurstPercentile {
    fn default() -> Self {
        Self::new(200)
    }
}

impl HurstPercentile {
    pub fn new(window: usize) -> Self {
        let w = window.max(50);
        Self {
            inner: HurstExponent::new(w),
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.5,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.5;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.inner.is_ready()
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn feed(&mut self, c: f64) -> f64 {
        let hval = self.inner.feed(c);
        self.buf[self.idx] = hval;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut cnt = 0usize;
            for i in 0..self.window {
                if self.buf[i] <= hval {
                    cnt += 1;
                }
            }
            self.value = (cnt as f64) / (self.window as f64);
        }
        self.value
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`HurstPercentile`] — period-only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct HurstPctConfig {
    pub period: Param<usize>,
}

impl Indicator for HurstPercentile {
    const ID: IndicatorId = IndicatorId::HurstPct;
    /// Chaos / persistence percentile scorer — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Single configurable close field (fed to the inner HurstExponent).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// Outer is O(window) per bar (rank scan of the rolling buffer). One Vec ring.
    /// Inner Hurst cost is charged via the Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[],
        inner: &[Port::new(IndicatorId::Hurst, &[IndicatorOutputId::Hurst])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::HurstPct)];

    type Config = HurstPctConfig;
    type Runtime = HurstPercentile;

    fn create(cfg: HurstPctConfig) -> HurstPercentile {
        HurstPercentile::new(cfg.period.resolved())
    }

    fn source_fields(cfg: &HurstPctConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for HurstPctConfig {
    fn defaults() -> Self {
        HurstPctConfig { period: Param::Solo(200) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
}


impl Render for HurstPercentile {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::HurstPct, "Hurst %", Color::hex(0x9C27B0))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(50.0, Color::hex(0x9E9E9E)).with_label("Random"))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hurst_percentile_creation() {
        let ind = HurstPercentile::new(50);
        assert!(!ind.is_ready());
        assert_eq!(ind.value, 0.5);
    }

    #[test]
    fn test_hurst_percentile_warmup() {
        let mut ind = HurstPercentile::new(50);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.feed(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_hurst_percentile_values_range() {
        let mut ind = HurstPercentile::new(50);
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let pct = ind.feed(price);
            assert!(pct >= 0.0 && pct <= 1.0);
        }
    }

    #[test]
    fn test_hurst_percentile_reset() {
        let mut ind = HurstPercentile::new(50);
        for i in 0..100 {
            ind.feed(100.0 + i as f64);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value, 0.5);
    }

    #[test]
    fn factory_feeds_resolved_hurst_pct() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::HurstPct(<<HurstPercentile as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..250 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        let v = f.read(IndicatorOutputId::HurstPct);
        assert!(v >= 0.0 && v <= 1.0);
    }
}
