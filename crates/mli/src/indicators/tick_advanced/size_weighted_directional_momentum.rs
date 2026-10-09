//! SizeWeightedDirectionalMomentum — rolling volume-weighted directional bias.
//!
//! Momentum = Σ(size × side) / Σ(size) over a time window, where
//!   side = +1 for buy ticks, -1 for sell ticks.
//!
//! Range: [-1.0, +1.0].
//!   +1.0 = all buying pressure
//!   -1.0 = all selling pressure
//!   0.0  = balanced
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

/// Rolling size-weighted directional momentum.
#[derive(Debug, Clone)]
pub struct SizeWeightedDirectionalMomentum {
    window_ms: i64,
    /// (timestamp_ms, signed_size)  signed_size = +size for buys, -size for sells
    events: VecDeque<(i64, f64)>,
    last_momentum: f64,
}

impl SizeWeightedDirectionalMomentum {
    /// Create with `window_ms` millisecond rolling window (default 60 000 ms).
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            events: VecDeque::with_capacity(512),
            last_momentum: 0.0,
        }
    }
}

impl TickConsumer for SizeWeightedDirectionalMomentum {
    fn update_tick(&mut self, tick: &Tick) {
        let signed = if tick.is_buy { tick.size } else { -tick.size };
        self.events.push_back((tick.time, signed));

        while let Some(&(ts, _)) = self.events.front() {
            if tick.time - ts > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }

        let sum_signed: f64 = self.events.iter().map(|&(_, s)| s).sum();
        let sum_abs: f64 = self.events.iter().map(|&(_, s)| s.abs()).sum();

        self.last_momentum = if sum_abs > 0.0 {
            // Clamp to [-1, 1] to guard against floating-point edge cases.
            (sum_signed / sum_abs).clamp(-1.0, 1.0)
        } else {
            0.0
        };

    }


    fn reset(&mut self) {
        self.events.clear();
        self.last_momentum = 0.0;
    }

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
    fn all_buys_give_plus_one() {
        let mut ind = SizeWeightedDirectionalMomentum::new(60_000);
        ind.update_tick(&tick(0, 5.0, true));
        ind.update_tick(&tick(1, 3.0, true));
        ind.update_tick(&tick(2, 2.0, true));
        let m = ind.value();
        assert!((m - 1.0).abs() < 1e-9);
    }

    #[test]
    fn balanced_gives_zero() {
        let mut ind = SizeWeightedDirectionalMomentum::new(60_000);
        ind.update_tick(&tick(0, 5.0, true));
        ind.update_tick(&tick(1, 5.0, false));
        let m = ind.value();
        assert!(m.abs() < 1e-9);
    }

    #[test]
    fn weighted_not_equal_count() {
        // 1 buy of size 9, 9 sells of size 1 each → buy volume = 9, sell volume = 9
        // → momentum = 0
        let mut ind = SizeWeightedDirectionalMomentum::new(60_000);
        ind.update_tick(&tick(0, 9.0, true));
        for i in 1..=9 {
            ind.update_tick(&tick(i, 1.0, false));
        }
        let m = ind.value();
        assert!(m.abs() < 1e-9, "balanced by volume: {}", m);
    }

    #[test]
    fn old_events_evicted() {
        let mut ind = SizeWeightedDirectionalMomentum::new(1_000);
        ind.update_tick(&tick(0, 100.0, false)); // large sell, old
        // 2 seconds later — sell evicted, only buy survives
        ind.update_tick(&tick(2_000, 1.0, true));
        let m = ind.value();
        assert!((m - 1.0).abs() < 1e-9, "only buy left: {}", m);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = SizeWeightedDirectionalMomentum::new(60_000);
        ind.update_tick(&tick(0, 5.0, true));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }
}

impl Default for SizeWeightedDirectionalMomentum {
    /// Factory default: window_ms=60000.
    fn default() -> Self {
        Self::new(60_000)
    }
}

// ---- Indicator contract ----

use crate::contract::Param;

/// Typed configuration for [`SizeWeightedDirectionalMomentum`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SizeWeightedDirectionalMomentumConfig {
    pub window: Param<TimeWindow>,
}

impl Indicator for SizeWeightedDirectionalMomentum {
    const ID: IndicatorId = IndicatorId::SizeWeightedDirectionalMomentum;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::SizeWeightedDirectionalMomentum)];
    type Config = SizeWeightedDirectionalMomentumConfig;
    type Runtime = SizeWeightedDirectionalMomentum;

    fn create(cfg: SizeWeightedDirectionalMomentumConfig) -> SizeWeightedDirectionalMomentum {
        SizeWeightedDirectionalMomentum::new(cfg.window.resolved().as_millis())
    }
}

impl crate::contract::Config for SizeWeightedDirectionalMomentumConfig {
    fn defaults() -> Self {
        SizeWeightedDirectionalMomentumConfig {
            window: Param::Solo(TimeWindow::Minutes(1)),
        }
    }
    fn machine_defaults() -> Self {
        // window (TimeWindow): Class N — 11-point curated discrete set.
        let mut s = Self::machine_defaults_auto();
        s.window = Param::many(vec![
            TimeWindow::Seconds(1), TimeWindow::Seconds(5), TimeWindow::Seconds(15),
            TimeWindow::Seconds(30), TimeWindow::Minutes(1), TimeWindow::Minutes(5),
            TimeWindow::Minutes(15), TimeWindow::Minutes(30),
            TimeWindow::Hours(1), TimeWindow::Hours(4), TimeWindow::Hours(24),
        ]);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for SizeWeightedDirectionalMomentum {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::SizeWeightedDirectionalMomentum, "SWDM", Color::hex(0x26A69A))
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
    fn factory_feeds_resolved_size_weighted_directional_momentum() {
        let mut f = IndicatorOrder::SizeWeightedDirectionalMomentum(
            <<SizeWeightedDirectionalMomentum as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let t = crate::core::types::Tick::new(0, 100.0, 5.0, true);
        f.feed(0, MarketSample::Tick(&t));
        assert!(f.primary().is_finite());
    }
}

impl SizeWeightedDirectionalMomentum {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_momentum
    }
}
