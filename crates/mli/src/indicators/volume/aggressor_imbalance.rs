//! AggressorImbalance — rolling buy-side vs sell-side tick frequency ratio.
//!
//! Counts buy ticks and sell ticks in a rolling window.
//! Output = `(buy_count - sell_count) / total_count` ∈ [-1.0, 1.0].
//!   +1.0 = all ticks are aggressor buys
//!   -1.0 = all ticks are aggressor sells
//!
//! Output: `imbalance`

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::contract::{
    Cost, Family, Indicator, Output, Render, SourceAxis, Store, StoreKind,
    UpdateComplexity, Color, RenderSpec,
};
use crate::core::types::Tick;
use crate::engine::stream_kind::StreamKind;

/// Rolling aggressor trade-count imbalance (not volume-weighted — pure frequency).
#[derive(Debug, Clone)]
pub struct AggressorImbalance {
    window: usize,
    /// Ring buffer: `true` = buy tick, `false` = sell tick.
    history: VecDeque<bool>,
    last_imbalance: f64,
}

impl AggressorImbalance {
    /// Create with `window` ticks rolling lookback.
    pub fn new(window: usize) -> Self {
        let cap = window.max(1);
        Self {
            window: cap,
            history: VecDeque::with_capacity(cap),
            last_imbalance: 0.0,
        }
    }
}

impl TickConsumer for AggressorImbalance {
    fn update_tick(&mut self, tick: &Tick) {
        self.history.push_back(tick.is_buy);
        if self.history.len() > self.window {
            self.history.pop_front();
        }

        let total = self.history.len();
        let buy = self.history.iter().filter(|&&b| b).count();
        let sell = total - buy;

        self.last_imbalance = if total > 0 {
            (buy as f64 - sell as f64) / total as f64
        } else {
            0.0
        };

    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_imbalance = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.history.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Tick;

    fn tick(is_buy: bool) -> Tick {
        Tick::new(0, 100.0, 1.0, is_buy)
    }

    #[test]
    fn all_buys_give_plus_one() {
        let mut ind = AggressorImbalance::new(10);
        for _ in 0..10 {
            ind.update_tick(&tick(true));
        }
        assert!((ind.last_imbalance - 1.0).abs() < 1e-9);
    }

    #[test]
    fn all_sells_give_minus_one() {
        let mut ind = AggressorImbalance::new(10);
        for _ in 0..10 {
            ind.update_tick(&tick(false));
        }
        assert!((ind.last_imbalance - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn equal_buys_sells_give_zero() {
        let mut ind = AggressorImbalance::new(4);
        ind.update_tick(&tick(true));
        ind.update_tick(&tick(false));
        ind.update_tick(&tick(true));
        ind.update_tick(&tick(false));
        assert!(ind.last_imbalance.abs() < 1e-9);
    }

    #[test]
    fn rolling_window_evicts_old_ticks() {
        let mut ind = AggressorImbalance::new(2);
        ind.update_tick(&tick(false));
        ind.update_tick(&tick(false));
        // Now push 2 buys — they fill the window
        ind.update_tick(&tick(true));
        ind.update_tick(&tick(true));
        assert!((ind.last_imbalance - 1.0).abs() < 1e-9);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = AggressorImbalance::new(5);
        for _ in 0..5 {
            ind.update_tick(&tick(true));
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }
}

impl Default for AggressorImbalance {
    fn default() -> Self {
        Self::new(50)
    }
}

// ---- Indicator contract ----

use crate::contract::Param;

/// Typed configuration for [`AggressorImbalance`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AggressorImbalanceConfig {
    /// Rolling window size in ticks.
    pub window: Param<usize>,
}

impl Indicator for AggressorImbalance {
    const ID: IndicatorId = IndicatorId::AggressorImbalance;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::AggressorImbalance),
    ];
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque)],
    );

    type Config = AggressorImbalanceConfig;
    type Runtime = AggressorImbalance;

    fn create(cfg: AggressorImbalanceConfig) -> AggressorImbalance {
        AggressorImbalance::new(cfg.window.resolved().max(1))
    }
}

impl crate::contract::Config for AggressorImbalanceConfig {
    fn defaults() -> Self {
        AggressorImbalanceConfig { window: Param::Solo(50) }
    }
    fn machine_defaults() -> Self {
        // window: Class A period/window — auto gives range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for AggressorImbalance {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::AggressorImbalance, "Aggressor Imbalance", Color::hex(0x26C6DA))
            .bounds(-1.0, 1.0)
            .zero_baseline()
            .precision(3)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_aggressor_imbalance() {
        let mut f = IndicatorOrder::AggressorImbalance(
            <<AggressorImbalance as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let t = Tick::new(0, 100.0, 1.0, true);
        f.feed(0, MarketSample::Tick(&t));
        assert!(f.read(IndicatorOutputId::AggressorImbalance).is_finite());
    }
}

impl AggressorImbalance {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_imbalance
    }
}
