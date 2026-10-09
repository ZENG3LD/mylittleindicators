//! IndexCorrelationBreakdown — minimum pairwise Pearson correlation of component weights.

use std::collections::{HashMap, VecDeque};

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::CompositeIndexConsumer;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity, ValueDomain};
use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::CompositeIndex;

/// Maximum number of components tracked in the correlation grid.
const MAX_COMPONENTS: u16 = 32;

/// Rolling minimum pairwise Pearson correlation of component weights.
///
/// Maintains a rolling window of component weight snapshots.
/// For each pair of components computes the Pearson correlation of their
/// weight time series, then returns the minimum.
///
/// Output: `Single(min_correlation)`. Returns `1.0` until at least two
/// snapshots and two components are available.
#[derive(Debug, Clone)]
pub struct IndexCorrelationBreakdown {
    window_size: usize,
    snapshots: VecDeque<HashMap<String, f64>>,
    last_min_corr: f64,
    /// Full N×N pairwise correlation grid, fixed at MAX_COMPONENTS × MAX_COMPONENTS.
    grid: MatrixGrid,
}

impl IndexCorrelationBreakdown {
    /// Create a new indicator. `window_size` is clamped to at least 2.
    pub fn new(window_size: usize) -> Self {
        let window_size = window_size.max(2);
        Self {
            window_size,
            snapshots: VecDeque::with_capacity(window_size),
            last_min_corr: 1.0,
            grid: MatrixGrid::new(MAX_COMPONENTS, MAX_COMPONENTS, false)
                .with_labels(AxisLabels::Ordinal("component"), AxisLabels::Ordinal("component")),
        }
    }

    /// Recompute all pairwise Pearson correlations, fill the N×N grid, and return the minimum.
    ///
    /// The grid is reset first, then every `(i, j)` pair is written symmetrically.
    /// Diagonal cells are set to `1.0`. Indices `>= MAX_COMPONENTS` are skipped.
    fn recompute_corr(&mut self) -> f64 {
        self.grid.reset();

        if self.snapshots.len() < 2 {
            return 1.0;
        }

        // Collect all unique symbols across all snapshots
        let mut symbols: Vec<String> = self
            .snapshots
            .iter()
            .flat_map(|snap| snap.keys().cloned())
            .collect();
        symbols.sort();
        symbols.dedup();

        if symbols.len() < 2 {
            return 1.0;
        }

        let n = self.snapshots.len();

        // Build weight series per symbol
        let series: Vec<Vec<f64>> = symbols
            .iter()
            .map(|sym| {
                self.snapshots
                    .iter()
                    .map(|snap| snap.get(sym).copied().unwrap_or(0.0))
                    .collect()
            })
            .collect();

        // Fill diagonal
        for i in 0..series.len() {
            if i as u16 >= MAX_COMPONENTS {
                break;
            }
            self.grid.set_direction(MatrixCell::new(i as u16, i as u16), 1.0);
        }

        let mut min_corr = 1.0_f64;

        for i in 0..series.len() {
            for j in (i + 1)..series.len() {
                let corr = pearson_correlation(&series[i], &series[j], n);
                if corr < min_corr {
                    min_corr = corr;
                }
                // Write symmetrically, skip if index exceeds grid bounds
                if (i as u16) < MAX_COMPONENTS && (j as u16) < MAX_COMPONENTS {
                    self.grid.set_direction(MatrixCell::new(i as u16, j as u16), corr);
                    self.grid.set_direction(MatrixCell::new(j as u16, i as u16), corr);
                }
            }
        }

        min_corr
    }

    /// The full N×N pairwise correlation grid (MAX_COMPONENTS × MAX_COMPONENTS).
    ///
    /// Populated after each `update_composite_index` call. Diagonal cells are `1.0`;
    /// off-diagonal `(i, j)` holds the Pearson correlation between component `i` and `j`.
    /// Cells beyond the number of active components retain their reset value (`0.0`).
    pub fn correlation_grid(&self) -> &MatrixGrid {
        &self.grid
    }
}

