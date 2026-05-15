//! Stop Hunt Detector — detects coordinated liquidation spikes followed by
//! immediate price reversals ("stop hunts").
//!
//! # Algorithm
//!
//! Two rolling buffers are maintained:
//! - `liq_buf` — recent liquidation events: `(timestamp, quote_value, side)`.
//! - `price_buf` — recent mark-price snapshots: `(timestamp, price)`.
//!
//! On every `update_mark`, the detector checks whether in the last
//! `reversal_window_ms`:
//! 1. Total liquidation volume on **one side** exceeded `spike_threshold_usd`.
//! 2. Price moved in the reversal direction after the spike was established:
//!    - Bullish stop hunt: long liquidations dominate **and** latest price >
//!      oldest price in the window (shorts squeezed out, price bounces up).
//!    - Bearish stop hunt: short liquidations dominate **and** latest price <
//!      oldest price in the window (longs squeezed out, price drops).
//!
//! This struct implements both `LiquidationConsumer` and `MarkPriceConsumer`.
//! Both trait impls delegate to inherent methods (`indicator_value`,
//! `indicator_reset`, `indicator_is_ready`) to avoid ambiguity at call-sites.
//!
//! # Output
//! `Signal(i8)`:
//! - `+1` — bullish stop hunt (long liqs spiked, price reversed upward).
//! - `-1` — bearish stop hunt (short liqs spiked, price reversed downward).
//! - `0`  — no stop hunt detected.
//!
//! # Parameters
//! - `spike_threshold_usd` — minimum one-sided liq USD volume to qualify as a spike.
//! - `reversal_window_ms`  — look-back window for both liqs and price movement.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::liquidation_consumer::LiquidationConsumer;
use crate::bar_indicators::mark_price_consumer::MarkPriceConsumer;
use crate::core::types::{Liquidation, LiquidationSide, MarkPrice};

/// Stop hunt detector: liq spike + immediate price reversal.
#[derive(Clone)]
pub struct StopHuntDetector {
    /// USD volume threshold for a one-sided spike to be considered a stop hunt.
    spike_threshold_usd: f64,
    /// Rolling window used for both liq accumulation and price reversal check (ms).
    reversal_window_ms: i64,
    /// Recent liquidations: `(timestamp_ms, quote_value, side)`.
    liq_buf: VecDeque<(i64, f64, LiquidationSide)>,
    /// Recent mark prices: `(timestamp_ms, price)`.
    price_buf: VecDeque<(i64, f64)>,
    /// Cached last signal.
    last_signal: i8,
}

impl StopHuntDetector {
    /// Create a new detector.
    ///
    /// - `spike_threshold_usd` — one-sided liq USD volume required to qualify as a spike.
    /// - `reversal_window_ms`  — look-back window in milliseconds.
    pub fn new(spike_threshold_usd: f64, reversal_window_ms: i64) -> Self {
        Self {
            spike_threshold_usd: spike_threshold_usd.max(0.0),
            reversal_window_ms: reversal_window_ms.max(1),
            liq_buf: VecDeque::new(),
            price_buf: VecDeque::new(),
            last_signal: 0,
        }
    }

    fn evict_liqs(&mut self, now: i64) {
        while let Some(&(ts, _, _)) = self.liq_buf.front() {
            if now - ts > self.reversal_window_ms {
                self.liq_buf.pop_front();
            } else {
                break;
            }
        }
    }

    fn evict_prices(&mut self, now: i64) {
        while let Some(&(ts, _)) = self.price_buf.front() {
            if now - ts > self.reversal_window_ms {
                self.price_buf.pop_front();
            } else {
                break;
            }
        }
    }

    fn detect(&self) -> i8 {
        if self.price_buf.len() < 2 {
            return 0;
        }

        let mut long_vol = 0.0_f64;
        let mut short_vol = 0.0_f64;
        for &(_, val, side) in &self.liq_buf {
            match side {
                LiquidationSide::Long => long_vol += val,
                LiquidationSide::Short => short_vol += val,
            }
        }

        let oldest_price = self.price_buf.front().map(|&(_, p)| p).unwrap_or(0.0);
        let latest_price = self.price_buf.back().map(|&(_, p)| p).unwrap_or(0.0);

        if long_vol >= self.spike_threshold_usd && latest_price > oldest_price {
            return 1;
        }
        if short_vol >= self.spike_threshold_usd && latest_price < oldest_price {
            return -1;
        }
        0
    }

    /// Current indicator value — use this to avoid trait-method ambiguity.
    #[inline]
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    /// Reset internal state — use this to avoid trait-method ambiguity.
    pub fn indicator_reset(&mut self) {
        self.liq_buf.clear();
        self.price_buf.clear();
        self.last_signal = 0;
    }

