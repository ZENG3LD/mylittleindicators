//! BlockTradeSizeAnomaly — z-score of current block trade size vs rolling history.
//!
//! Maintains a rolling window of `window` block trade quantities. On each new
//! event computes the population z-score of the current size relative to the
//! window mean and standard deviation.
//!
//! Output: `IndicatorValue::Single(z_score)`.
//! A high positive z-score indicates an unusually large block trade.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::BlockTradeConsumer;
use crate::core::types::BlockTrade;

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

impl BlockTradeConsumer for BlockTradeSizeAnomaly {
    fn update_block_trade(&mut self, bt: &BlockTrade) -> IndicatorValue {
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

        IndicatorValue::Single(self.last_z)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_z)
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
        if let IndicatorValue::Single(z) = det.value() {
            assert!(z.abs() < 1e-9, "z should be 0 for uniform sizes, got {z}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn large_outlier_gives_high_positive_z() {
        let mut det = BlockTradeSizeAnomaly::new(10);
        // Fill window with small trades.
        for _ in 0..9 {
            det.update_block_trade(&bt(1.0));
        }
        // Insert a very large trade.
        if let IndicatorValue::Single(z) = det.update_block_trade(&bt(100.0)) {
            assert!(z > 2.0, "large outlier should give z > 2, got {z}");
        } else {
            panic!("expected Single");
        }
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
        assert_eq!(det.value(), IndicatorValue::Single(0.0));
    }
}
