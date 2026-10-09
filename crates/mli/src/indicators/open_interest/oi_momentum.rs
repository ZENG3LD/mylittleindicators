//! OiMomentum — linear slope of open interest over a rolling window.
//!
//! slope = (latest_oi - oldest_oi) / (n - 1) where n = number of samples in window.
//! Positive = OI growing (positions accumulating).
//! Negative = OI shrinking (positions unwinding).
//!
//! Output: `Single(slope)`. Zero when fewer than 2 samples.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::open_interest_consumer::OpenInterestConsumer;
use crate::contract::{Family, Indicator, Output, Param, SourceAxis};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OpenInterest;

/// Rolling linear slope of open interest.
#[derive(Clone, Debug)]
pub struct OiMomentum {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl OiMomentum {
    /// Create with given period (minimum 2).
    pub fn new(period: usize) -> Self {
        Self {
            period: period.max(2),
            history: VecDeque::with_capacity(period.max(2)),
            last_slope: 0.0,
        }
    }
}

impl Default for OiMomentum {
    fn default() -> Self {
        Self::new(14)
    }
}

impl OpenInterestConsumer for OiMomentum {
    fn update_oi(&mut self, oi: &OpenInterest) {
        if self.history.len() == self.period {
            self.history.pop_front();
        }
        self.history.push_back(oi.open_interest);

        let n = self.history.len();
        if n >= 2 {
            let oldest = *self.history.front().expect("history non-empty checked above");
            let latest = *self.history.back().expect("history non-empty checked above");
            self.last_slope = (latest - oldest) / (n - 1) as f64;
        }
    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_slope = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= 2
    }
}

/// Typed configuration for [`OiMomentum`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct OiMomentumConfig {
    pub period: Param<usize>,
}

impl Indicator for OiMomentum {
    const ID: IndicatorId = IndicatorId::OiMomentum;
    const FAMILY: &'static [Family] = &[Family::OpenInterest];
    const INPUT: &'static [StreamKind] = &[StreamKind::OpenInterest];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::OiMomentum)];
    type Config = OiMomentumConfig;
    type Runtime = OiMomentum;

    fn create(cfg: OiMomentumConfig) -> OiMomentum {
        OiMomentum::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for OiMomentumConfig {
    fn defaults() -> Self {
        OiMomentumConfig { period: Param::Solo(14) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A (integer lookback) — auto already sets range(2, 4048, 1).
        Self::machine_defaults_auto()
    }
}


impl Render for OiMomentum {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::OiMomentum, "OI Momentum", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    fn make_oi(oi: f64) -> OpenInterest {
        OpenInterest {
            open_interest: oi,
            open_interest_value: None,
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn factory_feeds_resolved_oi_momentum() {
        let mut f = IndicatorOrder::OiMomentum(OiMomentumConfig { period: Param::Solo(3) }).build_solo().unwrap();
        let oi1 = make_oi(100.0);
        let oi2 = make_oi(150.0);
        let oi3 = make_oi(200.0);
        f.feed(0, MarketSample::OpenInterest(&oi1));
        f.feed(0, MarketSample::OpenInterest(&oi2));
        f.feed(0, MarketSample::OpenInterest(&oi3));
        let s = f.primary();
        assert!((s - 50.0).abs() < 1e-9, "slope={s}");
    }

    #[test]
    fn not_ready_on_single_sample() {
        let mut ind = OiMomentum::new(5);
        ind.update_oi(&make_oi(100.0));
        assert!(!ind.is_ready());
    }

    #[test]
    fn positive_slope_on_rising_oi() {
        // 100 → 200 over 3 bars: slope = (200-100)/(2) = 50
        let mut ind = OiMomentum::new(3);
        ind.update_oi(&make_oi(100.0));
        ind.update_oi(&make_oi(150.0));
        ind.update_oi(&make_oi(200.0));
        let s = ind.value();
        assert!((s - 50.0).abs() < 1e-9, "expected slope=50, got {s}");
    }

    #[test]
    fn negative_slope_on_falling_oi() {
        let mut ind = OiMomentum::new(3);
        ind.update_oi(&make_oi(200.0));
        ind.update_oi(&make_oi(150.0));
        ind.update_oi(&make_oi(100.0));
        let s = ind.value();
        assert!(s < 0.0, "slope should be negative, got {s}");
    }

    #[test]
    fn window_slides_correctly() {
        // period=2: always (latest - oldest) / 1
        let mut ind = OiMomentum::new(2);
        ind.update_oi(&make_oi(100.0));
        ind.update_oi(&make_oi(110.0));
        // slope = 10
        let s1 = ind.value();
        assert!((s1 - 10.0).abs() < 1e-9);
        // Next: oldest becomes 110, latest = 90 → slope = -20
        ind.update_oi(&make_oi(90.0));
        let s2 = ind.value();
        assert!((s2 - (-20.0)).abs() < 1e-9, "expected -20, got {s2}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = OiMomentum::new(5);
        ind.update_oi(&make_oi(100.0));
        ind.update_oi(&make_oi(200.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }
}

impl OiMomentum {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_slope
    }
}
