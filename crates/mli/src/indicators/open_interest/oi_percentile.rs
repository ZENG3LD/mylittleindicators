//! OiPercentile — rolling percentile rank of current open interest.
//!
//! rank = count_of_window_values_strictly_below_current / window_size ∈ [0, 1)
//! where window_size = number of historic values (excluding current bar).
//!
//! If no history yet: returns 0.0.
//!
//! Output: `Single(percentile_rank)` ∈ [0, 1].

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::open_interest_consumer::OpenInterestConsumer;
use crate::contract::{Family, Indicator, Output, Param, SourceAxis};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OpenInterest;

/// Rolling percentile rank of current OI versus its recent history.
#[derive(Clone, Debug)]
pub struct OiPercentile {
    window: usize,
    history: VecDeque<f64>,
    last_rank: f64,
}

impl OiPercentile {
    /// Create with given window size (minimum 1).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            history: VecDeque::with_capacity(window.max(1)),
            last_rank: 0.0,
        }
    }
}

impl Default for OiPercentile {
    fn default() -> Self {
        Self::new(50)
    }
}

impl OpenInterestConsumer for OiPercentile {
    fn update_oi(&mut self, oi: &OpenInterest) {
        let current = oi.open_interest;

        let n = self.history.len();
        if n > 0 {
            let count_below = self.history.iter().filter(|&&v| v < current).count();
            self.last_rank = count_below as f64 / n as f64;
        } else {
            self.last_rank = 0.0;
        }

        if self.history.len() == self.window {
            self.history.pop_front();
        }
        self.history.push_back(current);

    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_rank = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.history.is_empty()
    }
}

/// Typed configuration for [`OiPercentile`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct OiPercentileConfig {
    pub window: Param<usize>,
}

impl Indicator for OiPercentile {
    const ID: IndicatorId = IndicatorId::OiPercentile;
    const FAMILY: &'static [Family] = &[Family::OpenInterest];
    const INPUT: &'static [StreamKind] = &[StreamKind::OpenInterest];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::OiPercentile)];
    type Config = OiPercentileConfig;
    type Runtime = OiPercentile;

    fn create(cfg: OiPercentileConfig) -> OiPercentile {
        OiPercentile::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for OiPercentileConfig {
    fn defaults() -> Self {
        OiPercentileConfig { window: Param::Solo(50) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // window: Class A (integer lookback/window) — auto already sets range(2, 4048, 1).
        Self::machine_defaults_auto()
    }
}


impl Render for OiPercentile {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::OiPercentile, "OI Percentile", Color::hex(0x00BCD4))
            .bounds(0.0, 1.0)
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
    fn factory_feeds_resolved_oi_percentile() {
        let mut f = IndicatorOrder::OiPercentile(OiPercentileConfig { window: Param::Solo(50) }).build_solo().unwrap();
        for _ in 0..10 {
            let oi = make_oi(100.0);
            f.feed(0, MarketSample::OpenInterest(&oi));
        }
        let oi_high = make_oi(999.0);
        f.feed(0, MarketSample::OpenInterest(&oi_high));
        let r = f.primary();
        assert!((r - 1.0).abs() < 1e-9, "rank={r}");
    }

    #[test]
    fn not_ready_initially() {
        let ind = OiPercentile::new(5);
        assert!(!ind.is_ready());
    }

    #[test]
    fn first_update_returns_zero() {
        let mut ind = OiPercentile::new(5);
        ind.update_oi(&make_oi(100.0));
        // no history before first bar
        assert_eq!(ind.value(), 0.0, "first bar has no history to compare");
    }

    #[test]
    fn rank_one_when_above_all() {
        let mut ind = OiPercentile::new(50);
        // push 10 values of 100
        for _ in 0..10 {
            ind.update_oi(&make_oi(100.0));
        }
        // value 999 is above all 10 history entries
        ind.update_oi(&make_oi(999.0));
        let r = ind.value();
        assert!((r - 1.0).abs() < 1e-9, "expected rank=1.0, got {r}");
    }

    #[test]
    fn rank_zero_when_below_all() {
        let mut ind = OiPercentile::new(50);
        for _ in 0..5 {
            ind.update_oi(&make_oi(200.0));
        }
        ind.update_oi(&make_oi(1.0));
        let r = ind.value();
        assert_eq!(r, 0.0, "expected rank=0 when below all history");
    }

    #[test]
    fn rank_midpoint() {
        let mut ind = OiPercentile::new(50);
        // history: [100, 200, 300, 400]
        ind.update_oi(&make_oi(100.0));
        ind.update_oi(&make_oi(200.0));
        ind.update_oi(&make_oi(300.0));
        ind.update_oi(&make_oi(400.0));
        // current = 250: count_below = 2 (100, 200) out of 4 → 0.5
        ind.update_oi(&make_oi(250.0));
        let r = ind.value();
        assert!((r - 0.5).abs() < 1e-9, "expected 0.5, got {r}");
    }

    #[test]
    fn reset_clears() {
        let mut ind = OiPercentile::new(5);
        ind.update_oi(&make_oi(100.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }
}

impl OiPercentile {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_rank
    }
}
