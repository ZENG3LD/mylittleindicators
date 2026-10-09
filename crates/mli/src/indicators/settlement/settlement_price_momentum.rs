//! SettlementPriceMomentum — rolling linear slope of contract settlement price.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::settlement_event_consumer::SettlementEventConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Render, SourceAxis, RenderSpec, Color, Store, StoreKind, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::SettlementEvent;

/// Rolling linear slope of contract settlement price.
///
/// slope = (latest − oldest) / (n − 1)
///
/// Output: `Single(slope)`. Returns 0.0 until at least two events.
#[derive(Debug, Clone)]
pub struct SettlementPriceMomentum {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl SettlementPriceMomentum {
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
        (self.history[n - 1] - self.history[0]) / (n as f64 - 1.0)
    }
}

impl Default for SettlementPriceMomentum {
    fn default() -> Self {
        Self::new(14)
    }
}

impl SettlementEventConsumer for SettlementPriceMomentum {
    fn update_settlement(&mut self, s: &SettlementEvent) {
        self.history.push_back(s.settlement_price);
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

use crate::contract::Param;

/// Typed configuration for [`SettlementPriceMomentum`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SettlementPriceMomentumConfig {
    /// Rolling window size (minimum 2).
    pub period: Param<usize>,
}

impl Indicator for SettlementPriceMomentum {
    const ID: IndicatorId = IndicatorId::SettlementPriceMomentum;
    /// Settlement price slope — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Settlement];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::SettlementPriceMomentum)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    type Config = SettlementPriceMomentumConfig;
    type Runtime = Self;

    fn create(cfg: SettlementPriceMomentumConfig) -> Self {
        Self::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for SettlementPriceMomentumConfig {
    fn defaults() -> Self {
        SettlementPriceMomentumConfig { period: Param::Solo(14) }
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


impl Render for SettlementPriceMomentum {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::SettlementPriceMomentum, "Settlement Price Momentum", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_settlement(price: f64) -> SettlementEvent {
        SettlementEvent {
            settlement_price: price,
            settlement_time: 0,
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn rising_price_positive_slope() {
        let mut ind = SettlementPriceMomentum::new(5);
        for v in [100.0, 110.0, 120.0, 130.0, 140.0] {
            ind.update_settlement(&make_settlement(v));
        }
        let s = ind.value();
        assert!(s > 0.0, "slope should be positive, got {s}");
    }

    #[test]
    fn falling_price_negative_slope() {
        let mut ind = SettlementPriceMomentum::new(5);
        for v in [140.0, 130.0, 120.0, 110.0, 100.0] {
            ind.update_settlement(&make_settlement(v));
        }
        let s = ind.value();
        assert!(s < 0.0, "slope should be negative, got {s}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = SettlementPriceMomentum::new(3);
        ind.update_settlement(&make_settlement(100.0));
        ind.update_settlement(&make_settlement(200.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_settlement() {
        use crate::engine::contract_engine::IndicatorOrder;
        
        let mut f = IndicatorOrder::SettlementPriceMomentum(
            <<SettlementPriceMomentum as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        for price in [100.0_f64, 110.0, 120.0] {
            let ev = SettlementEvent {
                settlement_price: price,
                settlement_time: 0,
                timestamp: 0,
                ..Default::default()
            };
            f.feed(0, crate::contract::MarketSample::Settlement(&ev));
        }
        // slope = (120 - 100) / (3-1) = 10.0
        let slope = f.primary();
        assert!(slope > 0.0, "slope should be positive, got {slope}");
    }
}

impl SettlementPriceMomentum {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_slope
    }
}
