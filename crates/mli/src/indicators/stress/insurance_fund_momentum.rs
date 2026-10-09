//! InsuranceFundMomentum — EMA slope of insurance fund balance.
//!
//! Tracks an exponential moving average of the fund balance and reports
//! the difference between the new EMA and the previous EMA as slope.
//!
//! `new_ema = α × balance + (1 − α) × prev_ema`
//! `slope   = new_ema − prev_ema`
//!
//! where `α = 2 / (period + 1)`.
//!
//! Output: `Single(slope)`. Returns 0.0 until the EMA is initialized.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::insurance_fund_consumer::InsuranceFundConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::core::types::InsuranceFund;
use crate::engine::stream_kind::StreamKind;

/// EMA slope of insurance fund balance.
#[derive(Debug, Clone)]
pub struct InsuranceFundMomentum {
    alpha: f64,
    ema: f64,
    prev_ema: f64,
    last_slope: f64,
    initialized: bool,
}

impl InsuranceFundMomentum {
    /// Create a new indicator. `period` is clamped to at least 2.
    pub fn new(period: usize) -> Self {
        let period = period.max(2);
        let alpha = 2.0 / (period as f64 + 1.0);
        Self {
            alpha,
            ema: 0.0,
            prev_ema: 0.0,
            last_slope: 0.0,
            initialized: false,
        }
    }
}

impl Default for InsuranceFundMomentum {
    fn default() -> Self {
        Self::new(14)
    }
}

/// Typed configuration for [`InsuranceFundMomentum`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct InsuranceFundMomentumConfig {
    pub period: Param<usize>,
}

impl Indicator for InsuranceFundMomentum {
    const ID: IndicatorId = IndicatorId::InsuranceFundMomentum;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::InsuranceFund];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::InsuranceFundMomentum)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = InsuranceFundMomentumConfig;
    type Runtime = InsuranceFundMomentum;

    fn create(cfg: InsuranceFundMomentumConfig) -> InsuranceFundMomentum {
        InsuranceFundMomentum::new(cfg.period.resolved())
    }
}

impl InsuranceFundConsumer for InsuranceFundMomentum {
    fn update_insurance_fund(&mut self, ins: &InsuranceFund) {
        if !self.initialized {
            self.ema = ins.balance;
            self.prev_ema = ins.balance;
            self.initialized = true;
        } else {
            self.prev_ema = self.ema;
            self.ema = self.alpha * ins.balance + (1.0 - self.alpha) * self.prev_ema;
            self.last_slope = self.ema - self.prev_ema;
        }
    }


    fn reset(&mut self) {
        self.ema = 0.0;
        self.prev_ema = 0.0;
        self.last_slope = 0.0;
        self.initialized = false;
    }

    fn is_ready(&self) -> bool {
        self.initialized
    }
}

impl crate::contract::Config for InsuranceFundMomentumConfig {
    fn defaults() -> Self {
        InsuranceFundMomentumConfig { period: Param::Solo(14) }
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


impl Render for InsuranceFundMomentum {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::InsuranceFundMomentum, "Insurance Fund Momentum", Color::hex(0xAB47BC))
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
    fn rising_balance_positive_slope() {
        let mut ind = InsuranceFundMomentum::new(5);
        // Feed increasing values; EMA will follow upward → slope > 0
        for v in [10000.0, 11000.0, 12000.0, 13000.0, 14000.0] {
            ind.update_insurance_fund(&make_fund(v));
        }
        let s = ind.value();
        assert!(s > 0.0, "slope should be positive for rising fund, got {s}");
    }

    #[test]
    fn declining_balance_negative_slope() {
        let mut ind = InsuranceFundMomentum::new(5);
        for v in [14000.0, 13000.0, 12000.0, 11000.0, 10000.0] {
            ind.update_insurance_fund(&make_fund(v));
        }
        let s = ind.value();
        assert!(s < 0.0, "slope should be negative for declining fund, got {s}");
    }

    #[test]
    fn first_update_zero_slope() {
        let mut ind = InsuranceFundMomentum::new(5);
        ind.update_insurance_fund(&make_fund(10000.0));
        assert_eq!(ind.value(), 0.0, "first update should give 0 slope");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = InsuranceFundMomentum::new(5);
        ind.update_insurance_fund(&make_fund(10000.0));
        ind.update_insurance_fund(&make_fund(11000.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_insurance_fund_momentum() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::InsuranceFundMomentum(<<InsuranceFundMomentum as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let fund1 = InsuranceFund { balance: 10_000.0, timestamp: 1 };
        let fund2 = InsuranceFund { balance: 11_000.0, timestamp: 2 };
        let fund3 = InsuranceFund { balance: 12_000.0, timestamp: 3 };
        f.feed(0, MarketSample::InsuranceFund(&fund1));
        f.feed(0, MarketSample::InsuranceFund(&fund2));
        f.feed(0, MarketSample::InsuranceFund(&fund3));
        // After first feed: slope=0; after subsequent feeds slope > 0 for rising
        assert!(f.primary() > 0.0, "slope should be positive for rising fund");
    }
}

impl InsuranceFundMomentum {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_slope
    }
}
