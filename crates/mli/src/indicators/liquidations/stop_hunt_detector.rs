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

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::liquidation_consumer::LiquidationConsumer;
use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::{Liquidation, TradeSide, MarkPrice};

/// Stop hunt detector: liq spike + immediate price reversal.
#[derive(Debug, Clone)]
pub struct StopHuntDetector {
    /// USD volume threshold for a one-sided spike to be considered a stop hunt.
    spike_threshold_usd: f64,
    /// Rolling window used for both liq accumulation and price reversal check (ms).
    reversal_window_ms: i64,
    /// Recent liquidations: `(timestamp_ms, quote_value, side)`.
    liq_buf: VecDeque<(i64, f64, TradeSide)>,
    /// Recent mark prices: `(timestamp_ms, price)`.
    price_buf: VecDeque<(i64, f64)>,
    /// Cached last signal.
    last_signal: i8,
}

impl Default for StopHuntDetector {
    fn default() -> Self {
        Self::new(500_000.0, 10_000)
    }
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
                TradeSide::Buy => long_vol += val,
                TradeSide::Sell => short_vol += val,
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
    pub fn indicator_value(&self) -> f64 {
        self.last_signal as f64
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
    fn update_liquidation(&mut self, liq: &Liquidation) {
        self.liq_buf.push_back((liq.timestamp, liq.quote_value(), liq.side));
        self.evict_liqs(liq.timestamp);
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl MarkPriceConsumer for StopHuntDetector {
    fn update_mark(&mut self, mp: &MarkPrice) {
        self.price_buf.push_back((mp.timestamp, mp.mark_price));
        self.evict_prices(mp.timestamp);
        self.evict_liqs(mp.timestamp);
        self.last_signal = self.detect();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

use crate::contract::{Param, sweep_f64};

/// Typed dual-mode configuration for [`StopHuntDetector`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct StopHuntDetectorConfig {
    /// Minimum one-sided liquidation USD volume to qualify as a spike.
    pub spike_threshold_usd: Param<f64>,
    /// Look-back window in milliseconds.
    pub reversal_window_ms: Param<i64>,
}

impl Indicator for StopHuntDetector {
    const ID: IndicatorId = IndicatorId::StopHuntDetector;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Liquidation, StreamKind::MarkPrice];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::StopHuntDetector)];
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[
            Store::window(StoreKind::Deque),
            Store::window(StoreKind::Deque),
        ],
    );
    type Config = StopHuntDetectorConfig;
    type Runtime = StopHuntDetector;

    fn create(cfg: StopHuntDetectorConfig) -> StopHuntDetector {
        StopHuntDetector::new(cfg.spike_threshold_usd.resolved(), cfg.reversal_window_ms.resolved())
    }
}

