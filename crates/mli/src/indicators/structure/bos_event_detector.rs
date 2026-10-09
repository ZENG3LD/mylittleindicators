//! Break of Structure (BOS) event detector.
//!
//! Detects when the current bar breaks the rolling extremum over a lookback window:
//! - Up break:   `current high > max(highs[window-1 preceding bars])`
//! - Down break: `current low  < min(lows[window-1 preceding bars])`
//!
//! NOTE: CHoCH (Change of Character) is NOT implemented — this is a simple
//! rolling-extremum breakout detector, not a swing-based structure tracker.
//!
//! The algorithm matches the original `BosChochDetector` exactly:
//! - Circular buffer of `lookback` slots
//! - Fills current slot BEFORE computing prev extremes (using `idx` before increment)
//! - `filled` flag is set once the write pointer wraps around once
//! - Prev extremes = max/min over the `lookback-1` slots BEFORE the current slot
//! - Initial `highs` fill = `0.0` (matches original); detection is stable after warmup

use crate::core::signal::kind::StructureSub;
use crate::core::signal::{Direction, SignalKind};

// ---- Indicator contract imports ----
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderOutput, RenderSpec, SourceAxis, Store,
    StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Break of Structure event detector.
///
/// Uses a circular buffer of `lookback` bars. After warmup (`is_ready() == true`),
/// emits `BOS Up` when current high beats the window maximum and `BOS Down` when
/// current low undercuts the window minimum.
#[derive(Clone, Debug)]
pub struct BosEventDetector {
    lookback: usize,
    highs: Vec<f64>,
    lows: Vec<f64>,
    idx: usize,
    filled: bool,
    last_signal: i8,
}

impl BosEventDetector {
    pub fn new(lookback: usize) -> Self {
        let lookback = lookback.max(2);
        Self {
            lookback,
            highs: vec![0.0; lookback],
            lows: vec![0.0; lookback],
            idx: 0,
            filled: false,
            last_signal: 0,
        }
    }

    /// Feed a new bar and return the typed signal if a BOS pattern is detected.
    ///
    /// Mirrors the original `BosChochDetector::update_bar` logic precisely:
    /// write current bar into slot, advance pointer, then compare current vs
    /// the remaining `lookback-1` previous slots.
    pub fn detect_from_values(&mut self, high: f64, low: f64) -> Option<(SignalKind, Direction)> {
        self.highs[self.idx] = high;
        self.lows[self.idx] = low;
        self.idx = (self.idx + 1) % self.lookback;
        if self.idx == 0 {
            self.filled = true;
        }
        if !self.filled {
            self.last_signal = 0;
            return None;
        }

        // Compare current bar against the lookback-1 preceding bars.
        // Current bar lives at slot `(self.idx + lookback - 1) % lookback`.
        let len = self.lookback;
        let mut prev_max = f64::MIN;
        let mut prev_min = f64::MAX;
        for k in 1..len {
            let i = (self.idx + len - 1 - k) % len;
            if self.highs[i] > prev_max {
                prev_max = self.highs[i];
            }
            if self.lows[i] < prev_min {
                prev_min = self.lows[i];
            }
        }

        let curr_i = (self.idx + len - 1) % len;
        let h = self.highs[curr_i];
        let l = self.lows[curr_i];

        if h > prev_max {
            self.last_signal = 1;
            Some((SignalKind::Structure(StructureSub::BOS), Direction::Up))
        } else if l < prev_min {
            self.last_signal = -1;
            Some((SignalKind::Structure(StructureSub::BOS), Direction::Down))
        } else {
            self.last_signal = 0;
            None
        }
    }

    /// Feed the resolved input lanes — `[high, low]` (the factory resolves the fixed
    /// High/Low slice). Returns the current signal. Knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) {
        let h = lanes[0];
        let l = lanes[1];
        self.detect_from_values(h, l);
    }


    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }

    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn reset(&mut self) {
        self.highs.fill(0.0);
        self.lows.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.last_signal = 0;
    }
}

