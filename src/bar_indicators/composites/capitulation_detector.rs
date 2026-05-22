//! CapitulationDetector — detects capitulation events using liquidation spike,
//! volume spike, and price reversal patterns.
//!
//! Triple consumer: `LiquidationConsumer` + `AggTradeConsumer` + `MarkPriceConsumer`.
//!
//! Conditions:
//! 1. Liquidation rate spike: ≥ `liq_spike_count` events in `window_ms`
//! 2. Volume spike: rolling agg trade quote sum ≥ `volume_spike_threshold`
//! 3. Price reversal: strong move then partial bounce back
//!
//! Output: `Signal(i8)` — `+1` bullish capitulation, `-1` bearish capitulation, `0` none.

use std::collections::VecDeque;

use crate::bar_indicators::agg_trade_consumer::AggTradeConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::liquidation_consumer::LiquidationConsumer;
use crate::bar_indicators::mark_price_consumer::MarkPriceConsumer;
use crate::core::types::{AggTrade, Liquidation, TradeSide, MarkPrice};

/// Capitulation event detector.
///
/// Implements `LiquidationConsumer`, `AggTradeConsumer`, and `MarkPriceConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Clone)]
pub struct CapitulationDetector {
    window_ms: i64,
    liq_spike_count: usize,
    volume_spike_threshold: f64,

    // Rolling windows
    liq_long_events: VecDeque<i64>,
    liq_short_events: VecDeque<i64>,
    agg_events: VecDeque<(i64, f64)>,
    price_history: VecDeque<(i64, f64)>,

    last_signal: i8,
}

impl CapitulationDetector {
    /// Create a new indicator.
    ///
    /// - `liq_spike_count` — minimum liquidation count within window to qualify as spike.
    /// - `volume_spike_threshold` — minimum quote volume sum within window (e.g. `1_000_000.0`).
    /// - `window_ms` — rolling time window in milliseconds.
    pub fn new(liq_spike_count: usize, volume_spike_threshold: f64, window_ms: i64) -> Self {
        Self {
            window_ms,
            liq_spike_count,
            volume_spike_threshold,
            liq_long_events: VecDeque::new(),
            liq_short_events: VecDeque::new(),
            agg_events: VecDeque::new(),
            price_history: VecDeque::new(),
            last_signal: 0,
        }
    }

    fn evict_liq(&mut self, now: i64) {
        let cutoff = now - self.window_ms;
        while self.liq_long_events.front().map_or(false, |ts| *ts < cutoff) {
            self.liq_long_events.pop_front();
        }
        while self.liq_short_events.front().map_or(false, |ts| *ts < cutoff) {
            self.liq_short_events.pop_front();
        }
    }

    fn evict_agg(&mut self, now: i64) {
        let cutoff = now - self.window_ms;
        while self.agg_events.front().map_or(false, |(ts, _)| *ts < cutoff) {
            self.agg_events.pop_front();
        }
    }

    fn evict_price(&mut self, now: i64) {
        let cutoff = now - self.window_ms;
        while self.price_history.front().map_or(false, |(ts, _)| *ts < cutoff) {
            self.price_history.pop_front();
        }
    }

    fn recompute(&mut self) {
        // Check volume spike
        let vol_sum: f64 = self.agg_events.iter().map(|(_, v)| v).sum();
        let vol_spike = vol_sum >= self.volume_spike_threshold;

        // Check liq spikes by side
        let long_liq_spike = self.liq_long_events.len() >= self.liq_spike_count;
        let short_liq_spike = self.liq_short_events.len() >= self.liq_spike_count;

        // Check price reversal — need at least 3 points
        let (price_dropped, price_bounced, price_rose, price_fell) =
            if self.price_history.len() >= 3 {
                let oldest = self.price_history.front().map_or(0.0, |(_, p)| *p);
                let newest = self.price_history.back().map_or(0.0, |(_, p)| *p);
                // Find min and max in window
                let min_p = self.price_history.iter().map(|(_, p)| *p).fold(f64::INFINITY, f64::min);
                let max_p = self.price_history.iter().map(|(_, p)| *p).fold(f64::NEG_INFINITY, f64::max);

                // Bullish reversal: price made a low, then bounced back (newest > min_p significantly)
                let dropped = oldest > 0.0 && (min_p - oldest) / oldest < -0.01; // dropped >1%
                let bounced = newest > min_p * 1.005; // bounced at least 0.5% off low

                // Bearish reversal: price made a high, then fell back
                let rose = oldest > 0.0 && (max_p - oldest) / oldest > 0.01; // rose >1%
                let fell = newest < max_p * 0.995; // fell at least 0.5% off high

                (dropped, bounced, rose, fell)
            } else {
                (false, false, false, false)
            };

        // Bullish capitulation: long liq spike + vol spike + price dropped and bounced
        let bullish = long_liq_spike && vol_spike && price_dropped && price_bounced;
        // Bearish capitulation: short liq spike + vol spike + price rose and fell
        let bearish = short_liq_spike && vol_spike && price_rose && price_fell;

        self.last_signal = if bullish {
            1
        } else if bearish {
            -1
        } else {
            0
        };
    }

