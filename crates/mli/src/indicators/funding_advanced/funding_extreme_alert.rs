//! FundingExtremeAlert — flags extreme funding rate deviations.
//!
//! Computes rolling mean + std over `window` snapshots.
//! When `|current - mean| > sigma_threshold × std`:
//! - `signal = +1` if current > mean, `-1` if current < mean
//! - `magnitude = |current - mean| / std` (in sigmas)
//!
//! Output: `Double(signal_as_f64, magnitude_sigma)`.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::axis::sweep_f64;
use crate::contract::{Color, Render, RenderSpec};
use crate::core::types::FundingRate;
use crate::engine::stream_kind::StreamKind;

/// Detects statistically extreme funding rate events.
///
/// Fires a signal when `|rate - rolling_mean| > sigma_threshold × rolling_std`.
///
/// Output: `Double(signal_as_f64, magnitude_sigma)`.
/// `signal_as_f64 = +1.0 / -1.0 / 0.0`, `magnitude_sigma = deviation in sigmas`.
#[derive(Debug, Clone)]
pub struct FundingExtremeAlert {
    window: usize,
    sigma_threshold: f64,
    history: VecDeque<f64>,
    last_signal: f64,
    last_magnitude: f64,
}

impl FundingExtremeAlert {
    /// Create a new indicator.
    ///
    /// - `window`: rolling lookback length (min 2).
    /// - `sigma_threshold`: deviation threshold in standard deviations (default 2.0).
    pub fn new(window: usize, sigma_threshold: f64) -> Self {
        Self {
            window: window.max(2),
            sigma_threshold,
            history: VecDeque::new(),
            last_signal: 0.0,
            last_magnitude: 0.0,
        }
    }

    /// Extreme signal: `+1.0`, `-1.0`, or `0.0`.
    pub fn signal(&self) -> f64 {
        self.last_signal
    }

    /// Deviation magnitude in standard deviations.
    pub fn magnitude(&self) -> f64 {
        self.last_magnitude
    }

    fn compute(&self, current: f64) -> (f64, f64) {
        let n = self.history.len();
        if n < 2 {
            return (0.0, 0.0);
        }
        let mean = self.history.iter().sum::<f64>() / n as f64;
        let variance = self.history.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        let std = variance.sqrt();
        if std < 1e-15 {
            return (0.0, 0.0);
        }
        let deviation = current - mean;
        let magnitude = deviation.abs() / std;
        if magnitude > self.sigma_threshold {
            let signal = if deviation > 0.0 { 1.0 } else { -1.0 };
            (signal, magnitude)
        } else {
            (0.0, magnitude)
        }
    }
}

impl Default for FundingExtremeAlert {
    fn default() -> Self {
        Self::new(20, 2.0)
    }
}

/// Typed configuration for [`FundingExtremeAlert`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FundingExtremeAlertConfig {
    pub window: Param<usize>,
    pub sigma_threshold: Param<f64>,
}

impl Indicator for FundingExtremeAlert {
    const ID: IndicatorId = IndicatorId::FundingExtremeAlert;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Funding];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::FundingExtremeAlertSignal),
        Output::magnitude(IndicatorOutputId::FundingExtremeAlertMagnitude),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);
    type Config = FundingExtremeAlertConfig;
    type Runtime = Self;

    fn create(cfg: FundingExtremeAlertConfig) -> Self {
        Self::new(cfg.window.resolved(), cfg.sigma_threshold.resolved())
    }
}

impl crate::contract::Config for FundingExtremeAlertConfig {
    fn defaults() -> Self {
        FundingExtremeAlertConfig { window: Param::Solo(20), sigma_threshold: Param::Solo(2.0) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // window: Class A → auto range(2,4048,1); already set by auto.
        // sigma_threshold: Class F (sigma/z-score threshold) → sweep_f64(0.1,5.0,0.1).
        s.sigma_threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s
    }
}


impl Render for FundingExtremeAlert {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::FundingExtremeAlertSignal, "Extreme Signal", Color::hex(0xF44336))
            .line_output(IndicatorOutputId::FundingExtremeAlertMagnitude, "Magnitude (σ)", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

impl FundingRateConsumer for FundingExtremeAlert {
    fn update_funding(&mut self, fr: &FundingRate) {
        self.history.push_back(fr.rate);
        if self.history.len() > self.window {
            self.history.pop_front();
        }
        let (signal, magnitude) = self.compute(fr.rate);
        self.last_signal = signal;
        self.last_magnitude = magnitude;
    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_signal = 0.0;
        self.last_magnitude = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_funding_extreme_alert() {
        let mut f = IndicatorOrder::FundingExtremeAlert(
            <<FundingExtremeAlert as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let fr = FundingRate { rate: 0.0001, next_funding_time: None, timestamp: 1000, ..Default::default()};
        f.feed(0, MarketSample::Funding(&fr));
        let _ = f.primary();
    }

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate {
            rate,
            next_funding_time: None,
            timestamp: 1000,
            ..Default::default()
        }
    }

    #[test]
    fn no_extreme_for_normal_rate() {
        let mut ind = FundingExtremeAlert::new(10, 2.0);
        for _ in 0..10 {
            ind.update_funding(&make_fr(0.0001));
        }
        // Constant series — std≈0, no extreme
        assert_eq!(ind.signal(), 0.0);
    }

    #[test]
    fn extreme_positive_fires_plus_one() {
        let mut ind = FundingExtremeAlert::new(10, 2.0);
        // Fill with small rates
        for _ in 0..9 {
            ind.update_funding(&make_fr(0.0001));
        }
        // Push a very large rate to trigger alert
        ind.update_funding(&make_fr(0.1));
        assert_eq!(ind.signal(), 1.0, "extreme positive should give +1");
        assert!(ind.magnitude() > 2.0, "magnitude should exceed threshold");
    }

    #[test]
    fn extreme_negative_fires_minus_one() {
        let mut ind = FundingExtremeAlert::new(10, 2.0);
        for _ in 0..9 {
            ind.update_funding(&make_fr(0.0001));
        }
        ind.update_funding(&make_fr(-0.1));
        assert_eq!(ind.signal(), -1.0, "extreme negative should give -1");
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut ind = FundingExtremeAlert::new(5, 2.0);
        for i in 0..4 {
            ind.update_funding(&make_fr(i as f64 * 0.0001));
        }
        assert!(!ind.is_ready());
        ind.update_funding(&make_fr(0.0005));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = FundingExtremeAlert::new(5, 2.0);
        for _ in 0..5 {
            ind.update_funding(&make_fr(0.0001));
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.signal(), 0.0);
        assert_eq!(ind.magnitude(), 0.0);
    }
}
