// Rolling Quartiles (Q1, Median, Q3) of Close over window (naive O(window) updates)

use crate::indicators::utils::math::percentile::quartiles;

#[derive(Debug, Clone)]
pub struct RollingQuartiles {
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub q1: f64,
    pub q2: f64,
    pub q3: f64,
}

impl RollingQuartiles {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            buf: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            q1: 0.0,
            q2: 0.0,
            q3: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.q1 = 0.0;
        self.q2 = 0.0;
        self.q3 = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    fn recompute(&mut self, close: f64) -> (f64, f64, f64) {
        self.buf[self.idx] = close;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            // 🚀 O(n) quartiles function instead of O(n log n) sorting
            let mut tmp = self.buf[..len].to_vec();
            let (q1, q2, q3) = quartiles(&mut tmp);
            self.q1 = q1;
            self.q2 = q2;
            self.q3 = q3;
        }
        (self.q1, self.q2, self.q3)
    }

    /// Named output getter: brace `q1`.
    #[inline]
    pub fn q1(&self) -> f64 { self.q1 }

    /// Named output getter: brace `q2`.
    #[inline]
    pub fn q2(&self) -> f64 { self.q2 }

    /// Named output getter: brace `q3`.
    #[inline]
    pub fn q3(&self) -> f64 { self.q3 }


    /// Feed resolved `[close]` lane.
    pub fn feed(&mut self, v: f64) {
        self.recompute(v);
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderOutput, RenderSpec,
    Store, StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`RollingQuartiles`] — window period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RquartConfig {
    pub period: Param<usize>,
}

impl Indicator for RollingQuartiles {
    const ID: IndicatorId = IndicatorId::Rquart;
    /// Not a pluggable family member — rolling statistical levels producer.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable close field (default).
    // SOURCE stays at default: Field { default: Close }
    /// Triple: Q1, Q2 (median), Q3.
    /// O(window) per bar — partial-sort quartile on every update. One period-deep Vec.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::RquartQ1),
        Output::price(IndicatorOutputId::RquartQ2),
        Output::price(IndicatorOutputId::RquartQ3),
    ];
    type Config = RquartConfig;
    type Runtime = RollingQuartiles;

    fn create(cfg: RquartConfig) -> RollingQuartiles {
        RollingQuartiles::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for RquartConfig {
    fn defaults() -> Self {
        RquartConfig { period: Param::Solo(100) }
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


impl Render for RollingQuartiles {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::RquartQ1, "Q1", Color::hex(0x4CAF50), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::RquartQ2, "Q2", Color::hex(0x2196F3), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::RquartQ3, "Q3", Color::hex(0xF44336), 1.0))
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
    fn factory_feeds_resolved_rquart() {
        let mut f = IndicatorOrder::Rquart(<<RollingQuartiles as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=105 {
            let close = 100.0 + i as f64 * 0.1;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        // factory value() = primary output (q1); must be finite (q1 <= q2 <= q3 verified by unit tests)
        let q1 = f.primary();
        assert!(q1.is_finite(), "q1 must be finite: {q1}");
    }
}

impl Default for RollingQuartiles {
    /// Factory default: `new(100)` — window = `unwrap_or(100)`.
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_quartiles_creation() {
        let rq = RollingQuartiles::new(20);
        assert!(!rq.is_ready());
        assert_eq!(rq.q1, 0.0);
        assert_eq!(rq.q2, 0.0);
        assert_eq!(rq.q3, 0.0);
    }

    #[test]
    fn test_rolling_quartiles_warmup() {
        let mut rq = RollingQuartiles::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rq.recompute(price);
        }
        assert!(rq.is_ready());
    }

    #[test]
    fn test_rolling_quartiles_order() {
        let mut rq = RollingQuartiles::new(20);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let (q1, q2, q3) = rq.recompute(price);
            // Q1 <= Q2 <= Q3
            assert!(q1 <= q2, "Q1 should be <= Q2");
            assert!(q2 <= q3, "Q2 should be <= Q3");
        }
    }

    #[test]
    fn test_rolling_quartiles_reset() {
        let mut rq = RollingQuartiles::new(20);
        for i in 0..25 {
            rq.recompute(100.0 + i as f64);
        }
        rq.reset();
        assert!(!rq.is_ready());
        assert_eq!(rq.q1, 0.0);
        assert_eq!(rq.q2, 0.0);
        assert_eq!(rq.q3, 0.0);
    }
}
