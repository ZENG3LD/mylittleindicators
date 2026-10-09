//! BasisZScore — z-score of current basis relative to rolling mean and std.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::BasisConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::core::types::Basis;
use crate::engine::stream_kind::StreamKind;

/// Computes the z-score of the current basis value within a rolling window.
///
/// z = (current - mean) / std
///
/// Output: `Single(z)`. Returns 0.0 until at least 2 snapshots.
#[derive(Clone, Debug)]
pub struct BasisZScore {
    period: usize,
    history: VecDeque<f64>,
    last_z: f64,
}

impl BasisZScore {
    /// Create a new indicator. `period` is clamped to at least 2.
    pub fn new(period: usize) -> Self {
        let period = period.max(2);
        Self {
            period,
            history: VecDeque::with_capacity(period),
            last_z: 0.0,
        }
    }

    fn compute_z(&self, current: f64) -> f64 {
        let n = self.history.len();
        if n < 2 {
            return 0.0;
        }
        let mean = self.history.iter().sum::<f64>() / n as f64;
        let variance = self.history.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        let std = variance.sqrt();
        if std < 1e-12 {
            0.0
        } else {
            (current - mean) / std
        }
    }
}

impl Default for BasisZScore {
    fn default() -> Self {
        Self::new(20)
    }
}

impl Indicator for BasisZScore {
    const ID: IndicatorId = IndicatorId::BasisZScore;
    /// Not a pluggable family slot — a statistical normalization of the basis stream.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Basis];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::BasisZScore)];
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = BasisZScoreConfig;
    type Runtime = BasisZScore;

    fn create(cfg: BasisZScoreConfig) -> BasisZScore {
        BasisZScore::new(cfg.period.resolved())
    }
}

/// Own config for [`BasisZScore`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct BasisZScoreConfig {
    pub period: Param<usize>,
}

impl crate::contract::Config for BasisZScoreConfig {
    fn defaults() -> Self {
        BasisZScoreConfig { period: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // period: rolling z-score window — Class A period, auto sweep range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for BasisZScore {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::BasisZScore, "Basis Z-Score", Color::hex(0x7E57C2))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

impl BasisConsumer for BasisZScore {
    fn update_basis(&mut self, b: &Basis) {
        let current = b.basis;
        self.history.push_back(current);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
        self.last_z = self.compute_z(current);
    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_z = 0.0;
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
    fn factory_feeds_resolved_basis_z_score() {
        let mut f = IndicatorOrder::BasisZScore(<<BasisZScore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..20 {
            f.feed(0, MarketSample::Basis(&make_basis(i as f64)));
        }
        f.feed(0, MarketSample::Basis(&make_basis(9999.0)));
        // Spike well above mean → z > 2
        assert!(f.primary() > 2.0);
    }

    #[test]
    fn extreme_value_gives_high_zscore() {
        let mut ind = BasisZScore::new(20);
        for v in 0..20 {
            ind.update_basis(&make_basis(v as f64));
        }
        // Value far above mean should give positive z
        ind.update_basis(&make_basis(1000.0));
        let z = ind.value();
        assert!(z > 2.0, "z should be high for extreme value, got {z}");
    }

    #[test]
    fn constant_series_gives_zero_zscore() {
        let mut ind = BasisZScore::new(5);
        for _ in 0..5 {
            ind.update_basis(&make_basis(5.0));
        }
        let z = ind.value();
        assert!(z.abs() < 1e-9, "z should be 0 for constant series, got {z}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BasisZScore::new(5);
        for v in 0..5 {
            ind.update_basis(&make_basis(v as f64));
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }
}

impl BasisZScore {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_z
    }
}
