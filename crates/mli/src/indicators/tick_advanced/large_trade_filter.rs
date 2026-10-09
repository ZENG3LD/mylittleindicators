//! LargeTradeFilter — filters trades that are N× above rolling median size.
//!
//! Useful for detecting institutional activity, iceberg orders, or block trades.
//!
//! Outputs: `signal`, `size_ratio`
//!   signal:     +1.0 = large buy, -1.0 = large sell, 0.0 = normal trade
//!   size_ratio: current_size / rolling_median (how many × above median)

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::contract::{Family, Indicator, Output, SourceAxis};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::core::types::Tick;
use crate::engine::stream_kind::StreamKind;

/// Detects ticks whose size exceeds `multiplier × rolling_median_size`.
#[derive(Debug, Clone)]
pub struct LargeTradeFilter {
    window: usize,
    multiplier: f64,
    size_history: VecDeque<f64>,
    last_signal: f64,
    last_ratio: f64,
}

impl LargeTradeFilter {
    /// Create detector.
    ///
    /// - `window`: rolling window size for median computation.
    /// - `multiplier`: trades larger than `multiplier × median` are "large".
    pub fn new(window: usize, multiplier: f64) -> Self {
        let cap = window.max(2);
        Self {
            window: cap,
            multiplier: if multiplier > 0.0 { multiplier } else { 2.0 },
            size_history: VecDeque::with_capacity(cap),
            last_signal: 0.0,
            last_ratio: 0.0,
        }
    }

    /// Trade direction signal: `+1.0` (large buy), `-1.0` (large sell), `0.0` (normal).
    pub fn signal(&self) -> f64 {
        self.last_signal
    }

    /// `current_size / rolling_median` size ratio.
    pub fn ratio(&self) -> f64 {
        self.last_ratio
    }

    fn rolling_median(&self) -> f64 {
        if self.size_history.is_empty() {
            return 0.0;
        }
        let mut sorted: Vec<f64> = self.size_history.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid.saturating_sub(1)] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }
}

impl TickConsumer for LargeTradeFilter {
    fn update_tick(&mut self, tick: &Tick) {
        self.size_history.push_back(tick.size);
        if self.size_history.len() > self.window {
            self.size_history.pop_front();
        }

        if self.size_history.len() < self.window {
            return;
        }

        let median = self.rolling_median();
        let ratio = if median > 1e-12 {
            tick.size / median
        } else {
            0.0
        };

        let is_large = ratio >= self.multiplier;
        self.last_ratio = ratio;
        self.last_signal = if is_large {
            if tick.is_buy { 1.0 } else { -1.0 }
        } else {
            0.0
        };

    }


    fn reset(&mut self) {
        self.size_history.clear();
        self.last_signal = 0.0;
        self.last_ratio = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.size_history.len() >= self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Tick;

    fn tick(size: f64, is_buy: bool) -> Tick {
        Tick::new(0, 100.0, size, is_buy)
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut f = LargeTradeFilter::new(5, 3.0);
        for _ in 0..4 {
            f.update_tick(&tick(1.0, true));
            assert!(!f.is_ready());
            assert_eq!(f.signal(), 0.0);
            assert_eq!(f.ratio(), 0.0);
        }
        f.update_tick(&tick(1.0, true));
        assert!(f.is_ready());
    }

    #[test]
    fn large_buy_detected() {
        let mut f = LargeTradeFilter::new(5, 3.0);
        // Fill window with small ticks
        for _ in 0..4 {
            f.update_tick(&tick(1.0, true));
        }
        // 5th tick = 10× larger than median 1.0 → large buy
        f.update_tick(&tick(10.0, true));
        let sig = f.signal();
        let ratio = f.ratio();
        assert!((sig - 1.0).abs() < 1e-9, "expected large buy signal: {sig}");
        assert!(ratio >= 3.0, "ratio should be ≥ 3.0: {ratio}");
    }

    #[test]
    fn normal_tick_gives_zero_signal() {
        let mut f = LargeTradeFilter::new(5, 3.0);
        for _ in 0..5 {
            f.update_tick(&tick(1.0, true));
        }
        // Another normal-size tick
        f.update_tick(&tick(1.2, false));
        let sig = f.signal();
        assert!((sig - 0.0).abs() < 1e-9, "expected no signal: {sig}");
    }

    #[test]
    fn reset_clears_state() {
        let mut f = LargeTradeFilter::new(3, 2.0);
        for _ in 0..3 {
            f.update_tick(&tick(1.0, true));
        }
        f.reset();
        assert!(!f.is_ready());
        assert_eq!(f.signal(), 0.0);
        assert_eq!(f.ratio(), 0.0);
    }
}

impl Default for LargeTradeFilter {
    /// Factory default: window=50, multiplier=3.0.
    fn default() -> Self {
        Self::new(50, 3.0)
    }
}

// ---- Indicator contract ----

use crate::contract::{Param, sweep_f64};

/// Typed configuration for [`LargeTradeFilter`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LargeTradeFilterConfig {
    /// Rolling window size for median computation.
    pub window: Param<usize>,
    /// Multiplier: trades larger than `multiplier × median` are "large".
    pub multiplier: Param<f64>,
}

impl Indicator for LargeTradeFilter {
    const ID: IndicatorId = IndicatorId::LargeTradeFilter;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::LargeTradeFilterSignal),
        Output::ratio(IndicatorOutputId::LargeTradeFilterRatio),
    ];
    type Config = LargeTradeFilterConfig;
    type Runtime = LargeTradeFilter;

    fn create(cfg: LargeTradeFilterConfig) -> LargeTradeFilter {
        LargeTradeFilter::new(cfg.window.resolved(), cfg.multiplier.resolved())
    }
}

impl crate::contract::Config for LargeTradeFilterConfig {
    fn defaults() -> Self {
        LargeTradeFilterConfig {
            window: Param::Solo(50),
            multiplier: Param::Solo(3.0),
        }
    }
    fn machine_defaults() -> Self {
        // window (usize): Class A period — rolling median window; auto range(2,4048,1) is correct.
        // multiplier (f64): Class C multiplier — trades larger than multiplier × median; 0.1..10.0 step 0.1.
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


impl Render for LargeTradeFilter {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::LargeTradeFilterSignal, "Signal", Color::hex(0xFF9800), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::LargeTradeFilterRatio, "Size Ratio", Color::hex(0x9C27B0), 1.0))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_large_trade_filter() {
        let mut f = IndicatorOrder::LargeTradeFilter(
            <<LargeTradeFilter as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let t = crate::core::types::Tick::new(0, 100.0, 1.0, true);
        f.feed(0, MarketSample::Tick(&t));
        assert!(f.primary().is_finite());
    }
}
