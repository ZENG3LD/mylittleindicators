//! BasisExtreme — detects when basis is at extreme percentile levels.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::BasisConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::core::types::Basis;
use crate::engine::stream_kind::StreamKind;

/// Detects basis at percentile extremes within a rolling window.
///
/// Computes the 95th and 5th percentile of the rolling history.
/// If current basis > p95 → +1, < p5 → -1, otherwise 0.
///
/// Output: `Signal(i8)`.
#[derive(Clone, Debug)]
pub struct BasisExtreme {
    period: usize,
    history: VecDeque<f64>,
    last_signal: i8,
}

impl BasisExtreme {
    /// Create a new indicator. `period` is clamped to at least 3.
    pub fn new(period: usize) -> Self {
        let period = period.max(3);
        Self {
            period,
            history: VecDeque::with_capacity(period),
            last_signal: 0,
        }
    }

    fn compute_signal(&self, current: f64) -> i8 {
        let n = self.history.len();
        if n < 2 {
            return 0;
        }
        let mut sorted: Vec<f64> = self.history.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p95_idx = ((n as f64 * 0.95) as usize).min(n.saturating_sub(1));
        let p5_idx = ((n as f64 * 0.05) as usize).min(n.saturating_sub(1));

        let p95 = sorted[p95_idx];
        let p5 = sorted[p5_idx];

        if current > p95 {
            1
        } else if current < p5 {
            -1
        } else {
            0
        }
    }
}

impl Default for BasisExtreme {
    fn default() -> Self {
        Self::new(20)
    }
}

impl Indicator for BasisExtreme {
    const ID: IndicatorId = IndicatorId::BasisExtreme;
    /// Not a pluggable family slot — a detector/signal.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Basis];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::BasisExtreme)];
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Deque), Store::window(StoreKind::SortedVec)],
    );
    type Config = BasisExtremeConfig;
    type Runtime = BasisExtreme;

    fn create(cfg: BasisExtremeConfig) -> BasisExtreme {
        BasisExtreme::new(cfg.period.resolved())
    }
}

/// Own config for [`BasisExtreme`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct BasisExtremeConfig {
    pub period: Param<usize>,
}

impl crate::contract::Config for BasisExtremeConfig {
    fn defaults() -> Self {
        BasisExtremeConfig { period: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // period: rolling lookback — Class A period, auto sweep range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for BasisExtreme {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::BasisExtreme, "Basis Extreme", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(0)
            .build()
    }
}

impl BasisConsumer for BasisExtreme {
    fn update_basis(&mut self, b: &Basis) {
        let current = b.basis;
        // Compute signal against existing history before inserting new value
        self.last_signal = self.compute_signal(current);
        self.history.push_back(current);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    fn make_basis(v: f64) -> Basis {
        Basis { basis: v, timestamp: 0, ..Default::default()}
    }

    #[test]
    fn factory_feeds_resolved_basis_extreme() {
        let mut f = IndicatorOrder::BasisExtreme(<<BasisExtreme as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Feed 20 Basis samples, then a spike
        for i in 0..20 {
            f.feed(0, MarketSample::Basis(&make_basis(i as f64)));
        }
        f.feed(0, MarketSample::Basis(&make_basis(9999.0)));
        // Spike well above p95 → Signal(1)
        assert_eq!(f.primary().round() as i64, 1);
    }

    #[test]
    fn extreme_high_gives_plus_one() {
        let mut ind = BasisExtreme::new(20);
        for v in 0..20 {
            ind.update_basis(&make_basis(v as f64));
        }
        // Push a value well above p95
        ind.update_basis(&make_basis(1000.0));
        assert_eq!(ind.value().round() as i8, 1, "should be +1 for extreme high");
    }

    #[test]
    fn extreme_low_gives_minus_one() {
        let mut ind = BasisExtreme::new(20);
        for v in 0..20 {
            ind.update_basis(&make_basis(v as f64));
        }
        ind.update_basis(&make_basis(-1000.0));
        assert_eq!(ind.value().round() as i8, -1, "should be -1 for extreme low");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BasisExtreme::new(5);
        for v in 0..5 {
            ind.update_basis(&make_basis(v as f64));
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().round() as i8, 0);
    }
}

impl BasisExtreme {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }
}