    /// True when indicator has produced at least one non-zero detection or has
    /// both liq data and at least 2 price points. Use this to avoid ambiguity.
    pub fn indicator_is_ready(&self) -> bool {
        !self.liq_buf.is_empty() && self.price_buf.len() >= 2
    }
}

impl LiquidationConsumer for StopHuntDetector {
    fn update_liquidation(&mut self, liq: &Liquidation) -> IndicatorValue {
        self.liq_buf.push_back((liq.timestamp, liq.quote_value(), liq.side));
        self.evict_liqs(liq.timestamp);
        self.indicator_value()
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
    }

    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl MarkPriceConsumer for StopHuntDetector {
    fn update_mark(&mut self, mp: &MarkPrice) -> IndicatorValue {
        self.price_buf.push_back((mp.timestamp, mp.mark_price));
        self.evict_prices(mp.timestamp);
        self.evict_liqs(mp.timestamp);
        self.last_signal = self.detect();
        self.indicator_value()
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
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

    fn liq(ts: i64, side: LiquidationSide, price: f64, qty: f64) -> Liquidation {
        Liquidation { side, price, quantity: qty, timestamp: ts, value: None }
    }

    fn mp(ts: i64, price: f64) -> MarkPrice {
        MarkPrice {
            symbol: "BTCUSDT".to_string(),
            mark_price: price,
            index_price: None,
            funding_rate: None,
            timestamp: ts,
        }
    }

    #[test]
    fn no_signal_without_data() {
        let shd = StopHuntDetector::new(100_000.0, 5_000);
        assert_eq!(shd.indicator_value(), IndicatorValue::Signal(0));
        assert!(!shd.indicator_is_ready());
    }

    #[test]
    fn bullish_stop_hunt_detected() {
        // Long liquidation spike ($210k) + price goes up → bullish stop hunt.
        let mut shd = StopHuntDetector::new(100_000.0, 30_000);
        shd.update_liquidation(&liq(1_000, LiquidationSide::Long, 30_000.0, 7.0));
        shd.update_mark(&mp(2_000, 29_800.0));
        let signal = shd.update_mark(&mp(3_000, 30_200.0));
        assert_eq!(signal, IndicatorValue::Signal(1), "expected bullish stop hunt");
    }

    #[test]
    fn bearish_stop_hunt_detected() {
        // Short liquidation spike ($210k) + price goes down → bearish stop hunt.
        let mut shd = StopHuntDetector::new(100_000.0, 30_000);
        shd.update_liquidation(&liq(1_000, LiquidationSide::Short, 30_000.0, 7.0));
        shd.update_mark(&mp(2_000, 30_200.0));
        let signal = shd.update_mark(&mp(3_000, 29_800.0));
        assert_eq!(signal, IndicatorValue::Signal(-1), "expected bearish stop hunt");
    }

    #[test]
    fn below_threshold_no_signal() {
        // Only $10k liq — below $100k threshold.
        let mut shd = StopHuntDetector::new(100_000.0, 30_000);
        shd.update_liquidation(&liq(1_000, LiquidationSide::Long, 10_000.0, 1.0));
        shd.update_mark(&mp(2_000, 29_800.0));
        let signal = shd.update_mark(&mp(3_000, 30_200.0));
        assert_eq!(signal, IndicatorValue::Signal(0));
    }

    #[test]
    fn spike_but_no_reversal_no_signal() {
        // Large long liq but price goes DOWN — not a stop hunt.
        let mut shd = StopHuntDetector::new(100_000.0, 30_000);
        shd.update_liquidation(&liq(1_000, LiquidationSide::Long, 30_000.0, 7.0));
        shd.update_mark(&mp(2_000, 30_200.0));
        let signal = shd.update_mark(&mp(3_000, 29_800.0));
        assert_eq!(signal, IndicatorValue::Signal(0));
    }

    #[test]
    fn old_data_evicted() {
        // 5-second window. Liq at t=0, prices at t=20_000 and t=21_000.
        // Liq is outside window → no spike → no signal.
        let mut shd = StopHuntDetector::new(100_000.0, 5_000);
        shd.update_liquidation(&liq(0, LiquidationSide::Long, 30_000.0, 7.0));
        shd.update_mark(&mp(20_000, 29_800.0));
        let signal = shd.update_mark(&mp(21_000, 30_200.0));
        assert_eq!(signal, IndicatorValue::Signal(0), "old liq should be evicted");
    }

    #[test]
    fn reset_clears_state() {
        let mut shd = StopHuntDetector::new(100_000.0, 30_000);
        shd.update_liquidation(&liq(1_000, LiquidationSide::Long, 30_000.0, 7.0));
        shd.update_mark(&mp(2_000, 29_800.0));
        shd.update_mark(&mp(3_000, 30_200.0));
        shd.indicator_reset();
        assert_eq!(shd.indicator_value(), IndicatorValue::Signal(0));
        assert!(!shd.indicator_is_ready());
    }
}
