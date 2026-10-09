//! BasisMomentum — rolling linear slope of basis values.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::BasisConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::core::types::Basis;
use crate::engine::stream_kind::StreamKind;

/// Computes the linear slope of basis over the last `period` snapshots.
///
/// slope = (latest − oldest) / (n − 1)
///
/// Output: `Single(slope)`. Returns 0.0 until at least two snapshots.
#[derive(Clone, Debug)]
pub struct BasisMomentum {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl BasisMomentum {
    /// Create a new indicator. `period` is clamped to at least 2.
    pub fn new(period: usize) -> Self {
        let period = period.max(2);
        Self {
            period,
            history: VecDeque::with_capacity(period),
            last_slope: 0.0,
        }
    }

    fn compute_slope(&self) -> f64 {
        let n = self.history.len();
        if n < 2 {
            return 0.0;
        }
        let oldest = self.history[0];
        let latest = self.history[n - 1];
        (latest - oldest) / (n as f64 - 1.0)
    }
}

impl Default for BasisMomentum {
    fn default() -> Self {
        Self::new(14)
    }
}

impl Indicator for BasisMomentum {
    const ID: IndicatorId = IndicatorId::BasisMomentum;
    /// Not a pluggable family slot — a momentum slope of the basis stream.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Basis];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::BasisMomentum)];
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = BasisMomentumConfig;
    type Runtime = BasisMomentum;

    fn create(cfg: BasisMomentumConfig) -> BasisMomentum {
        BasisMomentum::new(cfg.period.resolved())
    }
}

/// Own config for [`BasisMomentum`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct BasisMomentumConfig {
    pub period: Param<usize>,
}

impl crate::contract::Config for BasisMomentumConfig {
    fn defaults() -> Self {
        BasisMomentumConfig { period: Param::Solo(14) }
    }
    fn machine_defaults() -> Self {
        // period: rolling slope window — Class A period, auto sweep range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for BasisMomentum {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::BasisMomentum, "Basis Momentum", Color::hex(0x26C6DA))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

impl BasisConsumer for BasisMomentum {
    fn update_basis(&mut self, b: &Basis) {
        self.history.push_back(b.basis);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
        self.last_slope = self.compute_slope();
    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_slope = 0.0;
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
    fn factory_feeds_resolved_basis_momentum() {
        let mut f = IndicatorOrder::BasisMomentum(<<BasisMomentum as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for v in [1.0f64, 2.0, 3.0, 4.0, 5.0] {
            f.feed(0, MarketSample::Basis(&make_basis(v)));
        }
        // Rising series → positive slope
        assert!(f.primary() > 0.0);
    }

    #[test]
    fn rising_basis_gives_positive_slope() {
        let mut ind = BasisMomentum::new(5);
        for v in [1.0, 2.0, 3.0, 4.0, 5.0] {
            ind.update_basis(&make_basis(v));
        }
        let s = ind.value();
        assert!(s > 0.0, "slope should be positive, got {s}");
    }

    #[test]
    fn falling_basis_gives_negative_slope() {
        let mut ind = BasisMomentum::new(5);
        for v in [5.0, 4.0, 3.0, 2.0, 1.0] {
            ind.update_basis(&make_basis(v));
        }
        let s = ind.value();
        assert!(s < 0.0, "slope should be negative, got {s}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BasisMomentum::new(3);
        ind.update_basis(&make_basis(1.0));
        ind.update_basis(&make_basis(2.0));
        ind.reset();
        assert!(!ind.is_ready());
        let v = ind.value();
        assert_eq!(v, 0.0);
    }
}

impl BasisMomentum {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_slope
    }
}
