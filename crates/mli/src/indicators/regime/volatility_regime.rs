//! VolatilityRegimeDetector primitive — classifies volatility into Low/Normal/High
//! regimes and fires on regime transitions.
//!
//! Accepts ATR, StdDev, or any scalar volatility measure.
//! Maps to `OperatorClass::VolatilityRegime`.

use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{SignalKind, VolatilitySub};

/// Volatility regime level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolatilityLevel {
    /// Below `low_threshold`.
    Low,
    /// Between `low_threshold` and `high_threshold`.
    Normal,
    /// Above `high_threshold`.
    High,
}

/// Detects transitions between volatility regimes.
#[derive(Debug, Clone)]
pub struct VolatilityRegimeDetector {
    low_threshold: f64,
    high_threshold: f64,
    current: Option<VolatilityLevel>,
    last_signal: i8,
}

impl VolatilityRegimeDetector {
    /// `low_threshold`: boundary between Low and Normal.
    /// `high_threshold`: boundary between Normal and High.
    pub fn new(low_threshold: f64, high_threshold: f64) -> Self {
        Self {
            low_threshold,
            high_threshold,
            current: None,
            last_signal: 0,
        }
    }

    /// Feed ONE resolved scalar (the configured source, default close — typically wired to
    /// an ATR/StdDev). Returns the volatility-regime transition signal (+1 / -1 / 0).
    pub fn feed(&mut self, value: f64) {
        self.last_signal = match self.detect_from_values(value) {
            Some((_, Direction::Up)) => 1,
            Some((_, Direction::Down)) => -1,
            _ => 0,
        };
    }

    /// The last regime transition signal.
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }

    /// Ready once it has classified its first value.
    pub fn is_ready(&self) -> bool {
        self.current.is_some()
    }

    fn classify(&self, value: f64) -> VolatilityLevel {
        if value < self.low_threshold {
            VolatilityLevel::Low
        } else if value > self.high_threshold {
            VolatilityLevel::High
        } else {
            VolatilityLevel::Normal
        }
    }

    /// Detect volatility regime from a pre-computed ATR/StdDev value (slice-based hot loop).
    ///
    /// Fires only on regime transitions:
    /// - Transition into High → `Volatility(Extreme)`, `Direction::Up`
    /// - Transition out of High → `Volatility(Shift)`, `Direction::Down`
    /// - Transition into Low → `Volatility(Squeeze)`, `Direction::Down`
    /// - Transition out of Low → `Volatility(Shift)`, `Direction::Up`
    pub fn detect_from_values(&mut self, atr_or_stddev: f64) -> Option<(SignalKind, Direction)> {
        let new_level = self.classify(atr_or_stddev);
        let prev = self.current;
        self.current = Some(new_level);

        let prev_level = match prev {
            None => return None,
            Some(l) => l,
        };

        if prev_level == new_level {
            return None;
        }

        let result = match (prev_level, new_level) {
            (_, VolatilityLevel::High) => {
                Some((SignalKind::Volatility(VolatilitySub::Extreme), Direction::Up))
            }
            (VolatilityLevel::High, _) => {
                Some((SignalKind::Volatility(VolatilitySub::Shift), Direction::Down))
            }
            (_, VolatilityLevel::Low) => {
                Some((SignalKind::Volatility(VolatilitySub::Squeeze), Direction::Down))
            }
            (VolatilityLevel::Low, _) => {
                Some((SignalKind::Volatility(VolatilitySub::Shift), Direction::Up))
            }
            _ => None,
        };
        result
    }

    /// Current regime level (None until first value fed).
    pub fn current_level(&self) -> Option<VolatilityLevel> {
        self.current
    }

    /// Reset state.
    pub fn reset(&mut self) {
        self.current = None;
        self.last_signal = 0;
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, UpdateComplexity, sweep_f64};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`VolatilityRegimeDetector`] — the low/high regime boundaries. A
/// scalar transition detector over a configurable source (default close; wire to a
/// volatility series like ATR/StdDev by choosing that source).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VolatilityRegimeConfig {
    pub low_threshold: Param<f64>,
    pub high_threshold: Param<f64>,
}

impl Indicator for VolatilityRegimeDetector {
    const ID: IndicatorId = IndicatorId::VolRegimeDetect;
    /// No family — an atomic volatility-regime transition DETECTOR, consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// O(1) per bar; one Option of prior level.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::VolRegimeDetect)];
    type Config = VolatilityRegimeConfig;
    type Runtime = VolatilityRegimeDetector;

    fn create(cfg: VolatilityRegimeConfig) -> VolatilityRegimeDetector {
        VolatilityRegimeDetector::new(cfg.low_threshold.resolved(), cfg.high_threshold.resolved())
    }
}

impl crate::contract::Config for VolatilityRegimeConfig {
    fn defaults() -> Self {
        VolatilityRegimeConfig {
            low_threshold: Param::Solo(0.5),
            high_threshold: Param::Solo(1.5),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // low_threshold / high_threshold: Class F sigma/threshold — sweep_f64(0.1, 5.0, 0.1).
        s.low_threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s.high_threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VolatilityRegimeDetector {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(
                IndicatorOutputId::VolRegimeDetect,
                "Volatility Regime",
                Color::hex(0x9C27B0),
            )
            .bounds(-1.0, 1.0)
            .zero_baseline()
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_vol_regime() {
        let cfg = VolatilityRegimeConfig {
            low_threshold: Param::Solo(10.0),
            high_threshold: Param::Solo(30.0),
        };
        let mut f = IndicatorOrder::VolRegimeDetect(cfg).build_solo().unwrap();
        let bar = |c: f64| MarketSample::Bar { open: c, high: c, low: c, close: c, volume: 1.0 };
        f.feed(0, bar(20.0)); // init Normal
        f.feed(0, bar(40.0));
        assert_eq!(f.primary(), 1.0, "transition into High = +1");
    }
}