fn pearson_correlation(a: &[f64], b: &[f64], n: usize) -> f64 {
    if n < 2 {
        return 1.0;
    }
    let mean_a = a.iter().sum::<f64>() / n as f64;
    let mean_b = b.iter().sum::<f64>() / n as f64;

    let mut num = 0.0_f64;
    let mut denom_a = 0.0_f64;
    let mut denom_b = 0.0_f64;

    for k in 0..n {
        let da = a[k] - mean_a;
        let db = b[k] - mean_b;
        num += da * db;
        denom_a += da * da;
        denom_b += db * db;
    }

    let denom = (denom_a * denom_b).sqrt();
    if denom < 1e-12 {
        1.0
    } else {
        (num / denom).clamp(-1.0, 1.0)
    }
}

impl Default for IndexCorrelationBreakdown {
    fn default() -> Self {
        Self::new(20)
    }
}

impl CompositeIndexConsumer for IndexCorrelationBreakdown {
    fn update_composite_index(&mut self, ci: &CompositeIndex) {
        let snap: HashMap<String, f64> = ci.components.iter().cloned().collect();
        self.snapshots.push_back(snap);
        while self.snapshots.len() > self.window_size {
            self.snapshots.pop_front();
        }
        self.last_min_corr = self.recompute_corr();
    }


    fn reset(&mut self) {
        self.snapshots.clear();
        self.last_min_corr = 1.0;
        self.grid.reset();
    }

    fn is_ready(&self) -> bool {
        self.snapshots.len() >= 2
    }
}

use crate::contract::Param;

/// Typed configuration for [`IndexCorrelationBreakdown`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct IndexCorrelationBreakdownConfig {
    /// Rolling window of snapshots (minimum 2).
    pub window_size: Param<usize>,
}

impl Indicator for IndexCorrelationBreakdown {
    const ID: IndicatorId = IndicatorId::IndexCorrelationBreakdown;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::CompositeIndex];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::IndexCorrelationBreakdown),
        Output::matrix(IndicatorOutputId::IndexCorrelationBreakdownCorrelationGrid, ValueDomain::Centered),
    ];
    const COST: Cost = Cost::new(
        UpdateComplexity::Quadratic,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = IndexCorrelationBreakdownConfig;
    type Runtime = IndexCorrelationBreakdown;

    fn create(cfg: IndexCorrelationBreakdownConfig) -> IndexCorrelationBreakdown {
        IndexCorrelationBreakdown::new(cfg.window_size.resolved())
    }
}

