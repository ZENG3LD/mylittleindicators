//! IndexComponentDrift — maximum relative weight change across composite index components.

use std::collections::HashMap;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::CompositeIndexConsumer;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::CompositeIndex;

/// Measures maximum relative drift in component weights between consecutive composite index snapshots.
///
/// For each component computes `|new_weight - old_weight| / old_weight`.
/// Returns the maximum across all components.
///
/// Output: `Single(max_drift_pct)`. Returns 0.0 until two consecutive snapshots arrive.
#[derive(Debug, Clone)]
pub struct IndexComponentDrift {
    prev_weights: HashMap<String, f64>,
    last_drift: f64,
}

impl IndexComponentDrift {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self {
            prev_weights: HashMap::new(),
            last_drift: 0.0,
        }
    }
}

impl Default for IndexComponentDrift {
    fn default() -> Self {
        Self::new()
    }
}

impl CompositeIndexConsumer for IndexComponentDrift {
    fn update_composite_index(&mut self, ci: &CompositeIndex) {
        let mut max_drift = 0.0_f64;
        if !self.prev_weights.is_empty() {
            for (sym, new_w) in &ci.components {
                if let Some(&old_w) = self.prev_weights.get(sym.as_str()) {
                    if old_w.abs() > 1e-12 {
                        let drift = (new_w - old_w).abs() / old_w.abs();
                        if drift > max_drift {
                            max_drift = drift;
                        }
                    }
                }
            }
        }
        self.prev_weights.clear();
        for (sym, w) in &ci.components {
            self.prev_weights.insert(sym.clone(), *w);
        }
        self.last_drift = max_drift;
    }


    fn reset(&mut self) {
        self.prev_weights.clear();
        self.last_drift = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.prev_weights.is_empty()
    }
}

/// Typed configuration for [`IndexComponentDrift`]. No parameters — the indicator
/// is stateless except for the previous snapshot.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct IndexComponentDriftConfig;

impl Indicator for IndexComponentDrift {
    const ID: IndicatorId = IndicatorId::IndexComponentDrift;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::CompositeIndex];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::IndexComponentDrift)];
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::fixed(StoreKind::Vec, 1)],
    );
    type Config = IndexComponentDriftConfig;
    type Runtime = IndexComponentDrift;

    fn create(_cfg: IndexComponentDriftConfig) -> IndexComponentDrift {
        IndexComponentDrift::new()
    }
}

impl crate::contract::Config for IndexComponentDriftConfig {
    fn defaults() -> Self {
        IndexComponentDriftConfig
    }
    fn machine_defaults() -> Self {
        // No Param fields — unit struct. Auto returns Self.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for IndexComponentDrift {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::IndexComponentDrift, "Component Drift", Color::hex(0x4CAF50))
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
        let mut ind = IndexComponentDrift::new();
        ind.update_composite_index(&make_ci(vec![("BTC", 0.5), ("ETH", 0.5)]));
        ind.update_composite_index(&make_ci(vec![("BTC", 0.6), ("ETH", 0.4)]));
        // BTC: |0.6-0.5|/0.5 = 0.2, ETH: |0.4-0.5|/0.5 = 0.2
        let d = ind.value();
        assert!((d - 0.2).abs() < 1e-9, "drift={d}");
    }

    #[test]
    fn zero_drift_on_identical_snapshot() {
        let mut ind = IndexComponentDrift::new();
        ind.update_composite_index(&make_ci(vec![("BTC", 0.5)]));
        ind.update_composite_index(&make_ci(vec![("BTC", 0.5)]));
        let d = ind.value();
        assert!(d.abs() < 1e-12);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = IndexComponentDrift::new();
        ind.update_composite_index(&make_ci(vec![("BTC", 0.5)]));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_index_component_drift() {
        
        let mut f = IndicatorOrder::IndexComponentDrift(
            <<IndexComponentDrift as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let ci1 = make_ci(vec![("BTC", 0.5), ("ETH", 0.5)]);
        let ci2 = make_ci(vec![("BTC", 0.7), ("ETH", 0.3)]);
        f.feed(0, MarketSample::CompositeIndex(&ci1));
        f.feed(0, MarketSample::CompositeIndex(&ci2));
        let d = f.primary();
        assert!(d > 0.0, "factory drift should be positive after weight change, got {d}");
    }
}

impl IndexComponentDrift {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_drift
    }
}
