//! OiZScore — rolling z-score of open interest.
//!
//! Measures how many standard deviations the current OI is from its rolling mean.
//! z = (current_oi - mean) / std
//!
//! Output: `Single(z_score)`. Zero when std = 0 or fewer than 2 samples.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::open_interest_consumer::OpenInterestConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Family, Indicator, Output, Param, SourceAxis};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OpenInterest;

/// Rolling z-score of open interest over a configurable window.
#[derive(Clone, Debug)]
pub struct OiZScore {
    window: usize,
    history: VecDeque<f64>,
    last_z: f64,
}

impl OiZScore {
    /// Create with given rolling window size (minimum 2).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            history: VecDeque::with_capacity(window.max(2)),
            last_z: 0.0,
        }
    }

    /// Compute z-score of `current` relative to the window (which already includes `current`).
    /// Requires at least 2 values in the window.
    fn compute_z_in_window(window: &VecDeque<f64>) -> f64 {
        let n = window.len();
        if n < 2 {
            return 0.0;
        }
        let nf = n as f64;
        let current = *window.back().expect("window non-empty");
        let mean = window.iter().sum::<f64>() / nf;
        // Population std over the window (consistent with history-based normalisation)
        let variance = window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / nf;
        let std = variance.sqrt();
        if std == 0.0 {
            0.0
        } else {
            (current - mean) / std
        }
    }
}

impl Default for OiZScore {
    fn default() -> Self {
        Self::new(50)
    }
}

/// Typed configuration for [`OiZScore`]. Input is the dig3-core `OpenInterest`
/// (consumed via `update_oi`), not a bar.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct OiZScoreConfig {
    pub window: Param<usize>,
}

impl Indicator for OiZScore {
    const ID: IndicatorId = IndicatorId::OiZScore;
    // Open-interest indicator (consumes `OpenInterest` via update_oi).
    const FAMILY: &'static [Family] = &[Family::OpenInterest];
    const INPUT: &'static [StreamKind] = &[StreamKind::OpenInterest];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::OiZScore)];
    type Config = OiZScoreConfig;
    type Runtime = OiZScore;

    fn create(cfg: OiZScoreConfig) -> OiZScore {
        OiZScore::new(cfg.window.resolved())
    }
}

impl OpenInterestConsumer for OiZScore {
    fn update_oi(&mut self, oi: &OpenInterest) {
        let current = oi.open_interest;
        // Push first, then compute z within the window (includes current bar)
        if self.history.len() == self.window {
            self.history.pop_front();
        }
        self.history.push_back(current);
        self.last_z = Self::compute_z_in_window(&self.history);
    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_z = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= 2
    }
}

impl crate::contract::Config for OiZScoreConfig {
    fn defaults() -> Self {
        OiZScoreConfig { window: Param::Solo(50) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // window: Class A (integer lookback/window) — auto already sets range(2, 4048, 1).
        Self::machine_defaults_auto()
    }
}


impl Render for OiZScore {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::OiZScore, "OI Z-Score", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_oi(oi: f64) -> OpenInterest {
        OpenInterest {
            open_interest: oi,
            open_interest_value: None,
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn not_ready_initially() {
        let ind = OiZScore::new(5);
        assert!(!ind.is_ready());
    }

    #[test]
    fn z_positive_on_spike() {
        let mut ind = OiZScore::new(50);
        // Push identical values: mean=100, std=0 at first then with variance
        for _ in 0..10 {
            ind.update_oi(&make_oi(100.0));
        }
        // Now push a spike: z should be positive
        ind.update_oi(&make_oi(150.0));
        let z = ind.value();
        assert!(z > 0.0, "z should be positive on spike above mean, got {z}");
    }

    #[test]
    fn z_negative_on_dip() {
        let mut ind = OiZScore::new(50);
        for _ in 0..10 {
            ind.update_oi(&make_oi(100.0));
        }
        ind.update_oi(&make_oi(50.0));
        let z = ind.value();
        assert!(z < 0.0, "z should be negative on dip below mean, got {z}");
    }

    #[test]
    fn z_zero_on_constant_series() {
        let mut ind = OiZScore::new(5);
        for _ in 0..4 {
            ind.update_oi(&make_oi(100.0));
        }
        // After 4 identical pushes, history is [100,100,100,100], next compute std=0
        ind.update_oi(&make_oi(100.0));
        let z = ind.value();
        assert_eq!(z, 0.0, "z should be 0 when std=0");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = OiZScore::new(5);
        ind.update_oi(&make_oi(100.0));
        ind.update_oi(&make_oi(150.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn deterministic_sequence() {
        // [100,100,100,100,150]: z for last > 0
        let mut ind = OiZScore::new(50);
        let values = [100.0f64, 100.0, 100.0, 100.0];
        for &v in &values {
            ind.update_oi(&make_oi(v));
        }
        ind.update_oi(&make_oi(150.0));
        let z = ind.value();
        assert!(z > 0.0, "z should be > 0 for value above mean, got {z}");
    }
}

impl OiZScore {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_z
    }
}
