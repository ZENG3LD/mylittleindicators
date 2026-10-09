//! MarkPriceMomentum — rolling linear slope of mark price.
//!
//! slope = (latest − oldest) / (n − 1)
//!
//! Analogous to BasisMomentum but consuming MarkPrice snapshots.
//!
//! Output: `Single(slope)`. Returns 0.0 until at least two snapshots.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::MarkPrice;

/// Rolling linear slope of mark price.
#[derive(Debug, Clone)]
pub struct MarkPriceMomentum {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl MarkPriceMomentum {
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

impl Default for MarkPriceMomentum {
    fn default() -> Self {
        Self::new(14)
    }
}

/// Typed configuration for [`MarkPriceMomentum`]. Consumes `MarkPrice` stream.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct MarkPriceMomentumConfig {
    pub period: crate::contract::Param<usize>,
}

impl Indicator for MarkPriceMomentum {
    const ID: IndicatorId = IndicatorId::MarkPriceMomentum;
    /// Mark-price indicator — not a pluggable family.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::MarkPrice];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::MarkPriceMomentum)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    type Config = MarkPriceMomentumConfig;
    type Runtime = MarkPriceMomentum;

    fn create(cfg: MarkPriceMomentumConfig) -> MarkPriceMomentum {
        MarkPriceMomentum::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for MarkPriceMomentumConfig {
    fn defaults() -> Self {
        MarkPriceMomentumConfig { period: crate::contract::Param::Solo(14) }
    }
    fn machine_defaults() -> Self {
        // period: Class A (rolling slope lookback) → auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for MarkPriceMomentum {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::MarkPriceMomentum, "Mark Price Momentum", Color::hex(0x42A5F5))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

impl MarkPriceConsumer for MarkPriceMomentum {
    fn update_mark(&mut self, mp: &MarkPrice) {
        self.history.push_back(mp.mark_price);
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

    #[test]
    fn factory_feeds_resolved_mark_price() {
        let mut f = IndicatorOrder::MarkPriceMomentum(<<MarkPriceMomentum as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let mp = MarkPrice {
            mark_price: 50_100.0,
            index_price: None,
            funding_rate: None,
            timestamp: 1,
            ..Default::default()
        };
        f.feed(0, MarketSample::MarkPrice(&mp));
        f.feed(0, MarketSample::MarkPrice(&mp));
        // After two feeds the slope is computable (0 here since price is constant)
        let s = f.primary();
        // constant price → slope = 0
        assert_eq!(s, 0.0);
    }

    fn make_mp(mark_price: f64) -> MarkPrice {
        MarkPrice {
            mark_price,
            index_price: None,
            funding_rate: None,
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn rising_price_gives_positive_slope() {
        let mut ind = MarkPriceMomentum::new(5);
        for p in [50000.0, 50100.0, 50200.0, 50300.0, 50400.0] {
            ind.update_mark(&make_mp(p));
        }
        let s = ind.value();
        assert!(s > 0.0, "slope should be positive, got {s}");
    }

    #[test]
    fn falling_price_gives_negative_slope() {
        let mut ind = MarkPriceMomentum::new(5);
        for p in [50400.0, 50300.0, 50200.0, 50100.0, 50000.0] {
            ind.update_mark(&make_mp(p));
        }
        let s = ind.value();
        assert!(s < 0.0, "slope should be negative, got {s}");
    }

    #[test]
    fn exact_slope_calculation() {
        let mut ind = MarkPriceMomentum::new(3);
        // Values: 100, 102, 106 → slope = (106 - 100) / (3 - 1) = 3.0
        for p in [100.0, 102.0, 106.0] {
            ind.update_mark(&make_mp(p));
        }
        let s = ind.value();
        assert!((s - 3.0).abs() < 1e-9, "expected slope=3.0, got {s}");
    }

    #[test]
    fn not_ready_with_single_sample() {
        let mut ind = MarkPriceMomentum::new(5);
        ind.update_mark(&make_mp(50000.0));
        assert!(!ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = MarkPriceMomentum::new(5);
        for p in [50000.0, 50100.0, 50200.0] {
            ind.update_mark(&make_mp(p));
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }
}

impl MarkPriceMomentum {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_slope
    }
}
