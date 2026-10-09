//! PinRiskDetector — detects pin risk near a strike at expiration.
//!
//! A position is at pin risk when:
//! - `|delta| - delta_target| <= delta_tolerance` (delta near ±0.5, i.e. near strike)
//! - `|theta| >= theta_threshold` (significant time decay, near expiration)
//!
//! Output: `Signal(i8)`. `+1` = pin risk high, `0` = no pin risk.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::option_greeks_consumer::OptionGreeksConsumer;
use crate::contract::{Family, Indicator, Output, Param, SourceAxis, sweep_f64};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OptionGreeks;

/// Detects options pin risk: position near strike price close to expiration.
///
/// Pin risk occurs when delta is close to ±0.5 (near-the-money) AND theta is
/// large in absolute value (close to expiry).
#[derive(Clone, Debug)]
pub struct PinRiskDetector {
    delta_target: f64,
    delta_tolerance: f64,
    theta_threshold: f64,
    last_signal: i8,
}

impl PinRiskDetector {
    /// Create a new indicator.
    ///
    /// - `delta_target`: target |delta| for pin risk (default 0.5).
    /// - `delta_tolerance`: allowed deviation from target (default 0.05).
    /// - `theta_threshold`: minimum `|theta|` to confirm near-expiry (default 0.5).
    pub fn new(delta_target: f64, delta_tolerance: f64, theta_threshold: f64) -> Self {
        Self {
            delta_target,
            delta_tolerance,
            theta_threshold,
            last_signal: 0,
        }
    }
}

impl Default for PinRiskDetector {
    fn default() -> Self {
        Self::new(0.5, 0.05, 0.5)
    }
}

impl OptionGreeksConsumer for PinRiskDetector {
    fn update_option_greeks(&mut self, g: &OptionGreeks) {
        let delta_close = (g.delta.abs() - self.delta_target).abs() <= self.delta_tolerance;
        let theta_big = g.theta.abs() >= self.theta_threshold;
        self.last_signal = if delta_close && theta_big { 1 } else { 0 };
    }


    fn reset(&mut self) {
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

/// Config for [`PinRiskDetector`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PinRiskDetectorConfig {
    pub delta_target: Param<f64>,
    pub delta_tolerance: Param<f64>,
    pub theta_threshold: Param<f64>,
}

impl Indicator for PinRiskDetector {
    const ID: IndicatorId = IndicatorId::PinRiskDetector;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::OptionGreeks];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::PinRiskDetector)];
    type Config = PinRiskDetectorConfig;
    type Runtime = PinRiskDetector;

    fn create(cfg: PinRiskDetectorConfig) -> PinRiskDetector {
        PinRiskDetector::new(cfg.delta_target.resolved(), cfg.delta_tolerance.resolved(), cfg.theta_threshold.resolved())
    }
}

impl crate::contract::Config for PinRiskDetectorConfig {
    fn defaults() -> Self {
        PinRiskDetectorConfig {
            delta_target: Param::Solo(0.5),
            delta_tolerance: Param::Solo(0.05),
            theta_threshold: Param::Solo(0.5),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // delta_target: D — ratio/fraction in [0,1] (how close to ATM strike)
        s.delta_target = Param::many(sweep_f64(0.0, 1.0, 0.05));
        // delta_tolerance: D — allowed deviation from target, also [0,1]
        s.delta_tolerance = Param::many(sweep_f64(0.0, 1.0, 0.05));
        // theta_threshold: F — threshold on |theta|, sigma-class
        s.theta_threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for PinRiskDetector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::PinRiskDetector, "Pin Risk", Color::hex(0xEF5350))
            .zero_baseline()
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_greeks(delta: f64, theta: f64) -> OptionGreeks {
        OptionGreeks {
            delta,
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
    fn fires_when_near_strike_and_high_theta() {
        let mut ind = PinRiskDetector::default();
        // delta ≈ 0.5, |theta| > 0.5
        ind.update_option_greeks(&make_greeks(0.48, -0.8));
        assert_eq!(ind.value() as i8, 1, "should detect pin risk");
    }

    #[test]
    fn no_signal_when_delta_far_from_target() {
        let mut ind = PinRiskDetector::default();
        // delta = 0.1 — far from 0.5
        ind.update_option_greeks(&make_greeks(0.1, -1.0));
        assert_eq!(ind.value() as i8, 0, "delta far from 0.5 should not trigger");
    }

    #[test]
    fn no_signal_when_theta_low() {
        let mut ind = PinRiskDetector::default();
        // delta close to 0.5 but theta near zero
        ind.update_option_greeks(&make_greeks(0.5, -0.1));
        assert_eq!(ind.value() as i8, 0, "low theta should not trigger");
    }

    #[test]
    fn works_with_negative_delta() {
        let mut ind = PinRiskDetector::default();
        // delta = -0.5 (put near-the-money)
        ind.update_option_greeks(&make_greeks(-0.5, -0.9));
        assert_eq!(ind.value() as i8, 1, "negative delta near -0.5 should also trigger");
    }

    #[test]
    fn reset_clears_signal() {
        let mut ind = PinRiskDetector::default();
        ind.update_option_greeks(&make_greeks(0.5, -1.0));
        ind.reset();
        assert_eq!(ind.value() as i8, 0);
    }

    #[test]
    fn factory_feeds_resolved_pin_risk_detector() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let cfg = super::PinRiskDetectorConfig { delta_target: Param::Solo(0.5), delta_tolerance: Param::Solo(0.05), theta_threshold: Param::Solo(0.5) };
        let mut f = IndicatorOrder::PinRiskDetector(cfg)
            .build_solo()
            .unwrap();
        // delta=0.48 (near 0.5), theta=-0.8 (|theta| > 0.5) → pin risk fires
        // gamma=9999.0 (unused) to prove only delta/theta matter
        let g = OptionGreeks { delta: 0.48, gamma: 9999.0, vega: 0.0, theta: -0.8, rho: 0.0,
            mark_iv: 0.0, bid_iv: None, ask_iv: None, timestamp: 0 };
        f.feed(0, MarketSample::OptionGreeks(&g));
        assert_eq!(f.primary(), 1.0, "pin risk should fire, got {}", f.primary());
    }
}

impl PinRiskDetector {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }
}
