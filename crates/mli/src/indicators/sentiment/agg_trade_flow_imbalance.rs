//! AggTradeFlowImbalance — rolling buy/sell volume imbalance over a time window.

use std::collections::VecDeque;

use crate::engine::streams::agg_trade_consumer::AggTradeConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::time_window::TimeWindow;
use crate::contract::render::Render;
use crate::contract::{Color, Cost, Family, Indicator, Output, Param, RenderSpec, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::core::types::AggTrade;
use crate::engine::stream_kind::StreamKind;

/// Computes `(buy_vol - sell_vol) / (buy_vol + sell_vol)` over a sliding time window.
///
/// Events older than `window_ms` milliseconds are dropped on each update.
///
/// Output: `imbalance` ∈ [-1, 1].
#[derive(Clone, Debug)]
pub struct AggTradeFlowImbalance {
    /// Window length in milliseconds.
    window_ms: i64,
    /// Buffered events: (timestamp_ms, quantity, is_buy).
    events: VecDeque<(i64, f64, bool)>,
    last_imbalance: f64,
}

impl AggTradeFlowImbalance {
    /// Create a new indicator.
    ///
    /// `window_ms` — rolling window in milliseconds. Default: 60 000 (1 minute).
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            events: VecDeque::new(),
            last_imbalance: 0.0,
        }
    }
}

/// Typed dual-mode config for [`AggTradeFlowImbalance`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AggTradeFlowImbalanceConfig {
    pub window: Param<TimeWindow>,
}

impl Indicator for AggTradeFlowImbalance {
    const ID: IndicatorId = IndicatorId::AggTradeFlowImbalance;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::AggTrade];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::AggTradeFlowImbalance)];
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);
    type Config = AggTradeFlowImbalanceConfig;
    type Runtime = AggTradeFlowImbalance;

    fn create(cfg: AggTradeFlowImbalanceConfig) -> AggTradeFlowImbalance {
        AggTradeFlowImbalance::new(cfg.window.resolved().as_millis())
    }
}

impl AggTradeConsumer for AggTradeFlowImbalance {
    fn update_agg_trade(&mut self, t: &AggTrade) {
        self.events.push_back((t.timestamp, t.quantity, t.is_buy));

        // Evict stale events.
        while let Some(&(ts, _, _)) = self.events.front() {
            if t.timestamp - ts > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }

        let (buy, sell) = self
            .events
            .iter()
            .fold((0.0f64, 0.0f64), |(b, s), &(_, q, is_buy)| {
                if is_buy {
                    (b + q, s)
                } else {
                    (b, s + q)
                }
            });

        let total = buy + sell;
        self.last_imbalance = if total > 0.0 {
            (buy - sell) / total
        } else {
            0.0
        };

    }


    fn reset(&mut self) {
        self.events.clear();
        self.last_imbalance = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

impl Default for AggTradeFlowImbalance {
    /// Factory default: window_ms=60_000 (1 minute).
    fn default() -> Self {
        Self::new(60_000)
    }
}

impl crate::contract::Config for AggTradeFlowImbalanceConfig {
    fn defaults() -> Self {
        AggTradeFlowImbalanceConfig { window: Param::Solo(TimeWindow::Minutes(1)) }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // Class N — TimeWindow: 11-value curated discrete set spanning 1s..24h
        s.window = Param::many(vec![
            TimeWindow::Seconds(1),
            TimeWindow::Seconds(5),
            TimeWindow::Seconds(15),
            TimeWindow::Seconds(30),
            TimeWindow::Minutes(1),
            TimeWindow::Minutes(5),
            TimeWindow::Minutes(15),
            TimeWindow::Minutes(30),
            TimeWindow::Hours(1),
            TimeWindow::Hours(4),
            TimeWindow::Hours(24),
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


impl Render for AggTradeFlowImbalance {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(
                IndicatorOutputId::AggTradeFlowImbalance,
                "Agg Trade Flow Imbalance",
                Color::hex(0x26C6DA),
            )
            .bounds(-1.0, 1.0)
            .zero_baseline()
            .precision(3)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;

    fn make_trade(timestamp: i64, quantity: f64, is_buy: bool) -> AggTrade {
        AggTrade {
            aggregate_id: 0,
            price: 100.0,
            quantity,
            first_trade_id: 0,
            last_trade_id: 0,
            is_buy,
            timestamp,
            ..Default::default()
        }
    }

    #[test]
    fn all_buy_gives_one() {
        let mut ind = AggTradeFlowImbalance::new(60_000);
        ind.update_agg_trade(&make_trade(1000, 5.0, true));
        ind.update_agg_trade(&make_trade(2000, 3.0, true));
        let v = ind.value();
        assert!((v - 1.0).abs() < 1e-9, "expected 1.0, got {v}");
    }

    #[test]
    fn all_sell_gives_minus_one() {
        let mut ind = AggTradeFlowImbalance::new(60_000);
        ind.update_agg_trade(&make_trade(1000, 5.0, false));
        ind.update_agg_trade(&make_trade(2000, 3.0, false));
        let v = ind.value();
        assert!((v + 1.0).abs() < 1e-9, "expected -1.0, got {v}");
    }

    #[test]
    fn equal_buy_sell_gives_zero() {
        let mut ind = AggTradeFlowImbalance::new(60_000);
        ind.update_agg_trade(&make_trade(1000, 4.0, true));
        ind.update_agg_trade(&make_trade(2000, 4.0, false));
        let v = ind.value();
        assert!(v.abs() < 1e-9, "expected 0.0, got {v}");
    }

    #[test]
    fn old_events_evicted() {
        let window = 60_000i64;
        let mut ind = AggTradeFlowImbalance::new(window);
        // old sell trade
        ind.update_agg_trade(&make_trade(0, 100.0, false));
        // new buy trade far in the future — old one should be dropped
        ind.update_agg_trade(&make_trade(window + 1, 1.0, true));
        let v = ind.value();
        assert!((v - 1.0).abs() < 1e-9, "old sell should be evicted, got {v}");
    }

    #[test]
    fn factory_feeds_resolved_agg_trade_flow_imbalance() {
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::AggTradeFlowImbalance(
            <<AggTradeFlowImbalance as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // Feed a buy trade
        let t = make_trade(1_000, 10.0, true);
        f.feed(0, MarketSample::AggTrade(&t));
        // Feed a sell trade of equal size
        let t2 = make_trade(2_000, 10.0, false);
        f.feed(0, MarketSample::AggTrade(&t2));
        let v = f.primary();
        assert!(v.abs() < 1e-9, "equal buy/sell should give 0.0, got {v}");
    }
}

impl AggTradeFlowImbalance {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_imbalance
    }
}
