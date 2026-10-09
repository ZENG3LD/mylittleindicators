//! Cumulative Volume Delta — rolling sum of estimated buy/sell delta.
//!
//! Without a real tick stream the delta per bar is estimated from the candle
//! direction:
//!
//! - `close > open` → bullish bar → `+volume` (buy pressure)
//! - `close < open` → bearish bar → `−volume` (sell pressure)
//! - `close == open` → doji → `0`
//!
//! The rolling window (`window` bars) keeps the CVD anchored to recent
//! history rather than accumulating from the beginning of time.

use std::collections::VecDeque;

/// Rolling Cumulative Volume Delta.
///
/// Output is `Single(cumulative_delta)` — an unbounded oscillator that
/// trends positive when buy pressure dominates and negative when sell
/// pressure dominates.
#[derive(Debug, Clone)]
pub struct CumulativeVolumeDelta {
    window: usize,
    delta_history: VecDeque<f64>,
    cumulative: f64,
}

impl CumulativeVolumeDelta {
    /// Create a new `CumulativeVolumeDelta` with the given rolling `window`.
    ///
    /// Minimum window is 1.
    pub fn new(window: usize) -> Self {
        let w = window.max(1);
        Self {
            window: w,
            delta_history: VecDeque::with_capacity(w + 1),
            cumulative: 0.0,
        }
    }

    /// Feed resolved lanes `[open, close, volume]` and return `Single(cumulative_delta)`.
    ///
    /// Uses a synthetic estimate: `delta = +volume` if close > open,
    /// `-volume` if close < open, else `0`.
    pub fn feed(&mut self, lanes: &[f64]) {
        let open = lanes[0];
        let close = lanes[1];
        let volume = lanes[2];
        const EPS: f64 = 1e-12;
        let delta = if close > open + EPS {
            volume
        } else if close < open - EPS {
            -volume
        } else {
            0.0
        };

        self.delta_history.push_back(delta);
        if self.delta_history.len() > self.window {
            if let Some(old) = self.delta_history.pop_front() {
                self.cumulative -= old;
            }
        }
        self.cumulative += delta;


    }

    /// Returns the last computed value without advancing state.
    pub fn value(&self) -> f64 {
        self.cumulative
    }

    /// Returns `true` after at least one bar has been fed.
    pub fn is_ready(&self) -> bool {
        !self.delta_history.is_empty()
    }

    /// Clears all accumulated state.
    pub fn reset(&mut self) {
        self.delta_history.clear();
        self.cumulative = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // feed lanes: [open, close, volume]
    fn bullish(cvd: &mut CumulativeVolumeDelta, vol: f64) -> f64 {
        cvd.feed(&[100.0, 101.0, vol]); // close > open
        cvd.value()
    }
    fn bearish(cvd: &mut CumulativeVolumeDelta, vol: f64) -> f64 {
        cvd.feed(&[101.0, 100.0, vol]); // close < open
        cvd.value()
    }
    fn doji(cvd: &mut CumulativeVolumeDelta, vol: f64) -> f64 {
        cvd.feed(&[100.0, 100.0, vol]); // close == open
        cvd.value()
    }

    #[test]
    fn bullish_bar_adds_volume() {
        let mut cvd = CumulativeVolumeDelta::new(10);
        let r = bullish(&mut cvd, 500.0);
        assert!((r - 500.0).abs() < 1e-9);
    }

    #[test]
    fn bearish_bar_subtracts_volume() {
        let mut cvd = CumulativeVolumeDelta::new(10);
        let r = bearish(&mut cvd, 500.0);
        assert!((r - (-500.0)).abs() < 1e-9);
    }

    #[test]
    fn doji_bar_zero_delta() {
        let mut cvd = CumulativeVolumeDelta::new(10);
        let r = doji(&mut cvd, 500.0);
        assert!((r - 0.0).abs() < 1e-9);
    }

    #[test]
    fn rolling_window_evicts_old_delta() {
        let mut cvd = CumulativeVolumeDelta::new(3);
        // Feed 3 bullish bars (+100 each) to fill window.
        for _ in 0..3 {
            bullish(&mut cvd, 100.0);
        }
        // cumulative = 300; 4th bar (bearish -200): evict oldest +100, add -200 → 0.
        let r = bearish(&mut cvd, 200.0);
        assert!((r - 0.0).abs() < 1e-9, "expected 0, got {r}");
    }

    #[test]
    fn not_ready_before_first_bar() {
        let cvd = CumulativeVolumeDelta::new(5);
        assert!(!cvd.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut cvd = CumulativeVolumeDelta::new(5);
        bullish(&mut cvd, 100.0);
        cvd.reset();
        assert!(!cvd.is_ready());
        assert!((cvd.value() - 0.0).abs() < 1e-9);
    }
}

impl Default for CumulativeVolumeDelta {
    fn default() -> Self {
        Self::new(50)
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity, Store, StoreKind};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::engine::ohlcv_field::OhlcvField;

/// Own config for [`CumulativeVolumeDelta`] — rolling window size for the CVD accumulator.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct CvdConfig {
    pub period: Param<usize>,
}

impl Indicator for CumulativeVolumeDelta {
    const ID: IndicatorId = IndicatorId::Cvd;
    /// Rolling CVD is a standalone accumulator — not a pluggable oscillator family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed lanes: open (lane 0, direction detection) + volume (lane 1, magnitude).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
    /// O(1) rolling deque — one push + one pop per bar.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    const OUTPUTS: &'static [Output] = &[Output::flow(IndicatorOutputId::Cvd)];
    type Config = CvdConfig;
    type Runtime = CumulativeVolumeDelta;

    fn create(cfg: CvdConfig) -> CumulativeVolumeDelta {
        CumulativeVolumeDelta::new(cfg.period.resolved().max(1))
    }
}

impl crate::contract::Config for CvdConfig {
    fn defaults() -> Self {
        CvdConfig { period: Param::Solo(50) }
    }
    fn machine_defaults() -> Self {
        // period: Class A period/window — auto gives range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for CumulativeVolumeDelta {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Cvd, "CVD", Color::hex(0x009688))
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
    fn factory_feeds_resolved_lanes() {
        let mut f = IndicatorOrder::Cvd(<<CumulativeVolumeDelta as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Bullish bar: close > open → +volume
        f.feed(0, MarketSample::Bar { open: 100.0, high: 9999.0, low: 9999.0, close: 101.0, volume: 500.0 });
        // Bearish bar: close < open → -volume
        f.feed(0, MarketSample::Bar { open: 102.0, high: 9999.0, low: 9999.0, close: 100.0, volume: 300.0 });
        // Net delta = +500 - 300 = +200
        let v = f.read(IndicatorOutputId::Cvd);
        assert!((v - 200.0).abs() < 1e-9, "expected +200 CVD, got {v}");
    }
}
