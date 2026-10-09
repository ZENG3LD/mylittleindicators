//! FundStressDetector — detects rapid depletion of the insurance fund.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::insurance_fund_consumer::InsuranceFundConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::core::types::InsuranceFund;
use crate::engine::stream_kind::StreamKind;

/// Detects stress on the insurance fund via rapid balance depletion.
///
/// Computes the rolling linear slope of the balance. Fires `Signal(1)` when
/// `slope < -threshold` (fund depleting faster than threshold per step).
///
/// Output: `Signal(i8)`. Returns 0 until at least two snapshots.
#[derive(Debug, Clone)]
pub struct FundStressDetector {
    period: usize,
    threshold: f64,
    history: VecDeque<f64>,
    last_signal: i8,
}

impl FundStressDetector {
    /// Create a new indicator.
    ///
    /// - `period`: rolling window size (clamped to at least 2).
    /// - `threshold`: depletion rate that triggers stress signal (default 1000.0).
    ///   Signal fires when `slope < -threshold.abs()`.
    pub fn new(period: usize, threshold: f64) -> Self {
        let period = period.max(2);
        Self {
            period,
            threshold: threshold.abs(),
            history: VecDeque::with_capacity(period),
            last_signal: 0,
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

impl Default for FundStressDetector {
    fn default() -> Self {
        Self::new(14, 1000.0)
    }
}

/// Typed configuration for [`FundStressDetector`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FundStressConfig {
    pub period: Param<usize>,
    /// Depletion rate magnitude that triggers a stress signal.
    /// Signal fires when `slope < -threshold`.
    pub threshold: Param<f64>,
}

impl Indicator for FundStressDetector {
    const ID: IndicatorId = IndicatorId::FundStressDetector;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::InsuranceFund];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::FundStressDetector)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    type Config = FundStressConfig;
    type Runtime = FundStressDetector;

    fn create(cfg: FundStressConfig) -> FundStressDetector {
        FundStressDetector::new(cfg.period.resolved(), cfg.threshold.resolved())
    }
}

impl InsuranceFundConsumer for FundStressDetector {
    fn update_insurance_fund(&mut self, ins: &InsuranceFund) {
        self.history.push_back(ins.balance);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
        let slope = self.compute_slope();
        self.last_signal = if slope < -self.threshold { 1 } else { 0 };
    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= 2
    }
}

impl crate::contract::Config for FundStressConfig {
    fn defaults() -> Self {
        FundStressConfig {
            period: Param::Solo(14),
            threshold: Param::Solo(1000.0),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // period: Class A → auto range(2,4048,1) — leave as-is
        // threshold: raw insurance-fund balance change per step (USDT magnitude).
        // Instrument-dimensioned — auto leaves f64 as Solo. Provide discrete presets
        // covering typical exchange fund depletion sensitivities (100 to 100 000 USDT/step).
        s.threshold = Param::many(vec![
            100.0_f64, 500.0, 1000.0, 5000.0, 10000.0, 50000.0, 100000.0,
        ]);
        s
    }
}


impl Render for FundStressDetector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::FundStressDetector, "Fund Stress", Color::hex(0xFF1744))
            .precision(0)
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
    fn stress_detected_on_rapid_depletion() {
        let mut ind = FundStressDetector::new(5, 1000.0);
        for v in [100_000.0, 90_000.0, 80_000.0, 70_000.0, 60_000.0] {
            ind.update_insurance_fund(&make_fund(v));
        }
        assert_eq!(ind.value() as i8, 1, "should detect stress for rapid depletion");
    }

    #[test]
    fn no_stress_on_slow_depletion() {
        let mut ind = FundStressDetector::new(5, 1000.0);
        for v in [100_000.0, 99_900.0, 99_800.0, 99_700.0, 99_600.0] {
            ind.update_insurance_fund(&make_fund(v));
        }
        assert_eq!(ind.value() as i8, 0, "should not detect stress for slow depletion");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = FundStressDetector::new(3, 500.0);
        ind.update_insurance_fund(&make_fund(100_000.0));
        ind.update_insurance_fund(&make_fund(50_000.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value() as i8, 0);
    }

    #[test]
    fn factory_feeds_resolved_fund_stress_detector() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::FundStressDetector(<<FundStressDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let fund1 = InsuranceFund { balance: 500_000.0, timestamp: 1 };
        let fund2 = InsuranceFund { balance: 490_000.0, timestamp: 2 };
        let fund3 = InsuranceFund { balance: 480_000.0, timestamp: 3 };
        f.feed(0, MarketSample::InsuranceFund(&fund1));
        f.feed(0, MarketSample::InsuranceFund(&fund2));
        f.feed(0, MarketSample::InsuranceFund(&fund3));
        // slope = -10000, threshold=1000 → stress signal = 1
        assert_eq!(f.primary(), 1.0, "should detect stress signal");
    }
}

impl FundStressDetector {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }
}
