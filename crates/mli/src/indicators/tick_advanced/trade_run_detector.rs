//! TradeRunDetector — consecutive same-side tick run counter.
//!
//! Increments a run counter while successive ticks are on the same side
//! (buy/sell). Resets the counter to 1 when the side flips.
//!
//! Outputs: `side_f64`, `run_length_f64`
//!   side: +1.0 = buy run, -1.0 = sell run
//!   run_length: how many consecutive same-side ticks (≥ 1)

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::contract::{Family, Indicator, Output, SourceAxis};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::core::types::Tick;
use crate::engine::stream_kind::StreamKind;

/// Consecutive same-side tick run detector.
///
/// `min_run_length` controls `is_ready`: the indicator signals as ready only
/// after at least one run of that length has been observed.
#[derive(Debug, Clone)]
pub struct TradeRunDetector {
    min_run: usize,
    current_run: usize,
    /// +1 = buy side, -1 = sell side, 0 = not yet started.
    current_side: i8,
    max_run_seen: usize,
}

impl TradeRunDetector {
    /// Create detector. `min_run_length` default is 3.
    pub fn new(min_run_length: usize) -> Self {
        Self {
            min_run: min_run_length.max(1),
            current_run: 0,
            current_side: 0,
            max_run_seen: 0,
        }
    }
}

impl TradeRunDetector {
    /// Current run side: `+1.0` (buy run), `-1.0` (sell run), `0.0` (not started).
    pub fn side(&self) -> f64 {
        self.current_side as f64
    }

    /// Number of consecutive same-side ticks in the current run.
    pub fn run_length(&self) -> f64 {
        self.current_run as f64
    }
}

impl TickConsumer for TradeRunDetector {
    fn update_tick(&mut self, tick: &Tick) {
        let side: i8 = if tick.is_buy { 1 } else { -1 };

        if self.current_side == side {
            self.current_run += 1;
        } else {
            self.current_side = side;
            self.current_run = 1;
        }

        if self.current_run > self.max_run_seen {
            self.max_run_seen = self.current_run;
        }

    }


    fn reset(&mut self) {
        self.current_run = 0;
        self.current_side = 0;
        self.max_run_seen = 0;
    }

    fn is_ready(&self) -> bool {
        self.max_run_seen >= self.min_run
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buy_tick() -> Tick {
        Tick::new(0, 100.0, 1.0, true)
    }

    fn sell_tick() -> Tick {
        Tick::new(0, 100.0, 1.0, false)
    }

    #[test]
    fn run_increments_on_same_side() {
        let mut det = TradeRunDetector::new(3);
        for expected_run in 1..=5usize {
            det.update_tick(&buy_tick());
            assert!((det.side() - 1.0).abs() < 1e-9);
            assert!((det.run_length() - expected_run as f64).abs() < 1e-9);
        }
    }

    #[test]
    fn run_resets_on_side_change() {
        let mut det = TradeRunDetector::new(1);
        det.update_tick(&buy_tick());
        det.update_tick(&buy_tick());
        det.update_tick(&sell_tick());
        assert!((det.side() - (-1.0)).abs() < 1e-9, "side should be sell");
        assert!((det.run_length() - 1.0).abs() < 1e-9, "run resets to 1");
    }

    #[test]
    fn is_ready_after_min_run_reached() {
        let mut det = TradeRunDetector::new(3);
        assert!(!det.is_ready());
        det.update_tick(&buy_tick());
        det.update_tick(&buy_tick());
        assert!(!det.is_ready());
        det.update_tick(&buy_tick());
        assert!(det.is_ready(), "min_run=3 reached");
    }

    #[test]
    fn reset_clears_state() {
        let mut det = TradeRunDetector::new(3);
        for _ in 0..5 {
            det.update_tick(&buy_tick());
        }
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.side(), 0.0);
        assert_eq!(det.run_length(), 0.0);
    }
}

impl Default for TradeRunDetector {
    /// Factory default: min_run_length=3.
    fn default() -> Self {
        Self::new(3)
    }
}

// ---- Indicator contract ----

use crate::contract::Param;

/// Typed configuration for [`TradeRunDetector`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct TradeRunDetectorConfig {
    /// Minimum consecutive same-side ticks to consider "ready".
    pub min_run_length: Param<usize>,
}

impl Indicator for TradeRunDetector {
    const ID: IndicatorId = IndicatorId::TradeRunDetector;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::TradeRunDetectorSide),
        Output::count(IndicatorOutputId::TradeRunDetectorRunLength),
    ];
    type Config = TradeRunDetectorConfig;
    type Runtime = TradeRunDetector;

    fn create(cfg: TradeRunDetectorConfig) -> TradeRunDetector {
        TradeRunDetector::new(cfg.min_run_length.resolved())
    }
}

impl crate::contract::Config for TradeRunDetectorConfig {
    fn defaults() -> Self {
        TradeRunDetectorConfig {
            min_run_length: Param::Solo(3),
        }
    }
    fn machine_defaults() -> Self {
        // min_run_length (usize): Class B count — consecutive same-side ticks threshold;
        //   audit §1.2 lists min_run_length range 2..=20 step 1.
        //   Auto would give 2..4048 (too wide for a run-length gate). Corrected.
        let mut s = Self::machine_defaults_auto();
        s.min_run_length = Param::range(2, 20, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for TradeRunDetector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::TradeRunDetectorSide, "Side", Color::hex(0x26A69A), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::TradeRunDetectorRunLength, "Run Length", Color::hex(0xFF7043), 1.0))
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
    fn factory_feeds_resolved_trade_run_detector() {
        let mut f = IndicatorOrder::TradeRunDetector(
            <<TradeRunDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let t = crate::core::types::Tick::new(0, 100.0, 1.0, true);
        f.feed(0, MarketSample::Tick(&t));
        assert!(f.primary().is_finite());
    }
}
