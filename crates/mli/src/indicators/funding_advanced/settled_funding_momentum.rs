//! SettledFundingMomentum — rolling linear slope of settled funding rates.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::funding_settlement_consumer::FundingSettlementConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::core::types::FundingSettlement;
use crate::engine::stream_kind::StreamKind;

/// Rolling linear slope of confirmed funding settlement rates.
///
/// slope = (latest − oldest) / (n − 1)
///
/// Returns 0.0 until at least two settlements have been received.
///
/// Output: `Single(slope)`.
#[derive(Debug, Clone)]
pub struct SettledFundingMomentum {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl SettledFundingMomentum {
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

impl Default for SettledFundingMomentum {
    fn default() -> Self {
        Self::new(8)
    }
}

/// Typed configuration for [`SettledFundingMomentum`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SettledFundingMomentumConfig {
    pub period: Param<usize>,
}

impl Indicator for SettledFundingMomentum {
    const ID: IndicatorId = IndicatorId::SettledFundingMomentum;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::FundingSettlement];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::SettledFundingMomentum)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    type Config = SettledFundingMomentumConfig;
    type Runtime = Self;

    fn create(cfg: SettledFundingMomentumConfig) -> Self {
        Self::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for SettledFundingMomentumConfig {
    fn defaults() -> Self {
        SettledFundingMomentumConfig { period: Param::Solo(8) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1); already set by auto.
        Self::machine_defaults_auto()
    }
}


impl Render for SettledFundingMomentum {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::SettledFundingMomentum, "Settled Funding Momentum", Color::hex(0x4CAF50))
            .zero_baseline()
            .precision(6)
            .build()
    }
}

impl FundingSettlementConsumer for SettledFundingMomentum {
    fn update_funding_settlement(&mut self, fs: &FundingSettlement) {
        self.history.push_back(fs.settled_rate);
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
    fn factory_feeds_resolved_settled_funding_momentum() {
        let mut f = IndicatorOrder::SettledFundingMomentum(
            <<SettledFundingMomentum as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let fs = FundingSettlement { settled_rate: 0.0001, settlement_time: 0, timestamp: 0 };
        f.feed(0, MarketSample::FundingSettlement(&fs));
        let _ = f.primary();
    }

    fn make_settlement(rate: f64) -> FundingSettlement {
        FundingSettlement {
            settled_rate: rate,
            settlement_time: 0,
            timestamp: 0,
        }
    }

    #[test]
    fn rising_rates_give_positive_slope() {
        let mut ind = SettledFundingMomentum::new(5);
        for v in [0.001, 0.002, 0.003, 0.004, 0.005] {
            ind.update_funding_settlement(&make_settlement(v));
        }
        let s = ind.value();
        assert!(s > 0.0, "slope should be positive, got {s}");
    }

    #[test]
    fn falling_rates_give_negative_slope() {
        let mut ind = SettledFundingMomentum::new(5);
        for v in [0.005, 0.004, 0.003, 0.002, 0.001] {
            ind.update_funding_settlement(&make_settlement(v));
        }
        let s = ind.value();
        assert!(s < 0.0, "slope should be negative, got {s}");
    }

    #[test]
    fn not_ready_before_two_events() {
        let mut ind = SettledFundingMomentum::new(5);
        assert!(!ind.is_ready());
        ind.update_funding_settlement(&make_settlement(0.001));
        assert!(!ind.is_ready());
        ind.update_funding_settlement(&make_settlement(0.002));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = SettledFundingMomentum::new(3);
        ind.update_funding_settlement(&make_settlement(0.001));
        ind.update_funding_settlement(&make_settlement(0.002));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }
}

impl SettledFundingMomentum {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_slope
    }
}
