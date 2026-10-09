//! Trade Flow Imbalance — rolling signed volume imbalance from tick stream.
//!
//! Computes `(buy_vol - sell_vol) / (buy_vol + sell_vol)` over the last N ticks.
//! Output range: [-1.0, 1.0].
//!   +1.0 = all volume is buy-side (maximum buying pressure)
//!   -1.0 = all volume is sell-side (maximum selling pressure)
//!    0.0 = perfectly balanced
//!
//! Outputs: `imbalance`, `total_volume`

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::contract::{
    Cost, Family, Indicator, Output, Render, SourceAxis, Store, StoreKind,
    UpdateComplexity, Color, RenderOutput, RenderSpec,
};
use crate::core::types::Tick;
use crate::engine::stream_kind::StreamKind;

/// Trade Flow Imbalance — amount-weighted rolling buy/sell imbalance.
#[derive(Debug, Clone)]
pub struct TradeFlowImbalance {
    rolling_window_ticks: usize,
    /// Ring buffer of (size, is_buy) per tick.
    tick_history: VecDeque<(f64, bool)>,
    last_imbalance: f64,
    last_total_volume: f64,
}

impl TradeFlowImbalance {
    /// Create with `window` ticks lookback.
    pub fn new(window: usize) -> Self {
        let cap = window.max(1);
        Self {
            rolling_window_ticks: cap,
            tick_history: VecDeque::with_capacity(cap),
            last_imbalance: 0.0,
            last_total_volume: 0.0,
        }
    }
}

impl TickConsumer for TradeFlowImbalance {
    fn update_tick(&mut self, tick: &Tick) {
        self.tick_history.push_back((tick.size, tick.is_buy));
        if self.tick_history.len() > self.rolling_window_ticks {
            self.tick_history.pop_front();
        }

        let (mut buy, mut sell) = (0.0_f64, 0.0_f64);
        for &(sz, is_buy) in &self.tick_history {
            if is_buy {
                buy += sz;
            } else {
                sell += sz;
            }
        }

        self.last_total_volume = buy + sell;
        self.last_imbalance = if self.last_total_volume > 0.0 {
            (buy - sell) / self.last_total_volume
        } else {
            0.0
        };

    }


    fn reset(&mut self) {
        self.tick_history.clear();
        self.last_imbalance = 0.0;
        self.last_total_volume = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.tick_history.is_empty()
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
    fn test_all_buy_imbalance_plus_one() {
        let mut tfi = TradeFlowImbalance::new(10);
        for _ in 0..5 {
            tfi.update_tick(&tick(10.0, true));
        }
        assert!(tfi.is_ready());
        assert!((tfi.last_imbalance - 1.0).abs() < 1e-9);
        assert!((tfi.last_total_volume - 50.0).abs() < 1e-9);
    }

    #[test]
    fn test_balanced_imbalance_zero() {
        let mut tfi = TradeFlowImbalance::new(10);
        for _ in 0..5 {
            tfi.update_tick(&tick(10.0, true));
            tfi.update_tick(&tick(10.0, false));
        }
        assert!((tfi.last_imbalance - 0.0).abs() < 1e-9);
        assert!((tfi.last_total_volume - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_all_sell_imbalance_minus_one() {
        let mut tfi = TradeFlowImbalance::new(10);
        for _ in 0..5 {
            tfi.update_tick(&tick(10.0, false));
        }
        assert!((tfi.last_imbalance - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn test_rolling_window_evicts_old() {
        // window=2: only last 2 ticks matter
        let mut tfi = TradeFlowImbalance::new(2);
        // push 3 sell ticks — window keeps only last 2
        tfi.update_tick(&tick(10.0, false));
        tfi.update_tick(&tick(10.0, false));
        // now push 2 buy ticks — they fill the window
        tfi.update_tick(&tick(10.0, true));
        tfi.update_tick(&tick(10.0, true));
        // window = [buy, buy] → imbalance = +1.0
        assert!((tfi.last_imbalance - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_value_returns_same_as_last() {
        let mut tfi = TradeFlowImbalance::new(5);
        tfi.update_tick(&tick(20.0, true));
        assert_eq!(tfi.imbalance(), tfi.last_imbalance);
        assert_eq!(tfi.volume(), tfi.last_total_volume);
    }

    #[test]
    fn test_reset() {
        let mut tfi = TradeFlowImbalance::new(5);
        tfi.update_tick(&tick(10.0, true));
        tfi.reset();
        assert!(!tfi.is_ready());
        assert_eq!(tfi.imbalance(), 0.0);
        assert_eq!(tfi.volume(), 0.0);
    }
}

impl TradeFlowImbalance {
    #[inline]
    pub fn imbalance(&self) -> f64 {
        self.last_imbalance
    }

    #[inline]
    pub fn volume(&self) -> f64 {
        self.last_total_volume
    }
}

impl Default for TradeFlowImbalance {
    fn default() -> Self {
        Self::new(100)
    }
}

// ---- Indicator contract ----

use crate::contract::Param;

/// Typed configuration for [`TradeFlowImbalance`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct TradeFlowImbalanceConfig {
    /// Rolling window size in ticks.
    pub window: Param<usize>,
}

impl Indicator for TradeFlowImbalance {
    const ID: IndicatorId = IndicatorId::TradeFlowImbalance;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::TradeFlowImbalanceImbalance),
        Output::count(IndicatorOutputId::TradeFlowImbalanceVolume),
    ];
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque)],
    );

    type Config = TradeFlowImbalanceConfig;
    type Runtime = TradeFlowImbalance;

    fn create(cfg: TradeFlowImbalanceConfig) -> TradeFlowImbalance {
        TradeFlowImbalance::new(cfg.window.resolved().max(1))
    }
}

impl crate::contract::Config for TradeFlowImbalanceConfig {
    fn defaults() -> Self {
        TradeFlowImbalanceConfig { window: Param::Solo(100) }
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


impl Render for TradeFlowImbalance {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::TradeFlowImbalanceImbalance,
                "TFI",
                Color::hex(0x26C6DA),
                2.0,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::TradeFlowImbalanceVolume,
                "Total Vol",
                Color::hex(0x9E9E9E),
                1.0,
            ))
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
    fn factory_feeds_resolved_trade_flow_imbalance() {
        let mut f = IndicatorOrder::TradeFlowImbalance(
            <<TradeFlowImbalance as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let t = Tick::new(0, 100.0, 10.0, true);
        f.feed(0, MarketSample::Tick(&t));
        assert!(f.read(IndicatorOutputId::TradeFlowImbalanceImbalance).is_finite());
    }
}
