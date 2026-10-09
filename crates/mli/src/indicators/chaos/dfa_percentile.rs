// DFA percentile over rolling window (uses Dfa result as feature)

use crate::indicators::chaos::dfa::Dfa;

#[derive(Debug, Clone)]
pub struct DfaPercentile {
    inner: Dfa,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl Default for DfaPercentile {
    fn default() -> Self {
        Self::new([16, 32, 64, 128], 200)
    }
}

impl DfaPercentile {
    pub fn new(scales: [usize; 4], window: usize) -> Self {
        let w = window.max(50);
        Self {
            inner: Dfa::new(scales),
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
        let dval = self.inner.feed(c);
        self.buf[self.idx] = dval;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut cnt = 0usize;
            for i in 0..self.window {
                if self.buf[i] <= dval {
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
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Config for DfaPercentile: inherits DFA's scale windows + the rolling window size.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DfaPercentileConfig {
    pub scales: Param<[usize; 4]>,
    pub window: Param<usize>,
}

impl Indicator for DfaPercentile {
    const ID: IndicatorId = IndicatorId::DfaPct;
    /// Chaos / statistical scorer — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Single configurable close field (fed to the inner DFA).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// Outer is O(window) per bar (rescans the ring for rank). One period-deep Vec ring.
    /// Inner DFA cost is charged via the Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[],
        inner: &[Port::new(IndicatorId::Dfa, &[IndicatorOutputId::Dfa])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::DfaPct)];

    type Config = DfaPercentileConfig;
    type Runtime = DfaPercentile;

    fn create(cfg: DfaPercentileConfig) -> DfaPercentile {
        DfaPercentile::new(cfg.scales.resolved(), cfg.window.resolved())
    }

    fn source_fields(cfg: &DfaPercentileConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for DfaPercentileConfig {
    fn defaults() -> Self {
        DfaPercentileConfig {
            scales: Param::Solo([16, 32, 64, 128]),
            window: Param::Solo(200),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // scales: Class S fixed-array → 4 canonical DFA presets
        s.scales = Param::many(vec![
            [4usize, 8, 16, 32],
            [8usize, 16, 32, 64],
            [16usize, 32, 64, 128],
            [32usize, 64, 128, 256],
        ]);
        // window: Class A period → auto range(2,4048,1) — leave as-is
        s
    }
}


impl Render for DfaPercentile {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::DfaPct, "DFA %", Color::hex(0x009688))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dfa_percentile_creation() {
        let ind = DfaPercentile::new([8, 16, 32, 64], 50);
        assert!(!ind.is_ready());
        assert_eq!(ind.value, 0.5);
    }

    #[test]
    fn test_dfa_percentile_warmup() {
        let mut ind = DfaPercentile::new([8, 16, 32, 64], 50);
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.feed(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_dfa_percentile_values_range() {
        let mut ind = DfaPercentile::new([8, 16, 32, 64], 50);
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let pct = ind.feed(price);
            assert!(pct >= 0.0 && pct <= 1.0);
        }
    }

    #[test]
    fn test_dfa_percentile_reset() {
        let mut ind = DfaPercentile::new([8, 16, 32, 64], 50);
        for i in 0..120 {
            ind.feed(100.0 + i as f64);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value, 0.5);
    }

    #[test]
    fn factory_feeds_resolved_dfa_pct() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::DfaPct(<<DfaPercentile as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..300 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        let v = f.read(IndicatorOutputId::DfaPct);
        assert!(v.is_finite());
    }
}
