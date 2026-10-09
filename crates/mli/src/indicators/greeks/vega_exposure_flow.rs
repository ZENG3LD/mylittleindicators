//! VegaExposureFlow — rolling linear slope of option vega.
//!
//! Mirrors the approach of `DeltaExposureFlow` but tracks vega instead of delta.
//! A rising slope indicates increasing vega (market pricing more vol risk).
//!
//! Output: `Single(slope)`.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::option_greeks_consumer::OptionGreeksConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OptionGreeks;

/// Rolling linear slope of option vega over the last `period` snapshots.
///
/// `slope = (latest - oldest) / (n - 1)`
///
/// Returns 0.0 until at least two snapshots are available.
#[derive(Clone, Debug)]
pub struct VegaExposureFlow {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl VegaExposureFlow {
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

impl Default for VegaExposureFlow {
    fn default() -> Self {
        Self::new(14)
    }
}

impl OptionGreeksConsumer for VegaExposureFlow {
    fn update_option_greeks(&mut self, g: &OptionGreeks) {
        self.history.push_back(g.vega);
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

/// Config for [`VegaExposureFlow`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VegaExposureFlowConfig {
    pub period: Param<usize>,
}

impl Indicator for VegaExposureFlow {
    const ID: IndicatorId = IndicatorId::VegaExposureFlow;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::OptionGreeks];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::VegaExposureFlow)];
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = VegaExposureFlowConfig;
    type Runtime = VegaExposureFlow;

    fn create(cfg: VegaExposureFlowConfig) -> VegaExposureFlow {
        VegaExposureFlow::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for VegaExposureFlowConfig {
    fn defaults() -> Self {
        VegaExposureFlowConfig { period: Param::Solo(14) }
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


impl Render for VegaExposureFlow {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::VegaExposureFlow, "Vega Exposure Flow", Color::hex(0x66BB6A))
            .zero_baseline()
            .precision(6)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_greeks(vega: f64) -> OptionGreeks {
        OptionGreeks {
            delta: 0.0,
            gamma: 0.0,
            vega,
            theta: 0.0,
            rho: 0.0,
            mark_iv: 0.0,
            bid_iv: None,
            ask_iv: None,
            timestamp: 0,
        }
    }

    #[test]
    fn rising_vega_positive_slope() {
        let mut ind = VegaExposureFlow::new(5);
        for v in [10.0, 20.0, 30.0, 40.0, 50.0] {
            ind.update_option_greeks(&make_greeks(v));
        }
        let s = ind.value();
        assert!(s > 0.0, "slope should be positive, got {s}");
    }

    #[test]
    fn falling_vega_negative_slope() {
        let mut ind = VegaExposureFlow::new(5);
        for v in [50.0, 40.0, 30.0, 20.0, 10.0] {
            ind.update_option_greeks(&make_greeks(v));
        }
        let s = ind.value();
        assert!(s < 0.0, "slope should be negative, got {s}");
    }

    #[test]
    fn not_ready_until_two_snapshots() {
        let mut ind = VegaExposureFlow::new(5);
        ind.update_option_greeks(&make_greeks(10.0));
        assert!(!ind.is_ready());
        ind.update_option_greeks(&make_greeks(20.0));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = VegaExposureFlow::new(3);
        ind.update_option_greeks(&make_greeks(10.0));
        ind.update_option_greeks(&make_greeks(20.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_vega_exposure_flow() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::VegaExposureFlow(VegaExposureFlowConfig { period: Param::Solo(5) })
            .build_solo()
            .unwrap();
        // Rising vega → positive slope. delta=9999.0 (unused) to prove only vega is read.
        for vega in [10.0_f64, 20.0, 30.0, 40.0, 50.0] {
            let g = OptionGreeks { delta: 9999.0, gamma: 0.0, vega, theta: 0.0, rho: 0.0,
                mark_iv: 0.0, bid_iv: None, ask_iv: None, timestamp: 0 };
            f.feed(0, MarketSample::OptionGreeks(&g));
        }
        assert!(f.primary() > 0.0, "slope should be positive for rising vega, got {}", f.primary());
    }
}

impl VegaExposureFlow {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_slope
    }
}
