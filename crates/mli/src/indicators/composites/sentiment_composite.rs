//! SentimentComposite — composite sentiment score from L/S ratio, trade flow, and funding.
//!
//! Triple consumer: `LongShortRatioConsumer` + `AggTradeConsumer` + `FundingRateConsumer`.
//!
//! Formula:
//! - `l_s_norm`        = (long_ratio - 0.5) × 2  ∈ [-1, 1]
//! - `flow_imb_norm`   = clamp(buy_vol / total_vol × 2 - 1, -1, 1)
//! - `funding_norm`    = clamp(funding × 1000, -1, 1)
//! - `composite`       = (l_s_norm + flow_imb_norm + funding_norm) / 3.0
//!
//! Output: `Single(composite)` ∈ [-1, 1].

use std::collections::VecDeque;

use crate::engine::streams::agg_trade_consumer::AggTradeConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::long_short_ratio_consumer::LongShortRatioConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::core::types::{AggTrade, FundingRate, LongShortRatio};
use crate::engine::stream_kind::StreamKind;

/// Composite sentiment indicator.
///
/// Implements `LongShortRatioConsumer`, `AggTradeConsumer`, and `FundingRateConsumer`.
/// Inherent methods used by contract dispatch to avoid UFCS ambiguity.
#[derive(Debug, Clone)]
pub struct SentimentComposite {
    window_ms: i64,
    l_s_norm: f64,
    funding_norm: f64,
    agg_events: VecDeque<(i64, f64, bool)>, // (ts, quote_qty, is_buy)
    last_composite: f64,
}

impl SentimentComposite {
    /// Create a new indicator.
    ///
    /// - `window_ms` — rolling window for agg trade flow (default 60_000)
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms,
            l_s_norm: 0.0,
            funding_norm: 0.0,
            agg_events: VecDeque::new(),
            last_composite: 0.0,
        }
    }

    fn flow_imbalance(&self) -> f64 {
        if self.agg_events.is_empty() {
            return 0.0;
        }
        let mut buy_vol = 0.0_f64;
        let mut total_vol = 0.0_f64;
        for &(_, qty, is_buy) in &self.agg_events {
            total_vol += qty;
            if is_buy {
                buy_vol += qty;
            }
        }
        if total_vol < 1e-12 {
            return 0.0;
        }
        (buy_vol / total_vol * 2.0 - 1.0).clamp(-1.0, 1.0)
    }

    fn recompute(&mut self) {
        let flow_imb = self.flow_imbalance();
        self.last_composite = (self.l_s_norm + flow_imb + self.funding_norm) / 3.0;
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> f64 {
        self.last_composite
    }

    /// True when at least one stream has delivered data.
    pub fn indicator_is_ready(&self) -> bool {
        self.l_s_norm != 0.0 || !self.agg_events.is_empty() || self.funding_norm != 0.0
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.l_s_norm = 0.0;
        self.funding_norm = 0.0;
        self.agg_events.clear();
        self.last_composite = 0.0;
    }
}

impl Default for SentimentComposite {
    fn default() -> Self {
        Self::new(60_000)
    }
}

/// Typed configuration for [`SentimentComposite`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SentimentCompositeConfig {
    /// Rolling window for agg trade flow in milliseconds (default 60_000).
    pub window_ms: Param<i64>,
}

impl Indicator for SentimentComposite {
    const ID: IndicatorId = IndicatorId::SentimentComposite;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[
        StreamKind::LongShortRatio,
        StreamKind::AggTrade,
        StreamKind::Funding,
    ];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::SentimentComposite)];
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = SentimentCompositeConfig;
    type Runtime = SentimentComposite;

    fn create(cfg: SentimentCompositeConfig) -> SentimentComposite {
        SentimentComposite::new(cfg.window_ms.resolved())
    }
}

impl crate::contract::Config for SentimentCompositeConfig {
    fn defaults() -> Self {
        SentimentCompositeConfig { window_ms: Param::Solo(60_000) }
    }
    fn machine_defaults() -> Self {
        // window_ms (i64): Class M raw-ms — canonical named durations (§3.8)
        let mut s = Self::machine_defaults_auto();
        s.window_ms = Param::many(vec![1000i64, 5000, 10000, 30000, 60000, 300000, 3600000]);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for SentimentComposite {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::SentimentComposite, "Sentiment", Color::hex(0xFF9800))
            .bounds(-1.0, 1.0)
            .precision(4)
            .build()
    }
}

impl LongShortRatioConsumer for SentimentComposite {
    fn update_long_short_ratio(&mut self, lsr: &LongShortRatio) {
        self.l_s_norm = ((lsr.long_ratio - 0.5) * 2.0).clamp(-1.0, 1.0);
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl AggTradeConsumer for SentimentComposite {
    fn update_agg_trade(&mut self, t: &AggTrade) {
        let cutoff = t.timestamp - self.window_ms;
        while self.agg_events.front().map_or(false, |(ts, _, _)| *ts < cutoff) {
            self.agg_events.pop_front();
        }
        let qty = t.price * t.quantity;
        self.agg_events.push_back((t.timestamp, qty, !t.is_buy)); // is_buy in AggTrade: false = buyer is taker
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl FundingRateConsumer for SentimentComposite {
    fn update_funding(&mut self, fr: &FundingRate) {
        self.funding_norm = (fr.rate * 1000.0).clamp(-1.0, 1.0);
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

    fn make_lsr(long_ratio: f64) -> LongShortRatio {
        LongShortRatio {
            symbol: String::new(),
            ratio_type: "global_account".to_string(),
            long_ratio,
            short_ratio: 1.0 - long_ratio,
            ratio: if long_ratio > 0.0 { Some(long_ratio / (1.0 - long_ratio).max(1e-9)) } else { None },
            timestamp: 1000,
            ..Default::default()
        }
    }

    fn make_agg(price: f64, qty: f64, is_buy: bool, ts: i64) -> AggTrade {
        AggTrade {
            aggregate_id: 1,
            price,
            quantity: qty,
            first_trade_id: 1,
            last_trade_id: 1,
            is_buy,
            timestamp: ts,
            ..Default::default()
        }
    }

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { rate, next_funding_time: None, timestamp: 1000, ..Default::default()}
    }

    #[test]
    fn all_bullish_gives_positive_composite() {
        let mut ind = SentimentComposite::new(60_000);
        // long_ratio = 0.7 → l_s_norm = +0.4
        ind.update_long_short_ratio(&make_lsr(0.7));
        // all buys → flow_imb = +1
        ind.update_agg_trade(&make_agg(100.0, 1.0, false, 1000)); // is_buy=false → buyer=maker → buy aggressor → !is_buy=true
        // positive funding
        ind.update_funding(&make_fr(0.0005));
        let v = ind.indicator_value();
        assert!(v > 0.0, "composite={v}");
        assert!(v <= 1.0, "composite out of range: {v}");
    }

    #[test]
    fn balanced_lsr_gives_near_zero() {
        let mut ind = SentimentComposite::new(60_000);
        ind.update_long_short_ratio(&make_lsr(0.5)); // exactly balanced → l_s_norm = 0
        let v = ind.indicator_value();
        // only l_s_norm matters here, flow and funding are 0
        assert!(v.abs() < 1e-9, "composite={v}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = SentimentComposite::default();
        ind.update_long_short_ratio(&make_lsr(0.8));
        ind.update_funding(&make_fr(0.001));
        ind.indicator_reset();
        let v = ind.indicator_value();
        assert_eq!(v, 0.0);
        assert!(!ind.indicator_is_ready());
    }
}
