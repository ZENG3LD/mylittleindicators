//! DirectionDetector primitive — fires on each bar that the value changes direction
//! relative to the previous bar.
//!
//! Maps to `OperatorClass::Direction`.

use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{SignalKind, TrendSub};

/// Detects up/down direction changes of a scalar value.
#[derive(Debug, Clone)]
pub struct DirectionDetector {
    prev: Option<f64>,
    last_signal: i8,
}

impl DirectionDetector {
    pub fn new() -> Self {
        Self { prev: None, last_signal: 0 }
    }

    /// Feed ONE resolved scalar (the configured source, default close). The factory
    /// extracts the source field and feeds it; returns the current direction signal
    /// (+1 up / -1 down / 0 unchanged).
    pub fn feed(&mut self, value: f64) {
        self.last_signal = match self.detect_from_values(value) {
            Some((_, Direction::Up)) => 1,
            Some((_, Direction::Down)) => -1,
            _ => 0,
        };
    }

    /// The last direction signal.
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }

    /// Ready once it has a previous value to compare against.
    pub fn is_ready(&self) -> bool {
        self.prev.is_some()
    }

    /// Detect direction from a pre-computed value (slice-based hot loop).
    ///
    /// Returns `Some((SignalKind::Trend(TrendSub::PriceCross), Direction::Up))`
    /// when `value > prev`, `Direction::Down` when `value < prev`, `None` when equal
    /// or on the first call.
    pub fn detect_from_values(&mut self, value: f64) -> Option<(SignalKind, Direction)> {
        let result = match self.prev {
            None => None,
            Some(p) if value > p => {
                Some((SignalKind::Trend(TrendSub::PriceCross), Direction::Up))
            }
            Some(p) if value < p => {
                Some((SignalKind::Trend(TrendSub::PriceCross), Direction::Down))
            }
            _ => None,
        };
        self.prev = Some(value);
        result
    }

    /// Reset state.
    pub fn reset(&mut self) {
        self.prev = None;
        self.last_signal = 0;
    }
}

impl Default for DirectionDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Color, Cost, Family, Indicator, Output, Render, RenderSpec, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`DirectionDetector`] — no parameters (pure direction-change edge).
#[derive(Debug, Clone, Copy, PartialEq, Eq, mli_contract_macros::ConfigAxes)]
pub struct DirectionDetectorConfig;

impl Indicator for DirectionDetector {
    const ID: IndicatorId = IndicatorId::DirDetect;
    /// No family — an atomic direction-change DETECTOR, consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// O(1) per bar; one scalar of state (the previous value).
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::ordinal(IndicatorOutputId::DirDetect)];
    type Config = DirectionDetectorConfig;
    type Runtime = DirectionDetector;

    fn create(_cfg: DirectionDetectorConfig) -> DirectionDetector {
        DirectionDetector::new()
    }
}

impl crate::contract::Config for DirectionDetectorConfig {
    fn defaults() -> Self {
        DirectionDetectorConfig
    }
    fn machine_defaults() -> Self {
        // No Param fields — the unit struct is its own machine default.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for DirectionDetector {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::DirDetect, "Direction", Color::hex(0x607D8B))
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
    fn factory_feeds_resolved_direction() {
        let mut f = IndicatorOrder::DirDetect(<<DirectionDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let bar = |c: f64| MarketSample::Bar { open: c, high: c, low: c, close: c, volume: 1.0 };
        f.feed(0, bar(100.0)); // first: no prev
        f.feed(0, bar(101.0)); // up
        assert_eq!(f.primary(), 1.0, "rising close -> +1");
        f.feed(0, bar(99.0)); // down
        assert_eq!(f.primary(), -1.0, "falling close -> -1");
    }
}
