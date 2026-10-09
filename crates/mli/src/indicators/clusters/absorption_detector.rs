//! AbsorptionDetector — detects large volume at minimal price movement.
//!
//! Absorption score = total_volume / (price_range + eps).
//! High score = someone absorbing order flow (price stays flat despite volume).
//!
//! Direction signal:
//!   +1.0 = buy absorption (buy_vol > sell_vol * 1.5, price flat)
//!   -1.0 = sell absorption (sell_vol > buy_vol * 1.5, price flat)
//!    0.0 = neutral / warming up
//!
//! Outputs: `score`, `signal`

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::Tick;

/// Detects absorption: high volume with minimal price movement.
#[derive(Debug, Clone)]
pub struct AbsorptionDetector {
    rolling_window_ticks: usize,
    /// Ring buffer of (price, size, is_buy) per tick.
    tick_buffer: VecDeque<(f64, f64, bool)>,
    last_absorption_score: f64,
    last_signal: f64,
}

impl AbsorptionDetector {
    /// Create with `window` ticks lookback.
    pub fn new(window: usize) -> Self {
        let cap = window.max(2);
        Self {
            rolling_window_ticks: cap,
            tick_buffer: VecDeque::with_capacity(cap),
            last_absorption_score: 0.0,
            last_signal: 0.0,
        }
    }
}

impl AbsorptionDetector {
    /// Named output: brace `score`.
    pub fn score(&self) -> f64 { self.last_absorption_score }
    /// Named output: brace `signal`.
    pub fn signal(&self) -> f64 { self.last_signal }
}

impl TickConsumer for AbsorptionDetector {
    fn update_tick(&mut self, tick: &Tick) {
        self.tick_buffer.push_back((tick.price, tick.size, tick.is_buy));
        if self.tick_buffer.len() > self.rolling_window_ticks {
            self.tick_buffer.pop_front();
        }
        if self.tick_buffer.len() < self.rolling_window_ticks {
            return;
        }

        let total_volume: f64 = self.tick_buffer.iter().map(|&(_, s, _)| s).sum();
        let first_price = self.tick_buffer.front().map(|&(p, _, _)| p).unwrap_or(0.0);
        let last_price = self.tick_buffer.back().map(|&(p, _, _)| p).unwrap_or(0.0);
        let price_range = (last_price - first_price).abs();

        let score = if price_range > 1e-9 {
            total_volume / price_range
        } else if total_volume > 0.0 {
            total_volume * 1000.0
        } else {
            0.0
        };

        let buy_vol: f64 = self.tick_buffer.iter()
            .filter(|&&(_, _, is_buy)| is_buy)
            .map(|&(_, s, _)| s)
            .sum();
        let sell_vol = total_volume - buy_vol;

        let signal = if buy_vol > sell_vol * 1.5 {
            1.0
        } else if sell_vol > buy_vol * 1.5 {
            -1.0
        } else {
            0.0
        };

        self.last_absorption_score = score;
        self.last_signal = signal;

    }


    fn reset(&mut self) {
        self.tick_buffer.clear();
        self.last_absorption_score = 0.0;
        self.last_signal = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.tick_buffer.len() >= self.rolling_window_ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Tick;

    fn tick(price: f64, size: f64, is_buy: bool) -> Tick {
        Tick::new(0, price, size, is_buy)
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut det = AbsorptionDetector::new(5);
        for _ in 0..4 {
            det.update_tick(&tick(100.0, 10.0, true));
            assert!(!det.is_ready());
        }
        det.update_tick(&tick(100.0, 10.0, true));
        assert!(det.is_ready());
    }

    #[test]
    fn flat_price_all_buy_gives_buy_absorption() {
        let mut det = AbsorptionDetector::new(4);
        // all ticks at same price, all buy → score high, signal +1
        for _ in 0..4 {
            det.update_tick(&tick(100.0, 10.0, true));
        }
        let score = det.score();
        let signal = det.signal();
        assert!(score > 100.0, "score should be large when price flat: {}", score);
        assert!((signal - 1.0).abs() < 1e-9, "signal should be +1: {}", signal);
    }

    #[test]
    fn reset_clears_state() {
        let mut det = AbsorptionDetector::new(3);
        for _ in 0..3 {
            det.update_tick(&tick(100.0, 10.0, true));
        }
        assert!(det.is_ready());
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.score(), 0.0);
        assert_eq!(det.signal(), 0.0);
    }
}

impl Default for AbsorptionDetector {
    /// Factory default: window=50.
    fn default() -> Self {
        Self::new(50)
    }
}

/// Typed dual-mode config for [`AbsorptionDetector`]: rolling tick-window size.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AbsorptionDetectorConfig {
    /// Rolling lookback in number of ticks (minimum 2).
    pub window: Param<usize>,
}

impl Indicator for AbsorptionDetector {
    const ID: IndicatorId = IndicatorId::AbsorptionDetector;
    /// Not a pluggable family — a tick-stream absorption signal detector.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::magnitude(IndicatorOutputId::AbsorptionDetectorScore),
        Output::discrete(IndicatorOutputId::AbsorptionDetectorSignal),
    ];
    /// O(1) per tick (VecDeque push/pop); rolling window VecDeque.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    type Config = AbsorptionDetectorConfig;
    type Runtime = Self;

    fn create(cfg: AbsorptionDetectorConfig) -> Self {
        Self::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for AbsorptionDetectorConfig {
    fn defaults() -> Self {
        AbsorptionDetectorConfig { window: Param::Solo(50) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // window: Class A period/window usize → auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
}


impl Render for AbsorptionDetector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::AbsorptionDetectorScore, "Absorption Score", Color::hex(0x2196F3))
            .line_output(IndicatorOutputId::AbsorptionDetectorSignal, "Signal", Color::hex(0xF44336))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;
    use crate::core::types::Tick as CoreTick;

    #[test]
    fn factory_feeds_resolved_tick() {
        let mut f = IndicatorOrder::AbsorptionDetector(
            <<AbsorptionDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();

        // Feed 50 identical-price buy ticks → absorption score > 0, signal +1
        for _ in 0..50 {
            let t = CoreTick::new(0, 100.0, 10.0, true);
            f.feed(0, MarketSample::Tick(&t));
        }
        assert!(f.is_ready());
        // main() = absorption score — should be large (flat price, high volume)
        assert!(f.primary() > 0.0, "absorption score should be > 0, got {}", f.primary());
    }
}
