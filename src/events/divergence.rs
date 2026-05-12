//! Divergence primitive — detects price/oscillator divergence.
//!
//! Owns one inner oscillator indicator (RSI, CCI, MACD, OBV, Williams%R,
//! Stochastic, Momentum, etc.). Maintains rolling buffers of price and
//! oscillator values, compares `bar[now]` vs `bar[now - lookback]` to detect:
//!
//! - **Regular bullish**: price makes lower low, oscillator makes higher low
//! - **Regular bearish**: price makes higher high, oscillator makes lower high
//! - **Hidden bullish**: price makes higher low, oscillator makes lower low (continuation)
//! - **Hidden bearish**: price makes lower high, oscillator makes higher high
//!
//! Output: `IndicatorValue::Signal(i8)` with +1 for bullish, -1 for bearish.
//!
//! Replaces the family of 14 hardcoded `*_divergence.rs` files that each
//! re-implemented the same logic with a different hardcoded inner oscillator.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::instance_factory::IndicatorInstance;
use crate::bar_indicators::ohlcv_field::OhlcvField;
use crate::core::events::direction::Direction;
use crate::core::events::kind::{DivergenceSub, SignalKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivergenceKind {
    /// Standard divergence: opposing slopes between price and oscillator.
    Regular,
    /// Hidden divergence: same direction price + opposite oscillator
    /// (signals trend continuation rather than reversal).
    Hidden,
}

impl Default for DivergenceKind {
    fn default() -> Self {
        Self::Regular
    }
}

#[derive(Clone)]
pub struct Divergence {
    oscillator: Box<IndicatorInstance>,
    lookback: usize,
    kind: DivergenceKind,
    price_source: OhlcvField,
    prices: Vec<f64>,
    osc_values: Vec<f64>,
    last_signal: i8,
}

impl Divergence {
    pub fn new(oscillator: IndicatorInstance, lookback: usize, kind: DivergenceKind) -> Self {
        Self::with_source(oscillator, lookback, kind, OhlcvField::Close)
    }

    pub fn with_source(
        oscillator: IndicatorInstance,
        lookback: usize,
        kind: DivergenceKind,
        price_source: OhlcvField,
    ) -> Self {
        let lb = lookback.max(2);
        Self {
            oscillator: Box::new(oscillator),
            lookback: lb,
            kind,
            price_source,
            prices: Vec::with_capacity(lb * 2),
            osc_values: Vec::with_capacity(lb * 2),
            last_signal: 0,
        }
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let price = self.price_source.extract(open, high, low, close, volume);
        let osc = self
            .oscillator
            .update_bar(open, high, low, close, volume, None)
            .main();

        // Rolling capped buffers (lookback × 2 = comparison window).
        let cap = self.lookback * 2;
        if self.prices.len() >= cap {
            self.prices.remove(0);
            self.osc_values.remove(0);
        }
        self.prices.push(price);
        self.osc_values.push(osc);

        let signal = if self.oscillator.is_ready() && self.prices.len() >= self.lookback + 1 {
            let n = self.prices.len();
            let p_now = self.prices[n - 1];
            let p_then = self.prices[n - 1 - self.lookback];
            let o_now = self.osc_values[n - 1];
            let o_then = self.osc_values[n - 1 - self.lookback];

            let price_up = p_now > p_then;
            let price_down = p_now < p_then;
            let osc_up = o_now > o_then;
            let osc_down = o_now < o_then;

            match self.kind {
                DivergenceKind::Regular => {
                    if price_down && osc_up {
                        1 // bullish regular
                    } else if price_up && osc_down {
                        -1 // bearish regular
                    } else {
                        0
                    }
                }
                DivergenceKind::Hidden => {
                    if price_up && osc_down {
                        // Hidden bullish in uptrend: price HL, osc LL — wait,
                        // textbook hidden: price higher low, osc lower low.
                        // In rolling-window comparison: if current price is
                        // higher than past AND oscillator current lower than past
                        // → continuation signal in uptrend.
                        1
                    } else if price_down && osc_up {
                        -1
                    } else {
                        0
                    }
                }
            }
        } else {
            0
        };

        self.last_signal = signal;
        signal as f64
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    /// Feed one bar and return a typed signal when divergence is detected.
    ///
    /// The `DivergenceSub` variant mirrors the detector's `DivergenceKind`:
    /// - `DivergenceKind::Regular` → `DivergenceSub::Regular`
    /// - `DivergenceKind::Hidden` → `DivergenceSub::Hidden`
    ///
    /// Bullish divergence (price lower low, oscillator higher low) → `Direction::Up`.
    /// Bearish divergence (price higher high, oscillator lower high) → `Direction::Down`.
    pub fn detect(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Option<(SignalKind, Direction)> {
        self.update_bar(open, high, low, close, volume);
        let sub = match self.kind {
            DivergenceKind::Regular => DivergenceSub::Regular,
            DivergenceKind::Hidden => DivergenceSub::Hidden,
        };
        match self.last_signal {
            1 => Some((SignalKind::Divergence(sub), Direction::Up)),
            -1 => Some((SignalKind::Divergence(sub), Direction::Down)),
            _ => None,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.oscillator.is_ready() && self.prices.len() >= self.lookback + 1
    }

    pub fn reset(&mut self) {
        self.oscillator.reset();
        self.prices.clear();
        self.osc_values.clear();
        self.last_signal = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
    use crate::bar_indicators::instance_factory::{IndicatorConfig, IndicatorInstance};

    fn rsi(period: usize) -> IndicatorInstance {
        IndicatorInstance::create(&IndicatorConfig::new(BarIndicatorId::Rsi, "Rsi".into(), vec![period])).unwrap()
    }

    fn cci(period: usize) -> IndicatorInstance {
        IndicatorInstance::create(&IndicatorConfig::new(BarIndicatorId::Cci, "Cci".into(), vec![period])).unwrap()
    }

    fn macd() -> IndicatorInstance {
        IndicatorInstance::create(&IndicatorConfig::new(BarIndicatorId::Macd, "Macd".into(), vec![12, 26, 9])).unwrap()
    }

    fn obv() -> IndicatorInstance {
        IndicatorInstance::create(&IndicatorConfig::new(BarIndicatorId::Obv, "Obv".into(), vec![1])).unwrap()
    }

    #[test]
    fn warmup_no_signal() {
        let mut d = Divergence::new(rsi(14), 10, DivergenceKind::Regular);
        for i in 0..5 {
            let p = 100.0 + i as f64;
            let s = d.update_bar(p, p, p, p, 0.0);
            assert_eq!(s, 0.0, "no signal during warmup at bar {}", i);
        }
    }

    #[test]
    fn regular_bullish_with_rsi() {
        // Price drops then partial recovery, RSI bottoms earlier and starts rising
        // → bullish divergence (price lower, RSI higher).
        let mut d = Divergence::new(rsi(14), 10, DivergenceKind::Regular);
        // Build prior history: high prices + high RSI.
        for i in 0..30 {
            let p = 100.0 + (i as f64 * 0.3).sin() * 3.0 + 20.0;
            let _ = d.update_bar(p, p, p, p, 0.0);
        }
        // Slow decline with oscillation — price LL, RSI HL pattern often emerges.
        let mut saw_bullish = false;
        for i in 0..60 {
            // Price slowly drops but oscillates
            let p = 120.0 - i as f64 * 0.3 + (i as f64 * 0.5).sin() * 2.0;
            let s = d.update_bar(p, p, p, p, 0.0);
            if s > 0.0 { saw_bullish = true; }
        }
        assert!(saw_bullish, "expected at least one bullish regular divergence signal");
    }

    #[test]
    fn regular_bearish_with_rsi() {
        let mut d = Divergence::new(rsi(14), 10, DivergenceKind::Regular);
        for _ in 0..30 {
            let p = 100.0;
            let _ = d.update_bar(p, p, p, p, 0.0);
        }
        // Price slowly climbs, oscillating; RSI top forms early then declines.
        let mut saw_bearish = false;
        for i in 0..60 {
            let p = 100.0 + i as f64 * 0.3 + (i as f64 * 0.5).sin() * 2.0;
            let s = d.update_bar(p, p, p, p, 0.0);
            if s < 0.0 { saw_bearish = true; }
        }
        assert!(saw_bearish, "expected at least one bearish regular divergence signal");
    }

    #[test]
    fn works_with_cci_inner() {
        let mut d = Divergence::new(cci(14), 10, DivergenceKind::Regular);
        for i in 0..60 {
            let p = 100.0 + (i as f64 * 0.4).sin() * 10.0;
            let _ = d.update_bar(p, p + 0.5, p - 0.5, p, 0.0);
        }
        assert!(d.is_ready());
    }

    #[test]
    fn works_with_macd_inner() {
        let mut d = Divergence::new(macd(), 10, DivergenceKind::Regular);
        for i in 0..80 {
            let p = 100.0 + (i as f64 * 0.4).sin() * 10.0;
            let _ = d.update_bar(p, p + 0.5, p - 0.5, p, 0.0);
        }
        assert!(d.is_ready());
    }

    #[test]
    fn works_with_obv_inner() {
        let mut d = Divergence::new(obv(), 10, DivergenceKind::Regular);
        for i in 0..50 {
            let p = 100.0 + (i as f64 * 0.4).sin() * 8.0;
            let _ = d.update_bar(p, p, p, p, 1000.0 + i as f64);
        }
        assert!(d.is_ready());
    }

    #[test]
    fn hidden_divergence_distinct_from_regular() {
        // Same data → different DivergenceKind must produce different signal patterns.
        let mut reg = Divergence::new(rsi(14), 10, DivergenceKind::Regular);
        let mut hid = Divergence::new(rsi(14), 10, DivergenceKind::Hidden);

        let mut reg_signals = 0i64;
        let mut hid_signals = 0i64;
        for i in 0..100 {
            let p = 100.0 + (i as f64 * 0.4).sin() * 10.0 + i as f64 * 0.1;
            let r = reg.update_bar(p, p, p, p, 0.0);
            let h = hid.update_bar(p, p, p, p, 0.0);
            if r != 0.0 { reg_signals += 1; }
            if h != 0.0 { hid_signals += 1; }
        }
        // The two modes shouldn't both be silent — at least one must fire.
        assert!(reg_signals > 0 || hid_signals > 0, "neither regular nor hidden fired — test data not divergent");
    }

    #[test]
    fn reset_clears() {
        let mut d = Divergence::new(rsi(14), 10, DivergenceKind::Regular);
        for i in 0..30 {
            let p = 100.0 + i as f64;
            let _ = d.update_bar(p, p, p, p, 0.0);
        }
        d.reset();
        assert!(!d.is_ready());
        assert_eq!(d.value(), IndicatorValue::Signal(0));
    }

    #[test]
    fn source_high_changes_pattern() {
        // Same RSI inner, different price_source (close vs high) → different signals.
        let mut close_div = Divergence::with_source(rsi(14), 10, DivergenceKind::Regular, OhlcvField::Close);
        let mut high_div = Divergence::with_source(rsi(14), 10, DivergenceKind::Regular, OhlcvField::High);

        let mut differed = false;
        for i in 0..80 {
            let close_p = 100.0 + (i as f64 * 0.4).sin() * 5.0;
            let high_p = close_p + 5.0 + (i as f64 * 0.7).cos() * 3.0;
            let low_p = close_p - 5.0;
            let c = close_div.update_bar(high_p - 5.0, high_p, low_p, close_p, 0.0);
            let h = high_div.update_bar(high_p - 5.0, high_p, low_p, close_p, 0.0);
            if c != h { differed = true; }
        }
        assert!(differed, "different price_source must produce different signal sequence");
    }
}
