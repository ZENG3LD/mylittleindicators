//! LargeTickMomentum — directional momentum of large-size ticks only.
//!
//! Filters ticks by a fixed `size_threshold`. Only ticks whose size exceeds
//! the threshold contribute to momentum. Momentum is the volume-weighted
//! directional bias of those large ticks over a rolling time window.
//!
//! momentum = Σ(size × side) / Σ(size)  for large ticks in window
//!   side = +1 for buy, -1 for sell
//!   range: [-1.0, +1.0]
//!
//! Output: `momentum`

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::engine::time_window::TimeWindow;
use crate::contract::{Family, Indicator, Output, SourceAxis};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::core::types::Tick;
use crate::engine::stream_kind::StreamKind;

/// Large-tick-only directional momentum over a rolling time window.
#[derive(Debug, Clone)]
pub struct LargeTickMomentum {
    size_threshold: f64,
    window_ms: i64,
    /// (timestamp_ms, size, is_buy) — only large ticks stored.
    events: VecDeque<(i64, f64, bool)>,
    last_momentum: f64,
}

impl LargeTickMomentum {
    /// Create indicator.
    ///
    /// - `size_threshold`: minimum tick size to include (e.g. 1.0 BTC).
    /// - `window_ms`: rolling time window in milliseconds.
    pub fn new(size_threshold: f64, window_ms: i64) -> Self {
        Self {
            size_threshold: size_threshold.max(0.0),
            window_ms: window_ms.max(1),
            events: VecDeque::with_capacity(256),
            last_momentum: 0.0,
        }
    }
}

impl TickConsumer for LargeTickMomentum {
    fn update_tick(&mut self, tick: &Tick) {
        // Only record large ticks.
        if tick.size > self.size_threshold {
            self.events.push_back((tick.time, tick.size, tick.is_buy));
        }

        // Evict old large ticks outside the window.
        while let Some(&(ts, _, _)) = self.events.front() {
            if tick.time - ts > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }

        let sum_signed: f64 = self.events.iter().map(|&(_, sz, buy)| {
            if buy { sz } else { -sz }
        }).sum();
        let sum_abs: f64 = self.events.iter().map(|&(_, sz, _)| sz).sum();

        self.last_momentum = if sum_abs > 0.0 {
            (sum_signed / sum_abs).clamp(-1.0, 1.0)
        } else {
            0.0
        };

    }


    fn reset(&mut self) {
        self.events.clear();
        self.last_momentum = 0.0;
    }

    /// Ready once at least one large tick is in the window.
    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(time_ms: i64, size: f64, is_buy: bool) -> Tick {
        Tick::new(time_ms, 100.0, size, is_buy)
    }

    #[test]
    fn small_ticks_ignored() {
        let mut ind = LargeTickMomentum::new(5.0, 60_000);
        // All ticks below threshold.
        ind.update_tick(&tick(0, 1.0, true));
        ind.update_tick(&tick(1, 2.0, false));
        ind.update_tick(&tick(2, 4.9, true));
        assert!(!ind.is_ready(), "no large ticks → not ready");
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn large_buy_ticks_give_plus_one() {
        let mut ind = LargeTickMomentum::new(5.0, 60_000);
        ind.update_tick(&tick(0, 10.0, true));
        ind.update_tick(&tick(1, 20.0, true));
        let m = ind.value();
        assert!((m - 1.0).abs() < 1e-9);
    }

    #[test]
    fn balanced_large_ticks_give_zero() {
        let mut ind = LargeTickMomentum::new(5.0, 60_000);
        ind.update_tick(&tick(0, 10.0, true));
        ind.update_tick(&tick(1, 10.0, false));
        let m = ind.value();
        assert!(m.abs() < 1e-9);
    }

    #[test]
    fn old_large_ticks_evicted() {
        let mut ind = LargeTickMomentum::new(5.0, 1_000); // 1 second window
        ind.update_tick(&tick(0, 100.0, false)); // large sell, old
        // 2 seconds later — sell evicted, large buy arrives.
        ind.update_tick(&tick(2_000, 10.0, true));
        let m = ind.value();
        assert!((m - 1.0).abs() < 1e-9);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = LargeTickMomentum::new(5.0, 60_000);
        ind.update_tick(&tick(0, 10.0, true));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }
}

impl Default for LargeTickMomentum {
    /// Factory default: size_threshold=1.0, window_ms=60000.
    fn default() -> Self {
        Self::new(1.0, 60_000)
    }
}

// ---- Indicator contract ----

use crate::contract::{Param, sweep_f64};

/// Typed configuration for [`LargeTickMomentum`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LargeTickMomentumConfig {
    /// Minimum tick size to count as "large".
    pub size_threshold: Param<f64>,
    /// Rolling time window.
    pub window: Param<TimeWindow>,
}

impl Indicator for LargeTickMomentum {
    const ID: IndicatorId = IndicatorId::LargeTickMomentum;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::LargeTickMomentum)];
    type Config = LargeTickMomentumConfig;
    type Runtime = LargeTickMomentum;

    fn create(cfg: LargeTickMomentumConfig) -> LargeTickMomentum {
        LargeTickMomentum::new(cfg.size_threshold.resolved(), cfg.window.resolved().as_millis())
    }
}

impl crate::contract::Config for LargeTickMomentumConfig {
    fn defaults() -> Self {
        LargeTickMomentumConfig {
            size_threshold: Param::Solo(1.0),
            window: Param::Solo(TimeWindow::Minutes(1)),
        }
    }
    fn machine_defaults() -> Self {
        // window (TimeWindow): Class N — 11-point curated discrete set.
        // size_threshold (f64): ambiguous — minimum tick size in base currency (default 1.0 BTC).
        //   Instrument-relative (like Class I price_bucket). Swept as Class F shape 0.1..5.0 step
        //   0.1 for machine search; user should narrow to instrument-appropriate range. Flagged.
        let mut s = Self::machine_defaults_auto();
        s.window = Param::many(vec![
            TimeWindow::Seconds(1), TimeWindow::Seconds(5), TimeWindow::Seconds(15),
            TimeWindow::Seconds(30), TimeWindow::Minutes(1), TimeWindow::Minutes(5),
            TimeWindow::Minutes(15), TimeWindow::Minutes(30),
            TimeWindow::Hours(1), TimeWindow::Hours(4), TimeWindow::Hours(24),
        ]);
        s.size_threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for LargeTickMomentum {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::LargeTickMomentum, "Large Tick Momentum", Color::hex(0xAB47BC))
            .bounds(-1.0, 1.0)
            .zero_baseline()
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
    fn factory_feeds_resolved_large_tick_momentum() {
        let mut f = IndicatorOrder::LargeTickMomentum(
            <<LargeTickMomentum as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let t = crate::core::types::Tick::new(0, 100.0, 5.0, true);
        f.feed(0, MarketSample::Tick(&t));
        assert!(f.primary().is_finite());
    }
}

impl LargeTickMomentum {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_momentum
    }
}
