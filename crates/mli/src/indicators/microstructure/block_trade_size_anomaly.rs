//! BlockTradeSizeAnomaly — z-score of current block trade size vs rolling history.
//!
//! Maintains a rolling window of `window` block trade quantities. On each new
//! event computes the population z-score of the current size relative to the
//! window mean and standard deviation.
//!
//! Output: `z_score`.
//! A high positive z-score indicates an unusually large block trade.

use std::collections::VecDeque;

use crate::engine::streams::block_trade_consumer::BlockTradeConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::Render;
use crate::contract::{Color, Cost, Family, Indicator, Output, RenderSpec, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::core::types::BlockTrade;
use crate::engine::stream_kind::StreamKind;

/// Rolling z-score anomaly detector for block trade sizes.
///
/// Parameter:
/// - `window` — number of past block trades used for the rolling baseline (≥ 2).
#[derive(Debug, Clone)]
pub struct BlockTradeSizeAnomaly {
    window: usize,
    sizes: VecDeque<f64>,
    last_z: f64,
}

impl BlockTradeSizeAnomaly {
    /// Create a new detector.
    ///
    /// `window` is clamped to ≥ 2 (z-score requires at least two observations).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            sizes: VecDeque::with_capacity(window.max(2) + 1),
            last_z: 0.0,
        }
    }
}

impl Default for BlockTradeSizeAnomaly {
    fn default() -> Self {
        Self::new(50)
    }
}

/// Typed dual-mode config for [`BlockTradeSizeAnomaly`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct BlockTradeSizeAnomalyConfig {
    pub window: crate::contract::Param<usize>,
}

impl Indicator for BlockTradeSizeAnomaly {
    const ID: IndicatorId = IndicatorId::BlockTradeSizeAnomaly;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::BlockTrade];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::BlockTradeSizeAnomaly)];
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);
    type Config = BlockTradeSizeAnomalyConfig;
    type Runtime = BlockTradeSizeAnomaly;

    fn create(cfg: BlockTradeSizeAnomalyConfig) -> BlockTradeSizeAnomaly {
        BlockTradeSizeAnomaly::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for BlockTradeSizeAnomalyConfig {
    fn defaults() -> Self {
        BlockTradeSizeAnomalyConfig { window: crate::contract::Param::Solo(50) }
    }
    fn machine_defaults() -> Self {
        // window: Param<usize> — Class A (event-count lookback window); auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for BlockTradeSizeAnomaly {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::BlockTradeSizeAnomaly, "Block Trade Size Z-Score", Color::hex(0xAB47BC))
            .precision(2)
            .zero_baseline()
            .build()
    }
}

impl BlockTradeConsumer for BlockTradeSizeAnomaly {
    fn update_block_trade(&mut self, bt: &BlockTrade) {
        self.sizes.push_back(bt.quantity);
        while self.sizes.len() > self.window {
            self.sizes.pop_front();
        }

        if self.sizes.len() >= 2 {
            let n = self.sizes.len() as f64;
            let mean: f64 = self.sizes.iter().sum::<f64>() / n;
            let var: f64 = self.sizes.iter().map(|&s| (s - mean).powi(2)).sum::<f64>() / n;
            let std = var.sqrt();
            self.last_z = if std > 1e-9 {
                (bt.quantity - mean) / std
            } else {
                0.0
            };
        }

    }


    fn reset(&mut self) {
        self.sizes.clear();
        self.last_z = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.sizes.len() >= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bt(quantity: f64) -> BlockTrade {
        BlockTrade {
            block_id: "test".to_string(),
            price: 100.0,
            quantity,
            is_buy: true,
            timestamp: 0,
            is_iv: false,
        }
    }

    #[test]
    fn uniform_sizes_give_zero_z() {
        let mut det = BlockTradeSizeAnomaly::new(5);
        for _ in 0..5 {
            det.update_block_trade(&bt(10.0));
        }
        // All identical → std = 0 → z = 0
        let z = det.value();
        assert!(z.abs() < 1e-9, "z should be 0 for uniform sizes, got {z}");
    }

    #[test]
    fn large_outlier_gives_high_positive_z() {
        let mut det = BlockTradeSizeAnomaly::new(10);
        // Fill window with small trades.
        for _ in 0..9 {
            det.update_block_trade(&bt(1.0));
        }
        // Insert a very large trade.
        det.update_block_trade(&bt(100.0));
        let z = det.value();
        assert!(z > 2.0, "large outlier should give z > 2, got {z}");
    }

    #[test]
    fn not_ready_until_two_events() {
        let mut det = BlockTradeSizeAnomaly::new(5);
        assert!(!det.is_ready());
        det.update_block_trade(&bt(10.0));
        assert!(!det.is_ready());
        det.update_block_trade(&bt(10.0));
        assert!(det.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut det = BlockTradeSizeAnomaly::new(5);
        det.update_block_trade(&bt(10.0));
        det.update_block_trade(&bt(20.0));
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_block_trade_size_anomaly() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::BlockTradeSizeAnomaly(<<BlockTradeSizeAnomaly as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for qty in [1.0_f64, 1.0, 1.0, 100.0] {
            let b = BlockTrade {
                block_id: "x".to_string(),
                price: 50_000.0,
                quantity: qty,
                is_buy: true,
                timestamp: 0,
                is_iv: false,
            };
            f.feed(0, MarketSample::BlockTrade(&b));
        }
        // outlier 100.0 should produce positive z
        let z = f.primary();
        assert!(z > 0.0, "z={z}");
    }
}

impl BlockTradeSizeAnomaly {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_z
    }
}
