//! AggTradeSizeDistribution — rolling median, p95 and current trade size.

use std::collections::VecDeque;

use crate::engine::streams::agg_trade_consumer::AggTradeConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::render::Render;
use crate::contract::{Color, Cost, Family, Indicator, Output, Param, RenderSpec, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::core::types::AggTrade;
use crate::engine::stream_kind::StreamKind;

/// Tracks the size distribution of the last `window_size` aggregated trades.
///
/// Computes median and 95th percentile on every update.
///
/// Outputs: `median`, `p95`, `current_size`.
#[derive(Clone, Debug)]
pub struct AggTradeSizeDistribution {
    window_size: usize,
    sizes: VecDeque<f64>,
    last_median: f64,
    last_p95: f64,
    last_current: f64,
}

impl AggTradeSizeDistribution {
    /// Create a new indicator. `window_size` is clamped to at least 1.
    pub fn new(window_size: usize) -> Self {
        let window_size = window_size.max(1);
        Self {
            window_size,
            sizes: VecDeque::with_capacity(window_size),
            last_median: 0.0,
            last_p95: 0.0,
            last_current: 0.0,
        }
    }
}

impl AggTradeSizeDistribution {
    /// Rolling median trade size over the window.
    pub fn median(&self) -> f64 {
        self.last_median
    }

    /// 95th-percentile trade size over the window.
    pub fn p95(&self) -> f64 {
        self.last_p95
    }

    /// Size of the most recent aggregated trade.
    pub fn current(&self) -> f64 {
        self.last_current
    }
}

/// Typed dual-mode config for [`AggTradeSizeDistribution`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AggTradeSizeDistributionConfig {
    pub window_size: Param<usize>,
}

impl Indicator for AggTradeSizeDistribution {
    const ID: IndicatorId = IndicatorId::AggTradeSizeDistribution;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::AggTrade];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::count(IndicatorOutputId::AggTradeSizeDistributionMedian),
        Output::count(IndicatorOutputId::AggTradeSizeDistributionP95),
        Output::count(IndicatorOutputId::AggTradeSizeDistributionCurrent),
    ];
    /// O(n log n) per update (sort on each call).
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[
        Store::window(StoreKind::Deque),
        Store::window(StoreKind::SortedVec),
    ]);
    type Config = AggTradeSizeDistributionConfig;
    type Runtime = AggTradeSizeDistribution;

    fn create(cfg: AggTradeSizeDistributionConfig) -> AggTradeSizeDistribution {
        AggTradeSizeDistribution::new(cfg.window_size.resolved())
    }
}

impl AggTradeConsumer for AggTradeSizeDistribution {
    fn update_agg_trade(&mut self, t: &AggTrade) {
        self.sizes.push_back(t.quantity);
        while self.sizes.len() > self.window_size {
            self.sizes.pop_front();
        }

        let mut sorted: Vec<f64> = self.sizes.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let n = sorted.len();
        self.last_median = if n > 0 { sorted[n / 2] } else { 0.0 };
        self.last_p95 = if n > 0 {
            let idx = ((n as f64 - 1.0) * 0.95) as usize;
            sorted[idx]
        } else {
            0.0
        };
        self.last_current = t.quantity;

    }


    fn reset(&mut self) {
        self.sizes.clear();
        self.last_median = 0.0;
        self.last_p95 = 0.0;
        self.last_current = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.sizes.is_empty()
    }
}

impl Default for AggTradeSizeDistribution {
    /// Factory default: window_size=100.
    fn default() -> Self {
        Self::new(100)
    }
}

impl crate::contract::Config for AggTradeSizeDistributionConfig {
    fn defaults() -> Self {
        AggTradeSizeDistributionConfig { window_size: Param::Solo(100) }
    }
    fn machine_defaults() -> Self {
        // window_size is Class A (plain count/window usize) — auto provides range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for AggTradeSizeDistribution {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(
                IndicatorOutputId::AggTradeSizeDistributionMedian,
                "Median Size",
                Color::hex(0x42A5F5),
            )
            .line_output(
                IndicatorOutputId::AggTradeSizeDistributionP95,
                "P95 Size",
                Color::hex(0xFF7043),
            )
            .line_output(
                IndicatorOutputId::AggTradeSizeDistributionCurrent,
                "Current Size",
                Color::hex(0xABCD4E),
            )
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;

    fn make_trade(quantity: f64) -> AggTrade {
        AggTrade {
            aggregate_id: 0,
            price: 100.0,
            quantity,
            first_trade_id: 0,
            last_trade_id: 0,
            is_buy: true,
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn single_trade() {
        let mut ind = AggTradeSizeDistribution::new(10);
        ind.update_agg_trade(&make_trade(5.0));
        assert_eq!(ind.current(), 5.0);
        assert_eq!(ind.median(), 5.0);
        assert_eq!(ind.p95(), 5.0);
    }

    #[test]
    fn median_correct_for_sorted_window() {
        let mut ind = AggTradeSizeDistribution::new(5);
        for q in [1.0, 2.0, 3.0, 4.0, 5.0] {
            ind.update_agg_trade(&make_trade(q));
        }
        // window = [1,2,3,4,5], sorted[2] = 3.0
        let median = ind.median();
        assert_eq!(median, 3.0, "expected median 3.0, got {median}");
    }

    #[test]
    fn window_rolls_out_old_trades() {
        let mut ind = AggTradeSizeDistribution::new(3);
        for q in [1.0, 1.0, 1.0, 100.0, 100.0, 100.0] {
            ind.update_agg_trade(&make_trade(q));
        }
        // window should now be [100, 100, 100]
        let median = ind.median();
        assert_eq!(median, 100.0, "old small trades should be evicted");
    }

    #[test]
    fn reset_works() {
        let mut ind = AggTradeSizeDistribution::new(5);
        ind.update_agg_trade(&make_trade(10.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.median(), 0.0);
        assert_eq!(ind.p95(), 0.0);
        assert_eq!(ind.current(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_agg_trade_size_distribution() {
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::AggTradeSizeDistribution(
            <<AggTradeSizeDistribution as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // feed a known size
        let t = AggTrade {
            aggregate_id: 0,
            price: 9999.0, // wild value — should not affect size distribution
            quantity: 7.0,
            first_trade_id: 0,
            last_trade_id: 0,
            is_buy: true,
            timestamp: 0,
            ..Default::default()
        };
        f.feed(0, MarketSample::AggTrade(&t));
        // f.value() = first output (median)
        let median = f.primary();
        assert_eq!(median, 7.0, "median of single trade should be 7.0, got {median}");
    }
}
