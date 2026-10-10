//! Cumulative Volume Delta — rolling sum of real aggressor-side delta.
//!
//! A bar's OHLCV has no taker side, so a bar sample does not move the
//! accumulator. Each tick contributes `+size` (buy) or `−size` (sell).
//! [`CumulativeVolumeDelta::update_with_delta`] is the same step for a
//! host that already aggregated buy and sell volume.
//!
//! The rolling window counts those real deltas, not bars.

use std::collections::VecDeque;

/// Rolling Cumulative Volume Delta.
///
/// Output is the rolling sum of real aggressor delta.
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

    /// Push one real delta (`buy_volume - sell_volume`, or a tick's signed size)
    /// into the rolling window.
    fn push_delta(&mut self, delta: f64) {
        self.delta_history.push_back(delta);
        if self.delta_history.len() > self.window {
            if let Some(old) = self.delta_history.pop_front() {
                self.cumulative -= old;
            }
        }
        self.cumulative += delta;
    }

    /// Feed aggregated aggressor volume for one window and return the running total.
    pub fn update_with_delta(&mut self, buy_volume: f64, sell_volume: f64) -> f64 {
        self.push_delta(buy_volume - sell_volume);
        self.cumulative
    }

    /// Returns the last computed value without advancing state.
    pub fn value(&self) -> f64 {
        self.cumulative
    }

    /// Returns `true` after at least one real delta has been pushed.
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

    #[test]
    fn real_delta_accumulates() {
        let mut cvd = CumulativeVolumeDelta::new(10);
        let r = cvd.update_with_delta(500.0, 200.0);
        assert!((r - 300.0).abs() < 1e-9);
        let r = cvd.update_with_delta(100.0, 400.0);
        assert!((r - 0.0).abs() < 1e-9);
    }

    #[test]
    fn rolling_window_evicts_old_delta() {
        let mut cvd = CumulativeVolumeDelta::new(3);
        for _ in 0..3 {
            cvd.update_with_delta(100.0, 0.0);
        }
        // cumulative = 300; next delta −200 evicts the oldest +100 → 0.
        let r = cvd.update_with_delta(0.0, 200.0);
        assert!((r - 0.0).abs() < 1e-9, "expected 0, got {r}");
    }

    #[test]
    fn not_ready_before_first_real_delta() {
        let cvd = CumulativeVolumeDelta::new(5);
        assert!(!cvd.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut cvd = CumulativeVolumeDelta::new(5);
        cvd.update_with_delta(100.0, 0.0);
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
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Own config for [`CumulativeVolumeDelta`] — rolling window size for the CVD accumulator.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct CvdConfig {
    pub period: Param<usize>,
}

impl Indicator for CumulativeVolumeDelta {
    const ID: IndicatorId = IndicatorId::Cvd;
    /// Rolling CVD is a standalone accumulator — not a pluggable oscillator family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    /// O(1) rolling deque — one push + one pop per real delta.
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


impl TickConsumer for CumulativeVolumeDelta {
    fn update_tick(&mut self, tick: &Tick) {
        let delta = if tick.is_buy { tick.size } else { -tick.size };
        self.push_delta(delta);
    }

    fn reset(&mut self) {
        CumulativeVolumeDelta::reset(self);
    }

    fn is_ready(&self) -> bool {
        CumulativeVolumeDelta::is_ready(self)
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

    /// `IndicatorOrder` is the whole-universe enum. Its largest variant does not
    /// fit the default Windows test-thread stack, so factory proofs run here.
    fn on_wide_stack(f: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(f)
            .expect("spawn")
            .join()
            .unwrap_or_else(|payload| std::panic::resume_unwind(payload));
    }

    #[test]
    fn bar_feed_does_not_fabricate_delta() {
        on_wide_stack(|| {
            let mut f = IndicatorOrder::Cvd(<<CumulativeVolumeDelta as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
            f.feed(0, MarketSample::Bar { open: 100.0, high: 110.0, low: 90.0, close: 109.0, volume: 500.0 });
            f.feed(0, MarketSample::Bar { open: 109.0, high: 110.0, low: 90.0, close: 91.0, volume: 800.0 });
            assert!(!f.is_ready());
            let v = f.read(IndicatorOutputId::Cvd);
            assert!(v.abs() < 1e-9, "bar OHLCV must not move CVD, got {v}");
        });
    }

    #[test]
    fn tick_signed_size_accumulates() {
        on_wide_stack(|| {
            let mut f = IndicatorOrder::Cvd(<<CumulativeVolumeDelta as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
            let buy = crate::core::types::Tick::new(0, 100.0, 500.0, true);
            let sell = crate::core::types::Tick::new(1, 100.0, 200.0, false);
            f.feed(0, MarketSample::Tick(&buy));
            f.feed(1, MarketSample::Tick(&sell));
            let v = f.read(IndicatorOutputId::Cvd);
            assert!((v - 300.0).abs() < 1e-9, "expected +300 CVD, got {v}");
        });
    }
}