impl crate::contract::Config for IndexCorrelationBreakdownConfig {
    fn defaults() -> Self {
        IndexCorrelationBreakdownConfig { window_size: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // window_size: rolling snapshot count for Pearson correlation — Class A period,
        // auto sweep range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for IndexCorrelationBreakdown {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::IndexCorrelationBreakdown, "Min Correlation", Color::hex(0xE91E63))
            .bounds(-1.0, 1.0)
            .precision(3)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::market_sample::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;

    fn make_ci(btc: f64, eth: f64) -> CompositeIndex {
        CompositeIndex {
            price: 1.0,
            components: vec![("BTC".to_string(), btc), ("ETH".to_string(), eth)],
            timestamp: 0,
        }
    }

    #[test]
    fn perfect_negative_correlation() {
        let mut ind = IndexCorrelationBreakdown::new(5);
        // BTC goes up, ETH goes down — perfect negative correlation
        for i in 0..5 {
            ind.update_composite_index(&make_ci(i as f64, (4 - i) as f64));
        }
        let c = ind.value();
        assert!(c < -0.9, "correlation should be near -1, got {c}");
    }

    #[test]
    fn returns_one_on_single_snapshot() {
        let mut ind = IndexCorrelationBreakdown::new(5);
        ind.update_composite_index(&make_ci(0.5, 0.5));
        let c = ind.value();
        assert!((c - 1.0).abs() < 1e-9);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = IndexCorrelationBreakdown::new(5);
        for i in 0..3 {
            ind.update_composite_index(&make_ci(i as f64, i as f64));
        }
        ind.reset();
        assert!(!ind.is_ready());
        let c = ind.value();
        assert!((c - 1.0).abs() < 1e-9);
    }

    #[test]
    fn factory_feeds_resolved_index_correlation_breakdown() {
        
        let mut f = IndicatorOrder::IndexCorrelationBreakdown(
            <<IndexCorrelationBreakdown as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // Feed 5 snapshots with BTC going up, ETH going down — strongly negative correlation
        for i in 0..5 {
            let ci = make_ci(i as f64 * 0.1, (4 - i) as f64 * 0.1);
            f.feed(0, MarketSample::CompositeIndex(&ci));
        }
        let v = f.primary();
        let c = v;
        // After 5 samples with perfectly inverse movements, min corr should be well below 1
        assert!(c < 1.0, "factory min correlation should be < 1.0, got {c}");
    }

    #[test]
    fn correlation_grid_labels() {
        use crate::engine::matrix_grid::Label;
        let ind = IndexCorrelationBreakdown::new(5);
        let g = ind.correlation_grid();
        assert_eq!(g.row_label(2), Label::Index(2));
        assert_eq!(g.col_label(0), Label::Index(0));
    }

    #[test]
    fn correlation_grid_fills_pairwise() {
        let mut ind = IndexCorrelationBreakdown::new(10);
        // BTC rises, ETH falls — perfect negative correlation between the two components.
        for i in 0..5 {
            ind.update_composite_index(&make_ci(i as f64, (4 - i) as f64));
        }

        let g = ind.correlation_grid();

        // Grid is fixed at MAX_COMPONENTS × MAX_COMPONENTS.
        assert_eq!(g.rows(), 32);
        assert_eq!(g.cols(), 32);

        // Diagonal must be 1.0 for BTC (row 0) and ETH (row 1).
        assert!((g.read_direction(MatrixCell::new(0, 0)) - 1.0).abs() < 1e-9,
            "diagonal (0,0) should be 1.0");
        assert!((g.read_direction(MatrixCell::new(1, 1)) - 1.0).abs() < 1e-9,
            "diagonal (1,1) should be 1.0");

        // BTC vs ETH is perfectly negatively correlated.
        let corr_01 = g.read_direction(MatrixCell::new(0, 1));
        assert!(corr_01 < 0.0, "off-diagonal (0,1) should be negative, got {corr_01}");

        // Grid must be symmetric.
        let corr_10 = g.read_direction(MatrixCell::new(1, 0));
        assert!((corr_01 - corr_10).abs() < 1e-9,
            "grid must be symmetric: (0,1)={corr_01} != (1,0)={corr_10}");
    }

    #[test]
    fn factory_exposes_correlation_matrix() {
        use crate::engine::indicator_id::IndicatorId;
        use crate::engine::contract_engine::IndicatorOutputId;
        let mut f = IndicatorOrder::IndexCorrelationBreakdown(
            <<IndexCorrelationBreakdown as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        ).build_solo().unwrap();
        for i in 0..5 {
            let ci = make_ci(i as f64 * 0.1, (4 - i) as f64 * 0.1);
            f.feed(0, MarketSample::CompositeIndex(&ci));
        }
        let g = f
            .grid(IndicatorOutputId::IndexCorrelationBreakdownCorrelationGrid)
            .expect("IndexCorrelationBreakdown emits the IndexCorrelationBreakdownCorrelationGrid matrix");
        assert_eq!(g.rows(), 32);
        assert_eq!(g.cols(), 32);

        // A scalar-only producer returns None for a foreign matrix id.
        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma).unwrap().build_solo().unwrap();
        assert!(sma.grid(IndicatorOutputId::IndexCorrelationBreakdownCorrelationGrid).is_none());
    }
}

impl IndexCorrelationBreakdown {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_min_corr
    }
}
