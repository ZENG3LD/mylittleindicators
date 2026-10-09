//! FundDepletionRate — rolling linear slope of insurance fund balance.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::insurance_fund_consumer::InsuranceFundConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::core::types::InsuranceFund;
use crate::engine::stream_kind::StreamKind;

/// Rolling linear slope of insurance fund balance.
///
/// slope = (latest − oldest) / (n − 1)
///
/// Output: `Single(slope)`. Returns 0.0 until at least two snapshots.
#[derive(Debug, Clone)]
pub struct FundDepletionRate {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl FundDepletionRate {
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

impl Default for FundDepletionRate {
    fn default() -> Self {
        Self::new(14)
    }
}

/// Typed configuration for [`FundDepletionRate`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FundDepletionRateConfig {
    pub period: Param<usize>,
}

impl Indicator for FundDepletionRate {
    const ID: IndicatorId = IndicatorId::FundDepletionRate;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::InsuranceFund];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::FundDepletionRate)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    type Config = FundDepletionRateConfig;
    type Runtime = FundDepletionRate;

    fn create(cfg: FundDepletionRateConfig) -> FundDepletionRate {
        FundDepletionRate::new(cfg.period.resolved())
    }
}

impl InsuranceFundConsumer for FundDepletionRate {
    fn update_insurance_fund(&mut self, ins: &InsuranceFund) {
        self.history.push_back(ins.balance);
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

impl crate::contract::Config for FundDepletionRateConfig {
    fn defaults() -> Self {
        FundDepletionRateConfig { period: Param::Solo(14) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
}


impl Render for FundDepletionRate {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::FundDepletionRate, "Fund Depletion Rate", Color::hex(0xEF5350))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fund(balance: f64) -> InsuranceFund {
        InsuranceFund { balance, timestamp: 0 }
    }

    #[test]
    fn declining_balance_negative_slope() {
        let mut ind = FundDepletionRate::new(5);
        for v in [10000.0, 9000.0, 8000.0, 7000.0, 6000.0] {
            ind.update_insurance_fund(&make_fund(v));
        }
        let s = ind.value();
        assert!(s < 0.0, "slope should be negative for declining fund, got {s}");
    }

    #[test]
    fn rising_balance_positive_slope() {
        let mut ind = FundDepletionRate::new(5);
        for v in [6000.0, 7000.0, 8000.0, 9000.0, 10000.0] {
            ind.update_insurance_fund(&make_fund(v));
        }
        let s = ind.value();
        assert!(s > 0.0, "slope should be positive for rising fund, got {s}");
    }

    #[test]
    fn not_ready_with_one_sample() {
        let mut ind = FundDepletionRate::new(5);
        ind.update_insurance_fund(&make_fund(1000.0));
        assert!(!ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = FundDepletionRate::new(3);
        ind.update_insurance_fund(&make_fund(100.0));
        ind.update_insurance_fund(&make_fund(200.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_fund_depletion_rate() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::FundDepletionRate(<<FundDepletionRate as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let fund1 = InsuranceFund { balance: 100_000.0, timestamp: 1 };
        let fund2 = InsuranceFund { balance: 90_000.0, timestamp: 2 };
        let fund3 = InsuranceFund { balance: 80_000.0, timestamp: 3 };
        f.feed(0, MarketSample::InsuranceFund(&fund1));
        f.feed(0, MarketSample::InsuranceFund(&fund2));
        f.feed(0, MarketSample::InsuranceFund(&fund3));
        // slope = (80000 - 100000) / 2 = -10000
        assert!(f.primary() < 0.0, "slope should be negative");
    }
}

impl FundDepletionRate {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_slope
    }
}