    /// Passthrough for bar pipeline — returns current value.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    /// True when all three streams have delivered at least one update.
    pub fn indicator_is_ready(&self) -> bool {
        (!self.liq_long_events.is_empty() || !self.liq_short_events.is_empty())
            && !self.agg_events.is_empty()
            && self.price_history.len() >= 3
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.liq_long_events.clear();
        self.liq_short_events.clear();
        self.agg_events.clear();
        self.price_history.clear();
        self.last_signal = 0;
    }
}

impl Default for CapitulationDetector {
    fn default() -> Self {
        Self::new(5, 1_000_000.0, 60_000)
    }
}

impl LiquidationConsumer for CapitulationDetector {
    fn update_liquidation(&mut self, liq: &Liquidation) -> IndicatorValue {
        self.evict_liq(liq.timestamp);
        match liq.side {
            TradeSide::Buy => self.liq_long_events.push_back(liq.timestamp),
            TradeSide::Sell => self.liq_short_events.push_back(liq.timestamp),
        }
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

impl AggTradeConsumer for CapitulationDetector {
    fn update_agg_trade(&mut self, t: &AggTrade) -> IndicatorValue {
        self.evict_agg(t.timestamp);
        let quote_vol = t.price * t.quantity;
        self.agg_events.push_back((t.timestamp, quote_vol));
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

impl MarkPriceConsumer for CapitulationDetector {
    fn update_mark(&mut self, mp: &MarkPrice) -> IndicatorValue {
        self.evict_price(mp.timestamp);
        self.price_history.push_back((mp.timestamp, mp.mark_price));
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

    fn make_liq_long(ts: i64) -> Liquidation {
        Liquidation { symbol: String::new(), side: TradeSide::Buy, price: 30000.0, quantity: 0.1, timestamp: ts, value: None }
    }

    fn make_agg(ts: i64, price: f64, qty: f64) -> AggTrade {
        AggTrade { aggregate_id: 1, price, quantity: qty, first_trade_id: 1, last_trade_id: 1, is_buy: true, timestamp: ts }
    }

    fn make_mp(ts: i64, mark_price: f64) -> MarkPrice {
        MarkPrice { mark_price, index_price: None, funding_rate: None, timestamp: ts }
    }

    #[test]
    fn bullish_capitulation_all_conditions_met() {
        // liq_spike_count=3, volume_threshold=10000, window=60000
        let mut ind = CapitulationDetector::new(3, 10_000.0, 60_000);

        // Liq spike (3 long liquidations)
        for i in 0..3 {
            ind.update_liquidation(&make_liq_long(1000 + i * 100));
        }
        // Volume spike
        for i in 0..5 {
            ind.update_agg_trade(&make_agg(1500 + i * 100, 30000.0, 1.0)); // 30000 each
        }
        // Price: dropped 2% then bounced 1%
        ind.update_mark(&make_mp(1000, 30000.0)); // start
        ind.update_mark(&make_mp(2000, 29400.0)); // -2%
        ind.update_mark(&make_mp(3000, 29550.0)); // bounce +0.5% off low

        if let IndicatorValue::Signal(s) = ind.indicator_value() {
            assert_eq!(s, 1, "expected bullish capitulation +1");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn no_signal_without_liq_spike() {
        let mut ind = CapitulationDetector::new(5, 10_000.0, 60_000);
        // Only 2 liquidations (need 5)
        ind.update_liquidation(&make_liq_long(1000));
        ind.update_liquidation(&make_liq_long(1100));
        for i in 0..5 {
            ind.update_agg_trade(&make_agg(1500 + i * 100, 30000.0, 1.0));
        }
        ind.update_mark(&make_mp(1000, 30000.0));
        ind.update_mark(&make_mp(2000, 29400.0));
        ind.update_mark(&make_mp(3000, 29550.0));
        if let IndicatorValue::Signal(s) = ind.indicator_value() {
            assert_eq!(s, 0);
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn reset_clears_signal() {
        let mut ind = CapitulationDetector::new(3, 10_000.0, 60_000);
        for i in 0..3 {
            ind.update_liquidation(&make_liq_long(1000 + i * 100));
        }
        for i in 0..5 {
            ind.update_agg_trade(&make_agg(1500 + i * 100, 30000.0, 1.0));
        }
        ind.update_mark(&make_mp(1000, 30000.0));
        ind.update_mark(&make_mp(2000, 29400.0));
        ind.update_mark(&make_mp(3000, 29550.0));
        ind.indicator_reset();
        if let IndicatorValue::Signal(s) = ind.indicator_value() {
            assert_eq!(s, 0);
        } else {
            panic!("expected Signal");
        }
    }
}
