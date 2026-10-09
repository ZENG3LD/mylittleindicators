//! DeltaExposureFlow — rolling linear slope of option delta.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::option_greeks_consumer::OptionGreeksConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OptionGreeks;

/// Computes the linear slope of option delta over the last `period` snapshots.
///
/// slope = (latest − oldest) / (n − 1)
///
/// Output: `Single(slope)`. Returns 0.0 until at least two snapshots.
#[derive(Clone, Debug)]
pub struct DeltaExposureFlow {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl DeltaExposureFlow {
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

impl Default for DeltaExposureFlow {
    fn default() -> Self {
        Self::new(14)
    }
}

impl OptionGreeksConsumer for DeltaExposureFlow {
    fn update_option_greeks(&mut self, g: &OptionGreeks) {
        self.history.push_back(g.delta);
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

/// Config for [`DeltaExposureFlow`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DeltaExposureFlowConfig {
    pub period: Param<usize>,
}

impl Indicator for DeltaExposureFlow {
    const ID: IndicatorId = IndicatorId::DeltaExposureFlow;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::OptionGreeks];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::DeltaExposureFlow)];
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = DeltaExposureFlowConfig;
    type Runtime = DeltaExposureFlow;

    fn create(cfg: DeltaExposureFlowConfig) -> DeltaExposureFlow {
        DeltaExposureFlow::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for DeltaExposureFlowConfig {
    fn defaults() -> Self {
        DeltaExposureFlowConfig { period: Param::Solo(14) }
    }
    fn machine_defaults() -> Self {
        // period: A period — auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for DeltaExposureFlow {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::DeltaExposureFlow, "Delta Exposure Flow", Color::hex(0x42A5F5))
            .zero_baseline()
            .precision(6)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_greeks(delta: f64) -> OptionGreeks {
        OptionGreeks {
            delta,
            gamma: 0.0,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
            mark_iv: 0.0,
            bid_iv: None,
            ask_iv: None,
            timestamp: 0,
        }
    }

    #[test]
    fn rising_delta_positive_slope() {
        let mut ind = DeltaExposureFlow::new(5);
        for v in [-0.5, -0.3, -0.1, 0.1, 0.3] {
            ind.update_option_greeks(&make_greeks(v));
        }
        let s = ind.value();
        assert!(s > 0.0, "slope should be positive, got {s}");
    }

    #[test]
    fn falling_delta_negative_slope() {
        let mut ind = DeltaExposureFlow::new(5);
        for v in [0.3, 0.1, -0.1, -0.3, -0.5] {
            ind.update_option_greeks(&make_greeks(v));
        }
        let s = ind.value();
        assert!(s < 0.0, "slope should be negative, got {s}");
    }

    #[test]
    fn reset_clears() {
        let mut ind = DeltaExposureFlow::new(3);
        ind.update_option_greeks(&make_greeks(0.1));
        ind.update_option_greeks(&make_greeks(0.2));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_delta_exposure_flow() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::DeltaExposureFlow(DeltaExposureFlowConfig { period: Param::Solo(5) })
            .build_solo()
            .unwrap();
        // Rising delta → positive slope. Use vega=9999.0 (unused field) to prove only delta is read.
        for delta in [-0.5_f64, -0.3, -0.1, 0.1, 0.3] {
            let g = OptionGreeks { delta, gamma: 0.0, vega: 9999.0, theta: 0.0, rho: 0.0,
                mark_iv: 0.0, bid_iv: None, ask_iv: None, timestamp: 0 };
            f.feed(0, MarketSample::OptionGreeks(&g));
        }
        assert!(f.primary() > 0.0, "slope should be positive for rising delta, got {}", f.primary());
    }
}

impl DeltaExposureFlow {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_slope
    }
}
