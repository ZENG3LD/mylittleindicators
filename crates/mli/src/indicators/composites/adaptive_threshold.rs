//! AdaptiveThreshold — dynamic threshold based on rolling mean + N×std.
//!
//! Consumer: `TickConsumer`.
//!
//! Logic: threshold = rolling_mean(prices) + N × rolling_std(prices).
//! Uses a fixed rolling window of price observations.
//!
//! Output: `Triple(mean, std, threshold)`.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::contract::sweep_f64;
use crate::core::types::Tick;
use crate::engine::stream_kind::StreamKind;

/// Dynamic threshold meta-indicator for z-score based detectors.
///
/// Implements `TickConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Debug, Clone)]
pub struct AdaptiveThreshold {
    window: usize,
    multiplier: f64,
    prices: VecDeque<f64>,
    last_mean: f64,
    last_std: f64,
    last_threshold: f64,
}

impl AdaptiveThreshold {
    /// Create a new indicator.
    ///
    /// - `window`     — rolling window size (default 50).
    /// - `multiplier` — std multiplier N (default 2.0).
    pub fn new(window: usize, multiplier: f64) -> Self {
        Self {
            window: window.max(2),
            multiplier,
            prices: VecDeque::with_capacity(window.max(2)),
            last_mean: 0.0,
            last_std: 0.0,
            last_threshold: 0.0,
        }
    }

    fn recompute(&mut self) {
        let n = self.prices.len();
        if n == 0 {
            return;
        }
        let mean: f64 = self.prices.iter().sum::<f64>() / n as f64;
        let std = if n >= 2 {
            let var: f64 = self.prices.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / (n - 1) as f64;
            var.sqrt()
        } else {
            0.0
        };
        self.last_mean = mean;
        self.last_std = std;
        self.last_threshold = mean + self.multiplier * std;
    }

    /// Named output: brace `mean`.
    pub fn mean(&self) -> f64 { self.last_mean }
    /// Named output: brace `std`.
    pub fn std(&self) -> f64 { self.last_std }
    /// Named output: brace `threshold`.
    pub fn threshold(&self) -> f64 { self.last_threshold }


    /// True when window is filled.
    pub fn indicator_is_ready(&self) -> bool {
        self.prices.len() >= self.window
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.prices.clear();
        self.last_mean = 0.0;
        self.last_std = 0.0;
        self.last_threshold = 0.0;
    }
}

impl Default for AdaptiveThreshold {
    fn default() -> Self {
        Self::new(50, 2.0)
    }
}

impl TickConsumer for AdaptiveThreshold {
    fn update_tick(&mut self, tick: &Tick) {
        if self.prices.len() >= self.window {
            self.prices.pop_front();
        }
        self.prices.push_back(tick.price);
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(price: f64) -> Tick {
        Tick::new(0, price, 1.0, true)
    }

    #[test]
    fn threshold_above_mean() {
        let mut ind = AdaptiveThreshold::new(5, 2.0);
        for p in [100.0, 102.0, 98.0, 101.0, 99.0] {
            ind.update_tick(&tick(p));
        }
        {
            let (mean, std, threshold) = (ind.mean(), ind.std(), ind.threshold());
            assert!(threshold > mean, "threshold={threshold} mean={mean}");
            assert!((threshold - mean).abs() - 2.0 * std < 1e-9);
        }
    }

    #[test]
    fn threshold_equals_mean_with_zero_std() {
        let mut ind = AdaptiveThreshold::new(3, 2.0);
        for _ in 0..3 {
            ind.update_tick(&tick(100.0)); // all same → std=0
        }
        {
            let (mean, std, threshold) = (ind.mean(), ind.std(), ind.threshold());
            assert!((mean - 100.0).abs() < 1e-9);
            assert!(std.abs() < 1e-9);
            assert!((threshold - 100.0).abs() < 1e-9);
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = AdaptiveThreshold::default();
        for p in [100.0, 105.0, 95.0] {
            ind.update_tick(&tick(p));
        }
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        assert_eq!((ind.mean(), ind.std(), ind.threshold()), (0.0, 0.0, 0.0));
    }
}

// ---- Indicator contract ----

/// Typed configuration for [`AdaptiveThreshold`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AdaptiveThresholdConfig {
    /// Rolling window size (number of ticks).
    pub window: Param<usize>,
    /// Standard-deviation multiplier N.
    pub multiplier: Param<f64>,
}

impl Indicator for AdaptiveThreshold {
    const ID: IndicatorId = IndicatorId::AdaptiveThreshold;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::AdaptiveThresholdMean),
        Output::magnitude(IndicatorOutputId::AdaptiveThresholdStd),
        Output::price(IndicatorOutputId::AdaptiveThresholdThreshold),
    ];
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = AdaptiveThresholdConfig;
    type Runtime = AdaptiveThreshold;

    fn create(cfg: AdaptiveThresholdConfig) -> AdaptiveThreshold {
        AdaptiveThreshold::new(cfg.window.resolved(), cfg.multiplier.resolved())
    }
}

impl crate::contract::Config for AdaptiveThresholdConfig {
    fn defaults() -> Self {
        AdaptiveThresholdConfig {
            window: Param::Solo(50),
            multiplier: Param::Solo(2.0),
        }
    }
    fn machine_defaults() -> Self {
        // window (usize): Class A period — auto gives range(2,4048,1) ✓
        // multiplier (f64): Class C band-width multiplier — sweep_f64(0.1,10.0,0.1)
        let mut s = Self::machine_defaults_auto();
        s.multiplier = Param::many(sweep_f64(0.1, 10.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for AdaptiveThreshold {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::AdaptiveThresholdMean, "Mean", Color::hex(0x42A5F5))
            .line_output(IndicatorOutputId::AdaptiveThresholdStd, "Std", Color::hex(0xFFA726))
            .line_output(IndicatorOutputId::AdaptiveThresholdThreshold, "Threshold", Color::hex(0xEF5350))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_adaptive_threshold() {
        let mut f = IndicatorOrder::AdaptiveThreshold(
            <AdaptiveThresholdConfig as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let t = crate::core::types::Tick::new(0, 100.0, 1.0, true);
        f.feed(0, MarketSample::Tick(&t));
        assert!(f.primary().is_finite(), "adaptive threshold factory value should be finite");
    }
}
