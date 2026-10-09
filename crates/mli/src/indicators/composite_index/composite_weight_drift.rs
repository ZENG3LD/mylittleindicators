//! CompositeWeightDrift — rolling max relative drift across composite index components.
//!
//! Consumer: `CompositeIndexConsumer`.
//!
//! Logic: For each component, compute `|new_weight - prev_weight| / |prev_weight|`.
//! Returns the maximum drift across all components between consecutive snapshots.
//! Unlike `IndexComponentDrift` (which only looks at prev→current), this maintains
//! a rolling history of snapshots and returns max drift across the full window.
//!
//! Output: `Single(max_drift_pct)`.

use std::collections::HashMap;
use std::collections::VecDeque;

use crate::engine::streams::CompositeIndexConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::CompositeIndex;

/// Rolling composite index weight drift indicator.
///
/// Implements `CompositeIndexConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Debug, Clone)]
pub struct CompositeWeightDrift {
    history: VecDeque<HashMap<String, f64>>,
    window_snapshots: usize,
    last_max_drift: f64,
}

impl CompositeWeightDrift {
    /// Create a new indicator.
    ///
    /// - `window_snapshots` — number of snapshots to keep in rolling history (default 10).
    pub fn new(window_snapshots: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(window_snapshots.max(2)),
            window_snapshots: window_snapshots.max(2),
            last_max_drift: 0.0,
        }
    }

    fn recompute_drift(&mut self) {
        if self.history.len() < 2 {
            self.last_max_drift = 0.0;
            return;
        }
        let prev = self.history.get(self.history.len() - 2).unwrap();
        let curr = self.history.back().unwrap();

        let mut max_drift = 0.0_f64;
        for (sym, new_w) in curr {
            if let Some(&old_w) = prev.get(sym.as_str()) {
                if old_w.abs() > 1e-12 {
                    let drift = (new_w - old_w).abs() / old_w.abs();
                    if drift > max_drift {
                        max_drift = drift;
                    }
                }
            }
        }
        self.last_max_drift = max_drift;
    }


    /// Primary scalar output (the single value the macro reads).
    pub fn value(&self) -> f64 {
        self.last_max_drift
    }

    /// True when at least two snapshots have been received.
    pub fn indicator_is_ready(&self) -> bool {
        self.history.len() >= 2
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.history.clear();
        self.last_max_drift = 0.0;
    }
}

impl Default for CompositeWeightDrift {
    fn default() -> Self {
        Self::new(10)
    }
}

impl CompositeIndexConsumer for CompositeWeightDrift {
    fn update_composite_index(&mut self, ci: &CompositeIndex) {
        if self.history.len() >= self.window_snapshots {
            self.history.pop_front();
        }
        let weights: HashMap<String, f64> = ci.components.iter().map(|(s, w)| (s.clone(), *w)).collect();
        self.history.push_back(weights);
        self.recompute_drift();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

use crate::contract::Param;

/// Typed configuration for [`CompositeWeightDrift`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct CompositeWeightDriftConfig {
    /// Number of snapshots to keep in rolling history (minimum 2).
    pub window_snapshots: Param<usize>,
}

impl Indicator for CompositeWeightDrift {
    const ID: IndicatorId = IndicatorId::CompositeWeightDrift;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::CompositeIndex];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::CompositeWeightDrift)];
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = CompositeWeightDriftConfig;
    type Runtime = CompositeWeightDrift;

    fn create(cfg: CompositeWeightDriftConfig) -> CompositeWeightDrift {
        CompositeWeightDrift::new(cfg.window_snapshots.resolved())
    }
}

impl crate::contract::Config for CompositeWeightDriftConfig {
    fn defaults() -> Self {
        CompositeWeightDriftConfig { window_snapshots: Param::Solo(10) }
    }
    fn machine_defaults() -> Self {
        // window_snapshots: rolling snapshot count — Class A period, auto sweep range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for CompositeWeightDrift {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::CompositeWeightDrift, "Weight Drift", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::market_sample::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;

    fn make_ci(components: Vec<(&str, f64)>) -> CompositeIndex {
        CompositeIndex {
            price: 1.0,
            components: components.into_iter().map(|(s, w)| (s.to_string(), w)).collect(),
            timestamp: 0,
        }
    }

    #[test]
    fn drift_detected_on_weight_change() {
        let mut ind = CompositeWeightDrift::new(5);
        ind.update_composite_index(&make_ci(vec![("BTC", 0.5), ("ETH", 0.5)]));
        ind.update_composite_index(&make_ci(vec![("BTC", 0.6), ("ETH", 0.4)]));
        // BTC: |0.6-0.5|/0.5 = 0.2, ETH: |0.4-0.5|/0.5 = 0.2
        let d = ind.value();
        assert!((d - 0.2).abs() < 1e-9, "drift={d}");
    }

    #[test]
    fn zero_drift_on_identical_snapshots() {
        let mut ind = CompositeWeightDrift::new(5);
        ind.update_composite_index(&make_ci(vec![("BTC", 0.5)]));
        ind.update_composite_index(&make_ci(vec![("BTC", 0.5)]));
        let d = ind.value();
        assert!(d.abs() < 1e-12, "drift={d}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = CompositeWeightDrift::default();
        ind.update_composite_index(&make_ci(vec![("BTC", 0.5)]));
        ind.update_composite_index(&make_ci(vec![("BTC", 0.6)]));
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_composite_weight_drift() {
        
        let mut f = IndicatorOrder::CompositeWeightDrift(
            <<CompositeWeightDrift as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let ci1 = make_ci(vec![("BTC", 0.5), ("ETH", 0.5)]);
        let ci2 = make_ci(vec![("BTC", 0.6), ("ETH", 0.4)]);
        f.feed(0, MarketSample::CompositeIndex(&ci1));
        f.feed(0, MarketSample::CompositeIndex(&ci2));
        let d = f.primary();
        assert!(d > 0.0, "factory drift should be positive after weight change, got {d}");
    }
}
