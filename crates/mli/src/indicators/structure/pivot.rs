//! Pivot primitive — N-bar high/low pivot detector.
//!
//! Confirms a pivot-high at position `n` if its value is the highest
//! in a `[left + 1 + right]` window centered on that position.
//! Similarly for pivot-low (minimum).
//!
//! Maps to `OperatorClass::Pivot`.

use std::collections::VecDeque;

use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{SignalKind, StructureSub};

/// N-bar pivot detector (left + right confirmation bars).
#[derive(Debug, Clone)]
pub struct Pivot {
    left: usize,
    right: usize,
    buffer: VecDeque<f64>,
    last_signal: i8,
}

impl Pivot {
    /// `left`: bars before the candidate pivot.
    /// `right`: bars after (confirmation lag).
    pub fn new(left: usize, right: usize) -> Self {
        let left = left.max(1);
        let right = right.max(1);
        Self {
            left,
            right,
            buffer: VecDeque::with_capacity(left + right + 1),
            last_signal: 0,
        }
    }

    /// Feed ONE resolved scalar (the configured source, default close). The signal lags
    /// by `right` bars: +1 pivot-high / -1 pivot-low / 0 none, on the confirming bar.
    pub fn feed(&mut self, value: f64) {
        self.last_signal = match self.detect_from_values(value) {
            Some((_, Direction::Up)) => 1,
            Some((_, Direction::Down)) => -1,
            _ => 0,
        };
    }

    /// The last pivot signal.
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }

    /// Ready once the confirmation window is full.
    pub fn is_ready(&self) -> bool {
        self.buffer.len() >= self.left + self.right + 1
    }

    /// Feed one value and return a signal if a pivot is confirmed.
    ///
    /// Returns `Some((SignalKind::Structure(StructureSub::OrderBlock), Direction::Up))`
    /// for pivot-high, `Direction::Down` for pivot-low.
    ///
    /// Note: result lags by `right` bars — the signal fires on the bar that
    /// provides the right-side confirmation, not on the pivot bar itself.
    pub fn detect_from_values(&mut self, value: f64) -> Option<(SignalKind, Direction)> {
        self.buffer.push_back(value);
        let needed = self.left + self.right + 1;
        if self.buffer.len() < needed {
            return None;
        }
        // Keep buffer at exactly `needed` length.
        if self.buffer.len() > needed {
            self.buffer.pop_front();
        }

        let pivot_idx = self.left; // 0-indexed center of window
        let pivot_val = self.buffer[pivot_idx];

        let is_high = (0..needed).all(|i| i == pivot_idx || self.buffer[i] <= pivot_val);
        let is_low = (0..needed).all(|i| i == pivot_idx || self.buffer[i] >= pivot_val);

        if is_high && !is_low {
            Some((SignalKind::Structure(StructureSub::OrderBlock), Direction::Up))
        } else if is_low && !is_high {
            Some((SignalKind::Structure(StructureSub::OrderBlock), Direction::Down))
        } else {
            None
        }
    }

    /// Reset buffer.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.last_signal = 0;
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, Store, StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Pivot`] — left/right confirmation-window depths.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PivotConfig {
    pub left: Param<usize>,
    pub right: Param<usize>,
}

impl Indicator for Pivot {
    const ID: IndicatorId = IndicatorId::NbarPivot;
    /// No family — an atomic pivot DETECTOR, consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// O(window) per bar (rescans the left+right+1 window); one bounded ring.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::NbarPivot)];
    type Config = PivotConfig;
    type Runtime = Pivot;

    fn create(cfg: PivotConfig) -> Pivot {
        Pivot::new(cfg.left.resolved(), cfg.right.resolved())
    }
}

impl crate::contract::Config for PivotConfig {
    fn defaults() -> Self {
        PivotConfig {
            left: Param::Solo(5),
            right: Param::Solo(5),
        }
    }
    fn machine_defaults() -> Self {
        // left, right: A period (confirmation window depth) — auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Pivot {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::NbarPivot, "Pivot", Color::hex(0x3F51B5))
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
    fn factory_feeds_resolved_pivot() {
        let mut f = IndicatorOrder::NbarPivot(<<Pivot as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        let bar = |c: f64| MarketSample::Bar { open: c, high: c, low: c, close: c, volume: 1.0 };
        // left=right=5: a clean peak at the center of an 11-bar window must confirm a
        // pivot-high on the last bar.
        let prices = [1.0, 2.0, 3.0, 4.0, 5.0, 9.0, 5.0, 4.0, 3.0, 2.0, 1.0];
        let mut saw_high = false;
        for p in prices {
            f.feed(0, bar(p));
            if f.primary() > 0.0 {
                saw_high = true;
            }
        }
        assert!(saw_high, "a clear center peak must confirm a pivot-high");
    }
}
