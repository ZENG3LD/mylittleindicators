//! IndexTrackingError — rolling standard deviation of (index_price - composite_price).
//!
//! Dual consumer: `IndexPriceConsumer` + `CompositeIndexConsumer`.
//!
//! Logic:
//! - On each update, if both last values are > 0, push `(last_index - last_composite)` to window.
//! - `tracking_error` = rolling std of differences in window.
//!
//! Output: `Single(tracking_error)`.

use std::collections::VecDeque;

use crate::engine::streams::composite_index_consumer::CompositeIndexConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::index_price_consumer::IndexPriceConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::core::types::{CompositeIndex, IndexPrice};
use crate::engine::stream_kind::StreamKind;

/// Rolling tracking error between index price and composite price.
///
/// Implements both `IndexPriceConsumer` and `CompositeIndexConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Debug, Clone)]
pub struct IndexTrackingError {
    window: usize,
    last_index: f64,
    last_composite: f64,
    diffs: VecDeque<f64>,
    last_error: f64,
}

impl IndexTrackingError {
    /// Create a new indicator.
    ///
    /// - `window` — rolling window for std computation (minimum 2, default 20).
    pub fn new(window: usize) -> Self {
        let w = window.max(2);
        Self {
            window: w,
            last_index: 0.0,
            last_composite: 0.0,
            diffs: VecDeque::with_capacity(w),
            last_error: 0.0,
        }
    }

    fn push_diff(&mut self) {
        if self.last_index <= 0.0 || self.last_composite <= 0.0 {
            return;
        }
        let d = self.last_index - self.last_composite;
        if self.diffs.len() >= self.window {
            self.diffs.pop_front();
        }
        self.diffs.push_back(d);
        self.recompute_error();
    }

    fn recompute_error(&mut self) {
        if self.diffs.len() < 2 {
            self.last_error = 0.0;
            return;
        }
        let n = self.diffs.len() as f64;
        let mean = self.diffs.iter().sum::<f64>() / n;
        let variance = self.diffs.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / n;
        self.last_error = variance.sqrt();
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> f64 {
        self.last_error
    }

    /// True when at least 2 paired observations have been collected.
    pub fn indicator_is_ready(&self) -> bool {
        self.diffs.len() >= 2
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.last_index = 0.0;
        self.last_composite = 0.0;
        self.diffs.clear();
        self.last_error = 0.0;
    }
}

impl Default for IndexTrackingError {
    fn default() -> Self {
        Self::new(20)
    }
}

impl IndexPriceConsumer for IndexTrackingError {
    fn update_index_price(&mut self, ip: &IndexPrice) {
        self.last_index = ip.price;
        self.push_diff();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl CompositeIndexConsumer for IndexTrackingError {
    fn update_composite_index(&mut self, ci: &CompositeIndex) {
        self.last_composite = ci.price;
        self.push_diff();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

/// Typed configuration for [`IndexTrackingError`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct IndexTrackingErrorConfig {
    /// Rolling window size (minimum 2).
    pub window: Param<usize>,
}

impl Indicator for IndexTrackingError {
    const ID: IndicatorId = IndicatorId::IndexTrackingError;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::IndexPrice, StreamKind::CompositeIndex];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::IndexTrackingError)];
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = IndexTrackingErrorConfig;
    type Runtime = IndexTrackingError;

    fn create(cfg: IndexTrackingErrorConfig) -> IndexTrackingError {
        IndexTrackingError::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for IndexTrackingErrorConfig {
    fn defaults() -> Self {
        IndexTrackingErrorConfig { window: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // window (usize): Class A period — rolling window for std computation;
        //   auto gives range(2,4048,1) ✓
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for IndexTrackingError {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::IndexTrackingError, "Index Tracking Error", Color::hex(0xE91E63))
            .precision(6)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ip(price: f64) -> IndexPrice {
        IndexPrice { price, timestamp: 1000, ..Default::default()}
    }

    fn make_ci(price: f64) -> CompositeIndex {
        CompositeIndex { price, components: vec![], timestamp: 1000 }
    }

    #[test]
    fn zero_error_when_perfectly_tracking() {
        let mut ind = IndexTrackingError::new(5);
        for _ in 0..5 {
            ind.update_index_price(&make_ip(100.0));
            ind.update_composite_index(&make_ci(100.0));
        }
        let err = ind.indicator_value();
        assert!(err < 1e-9, "err={err}");
    }

    #[test]
    fn nonzero_error_when_diverging() {
        let mut ind = IndexTrackingError::new(5);
        // Alternate offsets: diff = +1, -1, +1, -1, +1
        for i in 0..5 {
            let offset = if i % 2 == 0 { 1.0_f64 } else { -1.0_f64 };
            ind.update_index_price(&make_ip(100.0 + offset));
            ind.update_composite_index(&make_ci(100.0));
        }
        let err = ind.indicator_value();
        assert!(err > 0.0, "err={err}");
    }

    #[test]
    fn not_ready_before_two_observations() {
        let mut ind = IndexTrackingError::new(5);
        ind.update_index_price(&make_ip(100.0));
        ind.update_composite_index(&make_ci(100.0));
        assert!(!ind.indicator_is_ready(), "single observation should not be ready");
        ind.update_index_price(&make_ip(100.0));
        ind.update_composite_index(&make_ci(100.0));
        assert!(ind.indicator_is_ready());
    }

    #[test]
    fn constant_positive_spread_gives_zero_std() {
        let mut ind = IndexTrackingError::new(5);
        // Constant spread of 5.0 every time
        for _ in 0..6 {
            ind.update_index_price(&make_ip(105.0));
            ind.update_composite_index(&make_ci(100.0));
        }
        let err = ind.indicator_value();
        assert!(err < 1e-9, "constant spread should yield std≈0, got {err}");
    }

    #[test]
    fn factory_feeds_index_tracking_error() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::IndexTrackingError(IndexTrackingErrorConfig { window: Param::Solo(20) }).build_solo().unwrap();
        for _ in 0..5 {
            let ip = make_ip(100.0);
            let ci = make_ci(99.0);
            f.feed(0, MarketSample::IndexPrice(&ip));
            f.feed(0, MarketSample::CompositeIndex(&ci));
        }
        let err = f.primary();
        assert!(err < 1e-9, "constant 1.0 spread should yield std≈0, got {err}");
    }
}