impl crate::contract::Config for StopHuntDetectorConfig {
    fn defaults() -> Self {
        StopHuntDetectorConfig {
            spike_threshold_usd: Param::Solo(500_000.0),
            reversal_window_ms: Param::Solo(10_000),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // spike_threshold_usd: dimensioned USD volume — instrument-relative (Class F placeholder).
        // Default 500_000 USD; placeholder sweep covers the sigma-scaled relative range 0.1..5.0.
        // AMBIGUOUS: true scale depends on the symbol's typical liquidation volume.
        // Chosen: sweep_f64(0.1, 5.0, 0.1) as relative-scale placeholder; flag instrument-relative.
        s.spike_threshold_usd = Param::many(sweep_f64(0.1, 5.0, 0.1));
        // reversal_window_ms: Class M reversal detection window — 5 explicit values in ms.
        s.reversal_window_ms = Param::many(vec![500i64, 1000, 2000, 5000, 10000]);
        s
    }
}


impl Render for StopHuntDetector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(IndicatorOutputId::StopHuntDetector, "Stop Hunt Signal", Color::hex(0xE91E63)))
            .bounds(-1.5, 1.5)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn liq(ts: i64, side: TradeSide, price: f64, qty: f64) -> Liquidation {
        Liquidation { symbol: String::new(), side, price, quantity: qty, timestamp: ts, value: None, ..Default::default()}
    }

    fn mp(ts: i64, price: f64) -> MarkPrice {
        MarkPrice {
            mark_price: price,
            index_price: None,
            funding_rate: None,
            timestamp: ts,
            ..Default::default()
        }
    }

    #[test]
    fn no_signal_without_data() {
        let shd = StopHuntDetector::new(100_000.0, 5_000);
        assert_eq!(shd.indicator_value() as i8, 0);
        assert!(!shd.indicator_is_ready());
    }

    #[test]
    fn bullish_stop_hunt_detected() {
        // Long liquidation spike ($210k) + price goes up → bullish stop hunt.
        let mut shd = StopHuntDetector::new(100_000.0, 30_000);
        shd.update_liquidation(&liq(1_000, TradeSide::Buy, 30_000.0, 7.0));
        shd.update_mark(&mp(2_000, 29_800.0));
        shd.update_mark(&mp(3_000, 30_200.0));
        assert_eq!(shd.indicator_value() as i8, 1, "expected bullish stop hunt");
    }

    #[test]
    fn bearish_stop_hunt_detected() {
        // Short liquidation spike ($210k) + price goes down → bearish stop hunt.
        let mut shd = StopHuntDetector::new(100_000.0, 30_000);
        shd.update_liquidation(&liq(1_000, TradeSide::Sell, 30_000.0, 7.0));
        shd.update_mark(&mp(2_000, 30_200.0));
        shd.update_mark(&mp(3_000, 29_800.0));
        assert_eq!(shd.indicator_value() as i8, -1, "expected bearish stop hunt");
    }

    #[test]
    fn below_threshold_no_signal() {
        // Only $10k liq — below $100k threshold.
        let mut shd = StopHuntDetector::new(100_000.0, 30_000);
        shd.update_liquidation(&liq(1_000, TradeSide::Buy, 10_000.0, 1.0));
        shd.update_mark(&mp(2_000, 29_800.0));
        shd.update_mark(&mp(3_000, 30_200.0));
        assert_eq!(shd.indicator_value() as i8, 0);
    }

    #[test]
    fn spike_but_no_reversal_no_signal() {
        // Large long liq but price goes DOWN — not a stop hunt.
        let mut shd = StopHuntDetector::new(100_000.0, 30_000);
        shd.update_liquidation(&liq(1_000, TradeSide::Buy, 30_000.0, 7.0));
        shd.update_mark(&mp(2_000, 30_200.0));
        shd.update_mark(&mp(3_000, 29_800.0));
        assert_eq!(shd.indicator_value() as i8, 0);
    }

    #[test]
    fn old_data_evicted() {
        // 5-second window. Liq at t=0, prices at t=20_000 and t=21_000.
        // Liq is outside window → no spike → no signal.
        let mut shd = StopHuntDetector::new(100_000.0, 5_000);
        shd.update_liquidation(&liq(0, TradeSide::Buy, 30_000.0, 7.0));
        shd.update_mark(&mp(20_000, 29_800.0));
        shd.update_mark(&mp(21_000, 30_200.0));
        assert_eq!(shd.indicator_value() as i8, 0, "old liq should be evicted");
    }

    #[test]
    fn reset_clears_state() {
        let mut shd = StopHuntDetector::new(100_000.0, 30_000);
        shd.update_liquidation(&liq(1_000, TradeSide::Buy, 30_000.0, 7.0));
        shd.update_mark(&mp(2_000, 29_800.0));
        shd.update_mark(&mp(3_000, 30_200.0));
        shd.indicator_reset();
        assert_eq!(shd.indicator_value() as i8, 0);
        assert!(!shd.indicator_is_ready());
    }

    #[test]
    fn factory_feeds_resolved_stop_hunt_detector() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::StopHuntDetector(StopHuntDetectorConfig {
            spike_threshold_usd: Param::Solo(100_000.0),
            reversal_window_ms: Param::Solo(30_000),
        })
        .build_solo()
        .unwrap();
        let l = liq(1_000, TradeSide::Buy, 30_000.0, 7.0);
        let p1 = mp(2_000, 29_800.0);
        let p2 = mp(3_000, 30_200.0);
        f.feed(0, MarketSample::Liquidation(&l));
        f.feed(0, MarketSample::MarkPrice(&p1));
        f.feed(0, MarketSample::MarkPrice(&p2));
        assert_eq!(f.primary() as i8, 1, "expected bullish stop hunt");
    }
}