impl Default for BosEventDetector {
    fn default() -> Self {
        Self::new(20)
    }
}

// ---- Indicator contract ----

/// Typed contract config for [`BosEventDetector`] — a single lookback window depth.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct BosConfig {
    pub lookback: Param<usize>,
}

impl Indicator for BosEventDetector {
    const ID: IndicatorId = IndicatorId::Bos;
    /// No family — a structural DETECTOR (rolling-extremum breakout), an atomic producer
    /// consumed by name, not a pluggable oscillator member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed to High / Low — break detection operates on the bar's range.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// O(lookback) per bar: each call scans the preceding `lookback-1` slots.
    /// Two Vec windows (highs, lows) of depth `lookback`.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::Bos)];
    type Config = BosConfig;
    type Runtime = BosEventDetector;

    fn create(cfg: BosConfig) -> BosEventDetector {
        BosEventDetector::new(cfg.lookback.resolved())
    }
}

impl crate::contract::Config for BosConfig {
    fn defaults() -> Self {
        BosConfig { lookback: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // lookback: A period — auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for BosEventDetector {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(
                IndicatorOutputId::Bos,
                "Break of Structure",
                Color::hex(0xFF9800),
                2.0,
            ))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warmup(det: &mut BosEventDetector, n: usize, high: f64, low: f64) {
        for _ in 0..n {
            det.detect_from_values(high, low);
        }
    }

    #[test]
    fn not_ready_during_warmup() {
        let mut det = BosEventDetector::new(5);
        assert!(!det.is_ready());
        warmup(&mut det, 4, 101.0, 99.0);
        assert!(!det.is_ready());
        det.detect_from_values(101.0, 99.0);
        assert!(det.is_ready());
    }

    #[test]
    fn bos_up_detected_after_warmup() {
        let mut det = BosEventDetector::new(5);
        // Warmup with 5 stable bars (high=101, low=99)
        warmup(&mut det, 5, 101.0, 99.0);
        assert!(det.is_ready());
        // A bar whose high clearly exceeds the window maximum (101)
        let result = det.detect_from_values(115.0, 110.0);
        assert_eq!(
            result,
            Some((SignalKind::Structure(StructureSub::BOS), Direction::Up)),
            "high breakout should yield BOS Up"
        );
    }

    #[test]
    fn bos_down_detected_after_warmup() {
        let mut det = BosEventDetector::new(5);
        warmup(&mut det, 5, 101.0, 99.0);
        // A bar whose low clearly undercuts the window minimum (99)
        let result = det.detect_from_values(96.0, 85.0);
        assert_eq!(
            result,
            Some((SignalKind::Structure(StructureSub::BOS), Direction::Down)),
            "low breakout should yield BOS Down"
        );
    }

    #[test]
    fn normal_bar_returns_none() {
        let mut det = BosEventDetector::new(5);
        warmup(&mut det, 5, 101.0, 99.0);
        // Bar strictly inside the established range
        let result = det.detect_from_values(100.5, 99.5);
        assert_eq!(result, None, "bar inside range should produce no BOS");
    }

    #[test]
    fn reset_clears_state() {
        let mut det = BosEventDetector::new(5);
        warmup(&mut det, 5, 101.0, 99.0);
        det.detect_from_values(115.0, 110.0);
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.value(), 0.0);
    }

    /// The factory resolves the fixed High/Low lanes from `const SOURCE` (not the wild
    /// close or open) and feeds the pair; a clear upward breakout yields a positive signal.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f =
            IndicatorOrder::Bos(<<BosEventDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Warmup with 20 stable bars — high=101, low=99, wild open/close/volume ignored
        for _ in 0..20 {
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 101.0, low: 99.0, close: 9999.0, volume: 9999.0,
            });
        }
        // Feed a clear BOS-up bar: high >> window max (101)
        f.feed(0, MarketSample::Bar {
            open: 9999.0, high: 150.0, low: 140.0, close: 9999.0, volume: 9999.0,
        });
        assert!(f.primary() > 0.0, "BOS up must fire after high breakout; got {}", f.primary());
    }
}
