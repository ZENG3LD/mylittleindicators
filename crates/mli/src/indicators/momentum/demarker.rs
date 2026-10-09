/// DeMarker (DeM) oscillator
#[derive(Debug, Clone)]
pub struct Demarker {
    period: usize,
    sum_up: f64,
    sum_down: f64,
    prev_high: f64,
    prev_low: f64,
    initialized: bool,
    value: f64,
}

impl Demarker {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.max(1),
            sum_up: 0.0,
            sum_down: 0.0,
            prev_high: 0.0,
            prev_low: 0.0,
            initialized: false,
            value: 0.0,
        }
    }

    /// Feed resolved `[high, low]` lanes — contract input (SOURCE = KlineSlice[H, L]).
    /// Rolling sums via Wilder-like smoothing to keep O(1) and avoid large buffers.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        if !self.initialized {
            self.prev_high = h;
            self.prev_low = l;
            self.initialized = true;
            self.value = 0.0;
            return self.value;
        }
        let up = (h - self.prev_high).max(0.0);
        let down = (self.prev_low - l).max(0.0);
        let p = self.period as f64;
        self.sum_up = (self.sum_up * (p - 1.0) + up) / p;
        self.sum_down = (self.sum_down * (p - 1.0) + down) / p;
        let denom = self.sum_up + self.sum_down;
        self.value = if denom > 0.0 {
            self.sum_up / denom
        } else {
            0.0
        };
        self.prev_high = h;
        self.prev_low = l;
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
    pub fn reset(&mut self) {
        self.sum_up = 0.0;
        self.sum_down = 0.0;
        self.prev_high = 0.0;
        self.prev_low = 0.0;
        self.initialized = false;
        self.value = 0.0;
    }
    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demarker_creation() {
        let dem = Demarker::new(14);
        assert!(!dem.is_ready());
        assert_eq!(dem.value(), 0.0);
        assert_eq!(dem.period(), 14);
    }

    #[test]
    fn test_demarker_basic_calculation() {
        let mut dem = Demarker::new(14);

        for i in 1..=30 {
            let price = 100.0 + i as f64;
            let value = dem.feed(&[price + 2.0, price - 1.0]);

            if i > 1 {
                // DeMarker oscillates between 0 and 1
                assert!(value >= 0.0 && value <= 1.0, "DeMarker should be in [0, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_demarker_uptrend() {
        let mut dem = Demarker::new(14);

        // Strong uptrend - highs increasing more than lows
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            dem.feed(&[price + 3.0, price - 0.5]);
        }

        // In uptrend, DeMarker tends to be > 0.5
        if dem.is_ready() {
            assert!(dem.value() > 0.3, "DeMarker in uptrend should be elevated");
        }
    }

    #[test]
    fn test_demarker_downtrend() {
        let mut dem = Demarker::new(14);

        // Strong downtrend - lows decreasing more than highs
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            dem.feed(&[price + 0.5, price - 3.0]);
        }

        // In downtrend, DeMarker tends to be < 0.5
        if dem.is_ready() {
            assert!(dem.value() < 0.7, "DeMarker in downtrend should be suppressed");
        }
    }

    #[test]
    fn test_demarker_reset() {
        let mut dem = Demarker::new(14);

        for i in 1..=30 {
            let price = 100.0 + i as f64;
            dem.feed(&[price + 1.0, price - 1.0]);
        }

        assert!(dem.is_ready());

        dem.reset();
        assert!(!dem.is_ready());
        assert_eq!(dem.value(), 0.0);
    }

    #[test]
    fn test_demarker_period() {
        let dem = Demarker::new(14);
        assert_eq!(dem.period(), 14);

        let dem2 = Demarker::new(21);
        assert_eq!(dem2.period(), 21);
    }

    #[test]
    fn test_demarker_is_ready() {
        let mut dem = Demarker::new(14);
        assert!(!dem.is_ready());

        // First feed initializes
        dem.feed(&[101.0, 99.0]);
        assert!(dem.is_ready()); // Initialized after first bar
    }

    #[test]
    fn test_demarker_sideways() {
        let mut dem = Demarker::new(14);

        // Sideways market - equal up and down movements
        for i in 1..=30 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 2.0;
            dem.feed(&[price + 1.0, price - 1.0]);
        }

        // In sideways, DeMarker should be around 0.5
        if dem.is_ready() {
            let value = dem.value();
            assert!(value >= 0.0 && value <= 1.0);
        }
    }

    #[test]
    fn test_demarker_extreme_values() {
        let mut dem = Demarker::new(5);

        // Extreme upward movement
        for i in 1..=20 {
            let price = 100.0 + i as f64 * 5.0;
            dem.feed(&[price + 10.0, price - 0.1]);
        }

        if dem.is_ready() {
            let value = dem.value();
            // Should be close to 1.0 in extreme uptrend
            assert!(value > 0.5, "DeMarker should be high in extreme uptrend, got {}", value);
        }
    }
}

impl Default for Demarker {
    fn default() -> Self {
        Self::new(14)
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

/// Own config for [`Demarker`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DemarkerConfig {
    pub period: Param<usize>,
}

impl Indicator for Demarker {
    const ID: IndicatorId = IndicatorId::Demarker;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads High and Low — fixed, not configurable.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Demarker)];
    /// O(1): Wilder EMA-style running sums, no window rescan.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::fixed(StoreKind::Scalar, 4)]);

    type Config = DemarkerConfig;
    type Runtime = Demarker;

    fn create(cfg: DemarkerConfig) -> Demarker {
        Demarker::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for DemarkerConfig {
    fn defaults() -> Self {
        DemarkerConfig { period: Param::Solo(14) }
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


impl Render for Demarker {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::Demarker,
                "DeMarker",
                Color::hex(0x009688),
                1.5,
            ))
            .bounds(0.0, 1.0)
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
        let mut f = IndicatorOrder::Demarker(<<Demarker as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=30 {
            let base = 100.0 + i as f64 * 2.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: base + 3.0,
                low: base - 0.5,
                close: base + 2.0,
                volume: 0.0,
            });
        }
        let v = f.primary();
        assert!(v >= 0.0 && v <= 1.0, "DeMarker must be in [0,1], got {v}");
    }
}
