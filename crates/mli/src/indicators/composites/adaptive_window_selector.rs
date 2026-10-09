//! AdaptiveWindowSelector — meta-indicator that selects optimal window based on volatility.
//!
//! Consumer: `TickConsumer` (for volatility tracking via price std).
//!
//! Logic:
//! - Maintains rolling price history in `short_window` and `long_window` sizes.
//! - Computes rolling std in short window.
//! - If std > `volatility_threshold` → returns `short_window` (react faster in volatile regimes).
//! - If std <= threshold → returns `long_window` (smoother signal in calm regimes).
//!
//! Output: `Single(recommended_window_size)` as f64.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::contract::sweep_f64;
use crate::core::types::Tick;
use crate::engine::stream_kind::StreamKind;

/// Volatility-adaptive window size selector.
///
/// Implements `TickConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Debug, Clone)]
pub struct AdaptiveWindowSelector {
    short_window: usize,
    long_window: usize,
    volatility_threshold: f64,
    prices: VecDeque<f64>,
    last_window: f64,
}

impl AdaptiveWindowSelector {
    /// Create a new indicator.
    ///
    /// - `short_window`          — window size returned in high-vol regime (default 10).
    /// - `long_window`           — window size returned in low-vol regime (default 100).
    /// - `volatility_threshold`  — rolling std threshold in price units (default 10.0).
    pub fn new(short_window: usize, long_window: usize, volatility_threshold: f64) -> Self {
        let cap = long_window.max(2);
        Self {
            short_window: short_window.max(2),
            long_window: long_window.max(short_window).max(2),
            volatility_threshold,
            prices: VecDeque::with_capacity(cap),
            last_window: long_window as f64,
        }
    }

    fn rolling_std(&self, n: usize) -> f64 {
        let count = self.prices.len().min(n);
        if count < 2 {
            return 0.0;
        }
        let slice_start = self.prices.len().saturating_sub(count);
        let iter = self.prices.iter().skip(slice_start);
        let mean: f64 = iter.clone().sum::<f64>() / count as f64;
        let variance: f64 = iter.map(|p| (p - mean).powi(2)).sum::<f64>() / (count - 1) as f64;
        variance.sqrt()
    }

    fn recompute(&mut self) {
        let std = self.rolling_std(self.short_window);
        self.last_window = if std > self.volatility_threshold {
            self.short_window as f64
        } else {
            self.long_window as f64
        };
    }


    /// Primary scalar output (the single value the macro reads).
    pub fn value(&self) -> f64 {
        self.last_window
    }

    /// True when short_window prices have been received.
    pub fn indicator_is_ready(&self) -> bool {
        self.prices.len() >= self.short_window
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.prices.clear();
        self.last_window = self.long_window as f64;
    }
}

impl Default for AdaptiveWindowSelector {
    fn default() -> Self {
        Self::new(10, 100, 10.0)
    }
}

impl TickConsumer for AdaptiveWindowSelector {
    fn update_tick(&mut self, tick: &Tick) {
        if self.prices.len() >= self.long_window {
            self.prices.pop_front();
        }
        self.prices.push_back(tick.price);
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(price: f64) -> Tick {
        Tick::new(0, price, 1.0, true)
    }

    #[test]
    fn high_vol_returns_short_window() {
        let mut ind = AdaptiveWindowSelector::new(5, 20, 10.0);
        // Feed very volatile prices
        let prices = [100.0, 150.0, 80.0, 200.0, 60.0, 180.0];
        for p in prices {
            ind.update_tick(&tick(p));
        }
        let w = ind.value();
        assert_eq!(w as usize, 5, "expected short window 5, got {w}");
    }

    #[test]
    fn low_vol_returns_long_window() {
        let mut ind = AdaptiveWindowSelector::new(5, 20, 100.0); // very high threshold
        for i in 0..20 {
            ind.update_tick(&tick(100.0 + i as f64 * 0.01)); // tiny moves
        }
        let w = ind.value();
        assert_eq!(w as usize, 20, "expected long window 20, got {w}");
    }

    #[test]
    fn reset_restores_long_window() {
        let mut ind = AdaptiveWindowSelector::new(5, 20, 1.0);
        for i in 0..10 {
            ind.update_tick(&tick(100.0 + i as f64 * 50.0));
        }
        ind.indicator_reset();
        let w = ind.value();
        assert_eq!(w as usize, 20);
        assert!(!ind.indicator_is_ready());
    }
}

// ---- Indicator contract ----

/// Typed configuration for [`AdaptiveWindowSelector`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AdaptiveWindowSelectorConfig {
    /// Window size returned in high-volatility regimes.
    pub short_window: Param<usize>,
    /// Window size returned in low-volatility regimes.
    pub long_window: Param<usize>,
    /// Rolling std threshold in price units.
    pub volatility_threshold: Param<f64>,
}

impl Indicator for AdaptiveWindowSelector {
    const ID: IndicatorId = IndicatorId::AdaptiveWindowSelector;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::count(IndicatorOutputId::AdaptiveWindowSelector),
    ];
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = AdaptiveWindowSelectorConfig;
    type Runtime = AdaptiveWindowSelector;

    fn create(cfg: AdaptiveWindowSelectorConfig) -> AdaptiveWindowSelector {
        AdaptiveWindowSelector::new(
            cfg.short_window.resolved(),
            cfg.long_window.resolved(),
            cfg.volatility_threshold.resolved(),
        )
    }
}

impl crate::contract::Config for AdaptiveWindowSelectorConfig {
    fn defaults() -> Self {
        AdaptiveWindowSelectorConfig {
            short_window: Param::Solo(10),
            long_window: Param::Solo(100),
            volatility_threshold: Param::Solo(10.0),
        }
    }
    fn machine_defaults() -> Self {
        // short_window (usize): Class A period — auto gives range(2,4048,1) ✓
        // long_window (usize): Class A period — auto gives range(2,4048,1) ✓
        // volatility_threshold (f64): Class F — rolling std threshold in price units;
        //   sweep_f64(0.1,5.0,0.1) gives the sigma-scale range appropriate for normalised price moves.
        //   NOTE: generator must enforce short_window < long_window as a structural constraint.
        let mut s = Self::machine_defaults_auto();
        s.volatility_threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for AdaptiveWindowSelector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::AdaptiveWindowSelector, "Adaptive Window", Color::hex(0x66BB6A))
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
    fn factory_feeds_resolved_adaptive_window_selector() {
        let mut f = IndicatorOrder::AdaptiveWindowSelector(
            <<AdaptiveWindowSelector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let t = crate::core::types::Tick::new(0, 100.0, 1.0, true);
        f.feed(0, MarketSample::Tick(&t));
        assert!(f.primary().is_finite());
    }
}
