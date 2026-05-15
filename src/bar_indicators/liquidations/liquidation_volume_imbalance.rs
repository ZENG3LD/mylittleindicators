//! Liquidation Volume Imbalance — rolling long vs short liquidation volume ratio.
//!
//! Measures the balance of forced-close volume between long and short positions
//! over a rolling window.
//!
//! # Output
//! `Triple(imbalance, long_vol, short_vol)`
//!
//! - `imbalance ∈ [-1, 1]`:
//!   - `+1.0` — all volume is short liquidations (shorts forced-buy → bullish pressure).
//!   - `-1.0` — all volume is long liquidations (longs forced-sell → bearish pressure).
//!   - `0.0`  — balanced.
//! - `long_vol`  — cumulative quote volume of long liquidations in window.
//! - `short_vol` — cumulative quote volume of short liquidations in window.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::liquidation_consumer::LiquidationConsumer;
use crate::core::types::{Liquidation, TradeSide};

/// Rolling liquidation volume imbalance.
#[derive(Clone)]
pub struct LiquidationVolumeImbalance {
    /// Rolling window length in milliseconds.
    window_ms: i64,
    /// Buffered events: (timestamp, quote_value, side).
    events: VecDeque<(i64, f64, TradeSide)>,
    /// Cached imbalance.
    last_imbalance: f64,
    /// Cached long volume.
    last_long_vol: f64,
    /// Cached short volume.
    last_short_vol: f64,
}

impl LiquidationVolumeImbalance {
    /// Create with the given rolling window.
    ///
    /// `window_ms` — window size in milliseconds.
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            events: VecDeque::new(),
            last_imbalance: 0.0,
            last_long_vol: 0.0,
            last_short_vol: 0.0,
        }
    }

    fn evict(&mut self, now: i64) {
        while let Some(&(ts, _, _)) = self.events.front() {
            if now - ts > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }
    }

    fn recompute(&mut self) {
        let mut long_vol = 0.0_f64;
        let mut short_vol = 0.0_f64;
        for &(_, val, side) in &self.events {
            // TradeSide::Buy = long was liquidated (forced sell)
            // TradeSide::Sell = short was liquidated (forced buy)
            match side {
                TradeSide::Buy => long_vol += val,
                TradeSide::Sell => short_vol += val,
            }
        }
        let total = long_vol + short_vol;
        self.last_long_vol = long_vol;
        self.last_short_vol = short_vol;
        // positive = more short liquidations = bullish pressure (shorts forced to buy)
        self.last_imbalance = if total > 0.0 {
            (short_vol - long_vol) / total
        } else {
            0.0
        };
    }
}

impl LiquidationConsumer for LiquidationVolumeImbalance {
    fn update_liquidation(&mut self, liq: &Liquidation) -> IndicatorValue {
        self.events.push_back((liq.timestamp, liq.quote_value(), liq.side));
        self.evict(liq.timestamp);
        self.recompute();
        IndicatorValue::Triple(self.last_imbalance, self.last_long_vol, self.last_short_vol)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_imbalance, self.last_long_vol, self.last_short_vol)
    }

    fn reset(&mut self) {
        self.events.clear();
        self.last_imbalance = 0.0;
        self.last_long_vol = 0.0;
        self.last_short_vol = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn liq(ts: i64, side: TradeSide, price: f64, qty: f64) -> Liquidation {
        Liquidation { symbol: String::new(), side, price, quantity: qty, timestamp: ts, value: None }
    }

    #[test]
    fn zero_initially() {
        let lvi = LiquidationVolumeImbalance::new(60_000);
        assert_eq!(lvi.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
        assert!(!lvi.is_ready());
    }

    #[test]
    fn pure_long_liquidations_give_neg_one() {
        let mut lvi = LiquidationVolumeImbalance::new(60_000);
        lvi.update_liquidation(&liq(0, TradeSide::Buy, 30_000.0, 1.0));
        lvi.update_liquidation(&liq(1_000, TradeSide::Buy, 30_000.0, 1.0));
        if let IndicatorValue::Triple(imb, _lv, _sv) = lvi.value() {
            assert!((imb - (-1.0)).abs() < 1e-9, "imb={imb}");
        }
    }

    #[test]
    fn pure_short_liquidations_give_pos_one() {
        let mut lvi = LiquidationVolumeImbalance::new(60_000);
        lvi.update_liquidation(&liq(0, TradeSide::Sell, 30_000.0, 1.0));
        lvi.update_liquidation(&liq(1_000, TradeSide::Sell, 30_000.0, 1.0));
        if let IndicatorValue::Triple(imb, _lv, _sv) = lvi.value() {
            assert!((imb - 1.0).abs() < 1e-9, "imb={imb}");
        }
    }

    #[test]
    fn equal_volumes_give_zero_imbalance() {
        let mut lvi = LiquidationVolumeImbalance::new(60_000);
        lvi.update_liquidation(&liq(0, TradeSide::Buy, 30_000.0, 1.0));
        lvi.update_liquidation(&liq(1_000, TradeSide::Sell, 30_000.0, 1.0));
        if let IndicatorValue::Triple(imb, lv, sv) = lvi.value() {
            assert!((imb).abs() < 1e-9, "imb={imb}");
            assert!((lv - 30_000.0).abs() < 1e-6);
            assert!((sv - 30_000.0).abs() < 1e-6);
        }
    }

    #[test]
    fn old_events_evicted() {
        let mut lvi = LiquidationVolumeImbalance::new(10_000);
        // long liq at t=0 (will be evicted)
        lvi.update_liquidation(&liq(0, TradeSide::Buy, 30_000.0, 1.0));
        // short liq at t=15_000 (outside window for t=0)
        lvi.update_liquidation(&liq(15_000, TradeSide::Sell, 30_000.0, 1.0));
        // only short remains → imbalance = +1
        if let IndicatorValue::Triple(imb, _lv, _sv) = lvi.value() {
            assert!((imb - 1.0).abs() < 1e-9, "imb={imb}");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut lvi = LiquidationVolumeImbalance::new(60_000);
        lvi.update_liquidation(&liq(0, TradeSide::Buy, 30_000.0, 1.0));
        lvi.reset();
        assert!(!lvi.is_ready());
        assert_eq!(lvi.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }
}
