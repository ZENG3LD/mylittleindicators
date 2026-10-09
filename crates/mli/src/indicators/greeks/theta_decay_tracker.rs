//! ThetaDecayTracker — rolling cumulative theta (time decay).
//!
//! Sums theta values over a rolling window to track accumulated time decay.
//! Theta is typically negative for long options, so the cumulative sum will be
//! negative when holding long options over time.
//!
//! Output: `Single(cumulative_theta)`.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::option_greeks_consumer::OptionGreeksConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OptionGreeks;

/// Tracks cumulative theta decay over a rolling window.
///
/// `cumulative_theta = sum(theta[i] for i in window)`
#[derive(Clone, Debug)]
pub struct ThetaDecayTracker {
    window: usize,
    history: VecDeque<f64>,
    last_cumulative: f64,
}

impl ThetaDecayTracker {
    /// Create a new indicator with given rolling window (min 1).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            history: VecDeque::new(),
            last_cumulative: 0.0,
        }
    }
}

impl Default for ThetaDecayTracker {
    fn default() -> Self {
        Self::new(20)
    }
}

impl OptionGreeksConsumer for ThetaDecayTracker {
    fn update_option_greeks(&mut self, g: &OptionGreeks) {
        self.history.push_back(g.theta);
        if self.history.len() > self.window {
            self.history.pop_front();
        }
        self.last_cumulative = self.history.iter().sum();
    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_cumulative = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= self.window
    }
}

/// Config for [`ThetaDecayTracker`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ThetaDecayTrackerConfig {
    pub window: Param<usize>,
}

impl Indicator for ThetaDecayTracker {
    const ID: IndicatorId = IndicatorId::ThetaDecayTracker;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::OptionGreeks];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::ThetaDecayTracker)];
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = ThetaDecayTrackerConfig;
    type Runtime = ThetaDecayTracker;

    fn create(cfg: ThetaDecayTrackerConfig) -> ThetaDecayTracker {
        ThetaDecayTracker::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for ThetaDecayTrackerConfig {
    fn defaults() -> Self {
        ThetaDecayTrackerConfig { window: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // window: A period — auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for ThetaDecayTracker {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::ThetaDecayTracker, "Theta Decay", Color::hex(0xEF9A9A))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_greeks(theta: f64) -> OptionGreeks {
        OptionGreeks {
            delta: 0.0,
            gamma: 0.0,
            vega: 0.0,
            theta,
            rho: 0.0,
            mark_iv: 0.0,
            bid_iv: None,
            ask_iv: None,
            timestamp: 0,
        }
    }

    #[test]
    fn cumulative_sums_correctly() {
        let mut ind = ThetaDecayTracker::new(5);
        for _ in 0..5 {
            ind.update_option_greeks(&make_greeks(-0.5));
        }
        let v = ind.value();
        assert!((v - (-2.5)).abs() < 1e-10, "expected -2.5, got {v}");
    }

    #[test]
    fn rolling_window_drops_oldest() {
        let mut ind = ThetaDecayTracker::new(3);
        // Push 4 values: -1, -2, -3, -4
        // After 4 pushes window holds [-2, -3, -4] → sum = -9
        for i in 1..=4 {
            ind.update_option_greeks(&make_greeks(-(i as f64)));
        }
        let v = ind.value();
        assert!((v - (-9.0)).abs() < 1e-10, "expected -9.0, got {v}");
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut ind = ThetaDecayTracker::new(5);
        for i in 0..4 {
            ind.update_option_greeks(&make_greeks(-(i as f64) * 0.1));
        }
        assert!(!ind.is_ready());
        ind.update_option_greeks(&make_greeks(-0.5));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = ThetaDecayTracker::new(3);
        for _ in 0..3 {
            ind.update_option_greeks(&make_greeks(-0.5));
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_theta_decay_tracker() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::ThetaDecayTracker(ThetaDecayTrackerConfig { window: Param::Solo(5) })
            .build_solo()
            .unwrap();
        // Feed 5 × theta=-0.5 → cumulative = -2.5. gamma=9999.0 (unused) proves only theta matters.
        for _ in 0..5 {
            let g = OptionGreeks { delta: 0.0, gamma: 9999.0, vega: 0.0, theta: -0.5, rho: 0.0,
                mark_iv: 0.0, bid_iv: None, ask_iv: None, timestamp: 0 };
            f.feed(0, MarketSample::OptionGreeks(&g));
        }
        assert!((f.primary() - (-2.5)).abs() < 1e-10, "cumulative theta should be -2.5, got {}", f.primary());
    }
}

impl ThetaDecayTracker {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_cumulative
    }
}
