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

use crate::bar_indicators::agg_trade_consumer::AggTradeConsumer;
use crate::bar_indicators::funding_rate_consumer::FundingRateConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::long_short_ratio_consumer::LongShortRatioConsumer;
use crate::core::types::{AggTrade, FundingRate, LongShortRatio};

/// Composite sentiment indicator.
///
/// Implements `LongShortRatioConsumer`, `AggTradeConsumer`, and `FundingRateConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Clone)]
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

    /// Passthrough for bar pipeline — returns current value.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_composite)
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

impl LongShortRatioConsumer for SentimentComposite {
    fn update_long_short_ratio(&mut self, lsr: &LongShortRatio) -> IndicatorValue {
        self.l_s_norm = ((lsr.long_ratio - 0.5) * 2.0).clamp(-1.0, 1.0);
        self.recompute();
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

impl AggTradeConsumer for SentimentComposite {
    fn update_agg_trade(&mut self, t: &AggTrade) -> IndicatorValue {
        let cutoff = t.timestamp - self.window_ms;
        while self.agg_events.front().map_or(false, |(ts, _, _)| *ts < cutoff) {
            self.agg_events.pop_front();
        }
        let qty = t.price * t.quantity;
        self.agg_events.push_back((t.timestamp, qty, !t.is_buy)); // is_buy in AggTrade: false = buyer is taker
        self.recompute();
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

impl FundingRateConsumer for SentimentComposite {
    fn update_funding(&mut self, fr: &FundingRate) -> IndicatorValue {
        self.funding_norm = (fr.rate * 1000.0).clamp(-1.0, 1.0);
        self.recompute();
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

    fn make_lsr(long_ratio: f64) -> LongShortRatio {
        LongShortRatio {
            ratio_type: "global_account".to_string(),
            long_ratio,
            short_ratio: 1.0 - long_ratio,
            ratio: if long_ratio > 0.0 { Some(long_ratio / (1.0 - long_ratio).max(1e-9)) } else { None },
            timestamp: 1000,
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
        }
    }

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { symbol: "BTCUSDT".to_string(), rate, next_funding_time: None, timestamp: 1000 }
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
        if let IndicatorValue::Single(v) = ind.indicator_value() {
            assert!(v > 0.0, "composite={v}");
            assert!(v <= 1.0, "composite out of range: {v}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn balanced_lsr_gives_near_zero() {
        let mut ind = SentimentComposite::new(60_000);
        ind.update_long_short_ratio(&make_lsr(0.5)); // exactly balanced → l_s_norm = 0
        if let IndicatorValue::Single(v) = ind.indicator_value() {
            // only l_s_norm matters here, flow and funding are 0
            assert!(v.abs() < 1e-9, "composite={v}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = SentimentComposite::default();
        ind.update_long_short_ratio(&make_lsr(0.8));
        ind.update_funding(&make_fr(0.001));
        ind.indicator_reset();
        if let IndicatorValue::Single(v) = ind.indicator_value() {
            assert_eq!(v, 0.0);
        } else {
            panic!("expected Single");
        }
        assert!(!ind.indicator_is_ready());
    }
}
