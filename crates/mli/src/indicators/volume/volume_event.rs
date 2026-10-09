//! VolumeEventDetector primitive — detects volume spikes relative to a rolling average.
//!
//! Maps to `OperatorClass::VolumeEvent`.

use std::collections::VecDeque;

use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{SignalKind, VolumeSub};

/// Detects volume spikes as multiples of a rolling mean.
#[derive(Debug, Clone)]
pub struct VolumeEventDetector {
    /// Spike fires when `volume > multiplier * rolling_mean`.
    multiplier: f64,
    /// Rolling window of past volume values.
    history: VecDeque<f64>,
    /// Window period for the rolling mean.
    period: usize,
    last_signal: i8,
}

impl VolumeEventDetector {
    /// `period`: number of bars for rolling average.
    /// `multiplier`: volume must be `multiplier × average` to fire (e.g. `2.0`).
    pub fn new(period: usize, multiplier: f64) -> Self {
        let period = period.max(1);
        Self {
            multiplier,
            history: VecDeque::with_capacity(period),
            period,
            last_signal: 0,
        }
    }

    /// Feed the resolved input lanes — `[volume]` (the factory resolves the fixed Volume
    /// lane). Returns the spike signal (+1 spike / 0 none).
    pub fn feed(&mut self, lanes: &[f64]) {
        self.last_signal = match self.detect_from_values(lanes[0]) {
            Some((_, Direction::Up)) => 1,
            Some((_, Direction::Down)) => -1,
            _ => 0,
        };
    }

    /// The last spike signal.
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }

    /// Ready once the rolling window is full.
    pub fn is_ready(&self) -> bool {
        self.history.len() >= self.period
    }

    /// Detect volume event from a pre-computed volume value (slice-based hot loop).
    ///
    /// Returns `Some((SignalKind::Volume(VolumeSub::Spike), Direction::Up))`
    /// when `volume > multiplier × rolling_mean(period)`.
    /// Returns `None` during warmup (fewer than `period` bars seen).
    pub fn detect_from_values(&mut self, volume: f64) -> Option<(SignalKind, Direction)> {
        if self.history.len() >= self.period {
            self.history.pop_front();
        }
        self.history.push_back(volume);

        if self.history.len() < self.period {
            return None;
        }

        let mean: f64 = self.history.iter().sum::<f64>() / self.history.len() as f64;
        if mean <= 0.0 {
            return None;
        }

        if volume > self.multiplier * mean {
            Some((SignalKind::Volume(VolumeSub::Spike), Direction::Up))
        } else {
            None
        }
    }

    /// Reset history.
    pub fn reset(&mut self) {
        self.history.clear();
        self.last_signal = 0;
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, SourceAxis, Store, StoreKind,
    UpdateComplexity, sweep_f64,
};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`VolumeEventDetector`] — rolling period + spike multiplier.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VolumeEventConfig {
    pub period: Param<usize>,
    pub multiplier: Param<f64>,
}

impl Indicator for VolumeEventDetector {
    const ID: IndicatorId = IndicatorId::VolEvent;
    /// No family — an atomic volume-spike DETECTOR, consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed to the Volume lane — a volume-only detector.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::Volume]));
    const NEEDS_VOLUME: bool = true;
    /// O(window) per bar (rolling-mean rescan); one bounded ring of past volumes.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);
    const OUTPUTS: &'static [Output] = &[Output::ordinal(IndicatorOutputId::VolEvent)];
    type Config = VolumeEventConfig;
    type Runtime = VolumeEventDetector;

    fn create(cfg: VolumeEventConfig) -> VolumeEventDetector {
        VolumeEventDetector::new(cfg.period.resolved().max(1), cfg.multiplier.resolved())
    }
}

impl crate::contract::Config for VolumeEventConfig {
    fn defaults() -> Self {
        VolumeEventConfig { period: Param::Solo(20), multiplier: Param::Solo(2.0) }
    }
    fn machine_defaults() -> Self {
        // period: Class A period/window — auto gives range(2,4048,1)
        // multiplier (f64): Class C multiplier — sweep 0.1..=10.0 step 0.1
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


impl Render for VolumeEventDetector {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::VolEvent, "Volume Event", Color::hex(0x009688))
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
    fn factory_feeds_resolved_volume() {
        let mut f = IndicatorOrder::VolEvent(<<VolumeEventDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let bar = |v: f64| MarketSample::Bar { open: 1.0, high: 1.0, low: 1.0, close: 1.0, volume: v };
        // Warm the 20-bar mean with flat volume, then a clear 5x spike fires.
        for _ in 0..20 {
            f.feed(0, bar(100.0));
        }
        f.feed(0, bar(500.0));
        assert_eq!(f.read(IndicatorOutputId::VolEvent), 1.0, "5x volume spike must fire +1");
    }
}
