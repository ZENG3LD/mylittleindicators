// Polarized Fractal Efficiency (PFE) — measures trend directness.
//
// Formula (Davis/Kaufman variant):
//   efficiency = |close[N-1] - close[0]| / sum(|close[i] - close[i-1]|, i=1..N-1)
//   PFE = 100 * efficiency * sign(close[N-1] - close[0])
//
// Output in [-100, 100]:
//   +100 = perfectly efficient uptrend (straight line up)
//   -100 = perfectly efficient downtrend (straight line down)
//   near 0 = choppy / sideways


#[derive(Debug, Clone)]
pub struct Pfe {
    window: usize,
    closes: Vec<f64>,
    idx: usize,
    count: usize,
    value: f64,
}

impl Pfe {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.clamp(5, 1024),
            closes: Vec::with_capacity(window.clamp(5, 1024)),
            idx: 0,
            count: 0,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.closes.clear();
        self.idx = 0;
        self.count = 0;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.window
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed ONE pre-extracted scalar (close) — contract input (SOURCE = Field{Close}).
    pub fn feed(&mut self, c: f64) -> f64 {
        if self.count < self.window {
            self.closes.push(c);
            self.count += 1;
        } else {
            self.closes[self.idx] = c;
            self.idx = (self.idx + 1) % self.window;
        }

        if self.is_ready() {
            let newest_idx = (self.idx + self.window - 1) % self.window;
            let oldest_idx = self.idx;

            let c_newest = self.closes[newest_idx];
            let c_oldest = self.closes[oldest_idx];

            let numerator = (c_newest - c_oldest).abs();

            let mut denom = 0.0;
            for i in 0..(self.window - 1) {
                let a_idx = (self.idx + i) % self.window;
                let b_idx = (self.idx + i + 1) % self.window;
                denom += (self.closes[b_idx] - self.closes[a_idx]).abs();
            }

            let eff = if denom > 1e-12 { numerator / denom } else { 0.0 };
            let sign = if c_newest >= c_oldest { 1.0 } else { -1.0 };
            self.value = 100.0 * eff * sign;
        } else {
            self.value = 0.0;
        }
        self.value
    }

    pub fn period(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pfe_creation() {
        let pfe = Pfe::new(10);
        assert!(!pfe.is_ready());
        assert_eq!(pfe.value(), 0.0);
        assert_eq!(pfe.period(), 10);
    }

    #[test]
    fn test_pfe_min_period() {
        let pfe = Pfe::new(2);
        assert_eq!(pfe.period(), 5);
    }

    #[test]
    fn test_pfe_max_period() {
        let pfe = Pfe::new(2000);
        assert_eq!(pfe.period(), 1024);
    }

    #[test]
    fn test_pfe_is_ready_timing() {
        let mut pfe = Pfe::new(5);
        for i in 1..=10 {
            let price = 100.0 + i as f64;
            pfe.feed(price);
            if i < 5 {
                assert!(!pfe.is_ready(), "PFE should not be ready at bar {}", i);
            } else {
                assert!(pfe.is_ready(), "PFE should be ready at bar {}", i);
            }
        }
    }

    #[test]
    fn test_pfe_straight_uptrend() {
        let mut pfe = Pfe::new(5);
        for i in 1..=20 {
            pfe.feed(100.0 + i as f64);
        }
        assert!(pfe.is_ready());
        assert!(pfe.value() > 90.0, "PFE for straight uptrend should be near 100, got {}", pfe.value());
    }

    #[test]
    fn test_pfe_straight_downtrend() {
        let mut pfe = Pfe::new(5);
        for i in 1..=20 {
            pfe.feed(200.0 - i as f64);
        }
        assert!(pfe.is_ready());
        assert!(pfe.value() < -90.0, "PFE for straight downtrend should be near -100, got {}", pfe.value());
    }

    #[test]
    fn test_pfe_choppy_market() {
        let mut pfe = Pfe::new(5);
        for i in 1..=20 {
            let price = 100.0 + if i % 2 == 0 { 5.0 } else { -5.0 };
            pfe.feed(price);
        }
        assert!(pfe.is_ready());
        assert!(pfe.value().abs() < 50.0, "PFE in choppy market should be low, got {}", pfe.value());
    }

    #[test]
    fn test_pfe_range_bounds() {
        let mut pfe = Pfe::new(10);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = pfe.feed(price);
            if pfe.is_ready() {
                assert!(value >= -100.0 && value <= 100.0, "PFE should be in [-100, 100], got {}", value);
            }
        }
    }

    #[test]
    fn test_pfe_reset() {
        let mut pfe = Pfe::new(10);
        for i in 1..=20 {
            pfe.feed(100.0 + i as f64);
        }
        assert!(pfe.is_ready());
        pfe.reset();
        assert!(!pfe.is_ready());
        assert_eq!(pfe.value(), 0.0);
    }

    #[test]
    fn test_pfe_finite_values() {
        let mut pfe = Pfe::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            let value = pfe.feed(price);
            assert!(value.is_finite(), "PFE should always be finite");
        }
    }
}

impl Default for Pfe {
    fn default() -> Self {
        Self::new(10)
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

/// Own config for [`Pfe`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PfeConfig {
    pub period: Param<usize>,
}

impl Indicator for Pfe {
    const ID: IndicatorId = IndicatorId::Pfe;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Pfe)];
    /// O(period): each bar walks the window to sum absolute deltas.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );

    type Config = PfeConfig;
    type Runtime = Pfe;

    fn create(cfg: PfeConfig) -> Pfe {
        Pfe::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for PfeConfig {
    fn defaults() -> Self {
        PfeConfig { period: Param::Solo(10) }
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


impl Render for Pfe {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::Pfe,
                "PFE",
                Color::hex(0x9C27B0),
                1.5,
            ))
            .bounds(-100.0, 100.0)
            .zero_baseline()
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
    fn factory_feeds_resolved_source() {
        let mut f = IndicatorOrder::Pfe(<<Pfe as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=20 {
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: 100.0 + i as f64,
                volume: 0.0,
            });
        }
        let v = f.primary();
        assert!(v >= -100.0 && v <= 100.0, "PFE must be in [-100,100], got {v}");
    }
}
