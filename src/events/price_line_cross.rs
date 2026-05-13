//! Price × Line crossover with touch mode classification.
//!
//! Line = `Box<IndicatorInstance>` or `f64` constant.
//! Touch behaviour is selected via `TouchMode`.
//!
//! Output: `IndicatorValue::Triple(line_value, close, signal)`.
//! Signal:
//! - `+1.0` Bullish (close above / wick reject up / hammer above support)
//! - `-1.0` Bearish
//! - `0.0`  No event

use std::fmt;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::instance_factory::IndicatorInstance;
use crate::events::candle_pattern::CandlePatternKind;

/// Source operand for `PriceLineCross`.
pub enum LineSource {
    /// Any indicator whose `.value().main()` produces a scalar level.
    Indicator(Box<IndicatorInstance>),
    /// Fixed horizontal level.
    Constant(f64),
}

impl Clone for LineSource {
    fn clone(&self) -> Self {
        match self {
            LineSource::Indicator(b) => LineSource::Indicator(b.clone()),
            LineSource::Constant(k) => LineSource::Constant(*k),
        }
    }
}

impl fmt::Debug for LineSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LineSource::Indicator(_) => write!(f, "LineSource::Indicator(...)"),
            LineSource::Constant(k) => write!(f, "LineSource::Constant({k})"),
        }
    }
}

/// How price interacts with the line to trigger a signal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchMode {
    /// Close transitions from below to above the line → +1 bullish.
    CloseAbove,
    /// Close transitions from above to below the line → -1 bearish.
    CloseBelow,
    /// Any wick (high or low) crosses through the line.
    /// +1 if high crosses above, -1 if low crosses below.
    WickThrough,
    /// Wick crosses line but close returns to the same side as the previous bar.
    /// Bullish reject (+1): low < line && close > line (wick swept below, closed back above).
    /// Bearish reject (-1): high > line && close < line (wick swept above, closed back below).
    WickReject,
    /// High or low comes within `tolerance` of the line (absolute price distance).
    Touch {
        /// Maximum distance between bar extreme and line to fire.
        tolerance: f64,
    },
    /// Close-above/below crossover AND the bar also matches a candle pattern.
    WithCandle(CandlePatternKind),
}

/// Price × Line crossover detector with configurable touch semantics.
#[derive(Clone)]
pub struct PriceLineCross {
    line: LineSource,
    mode: TouchMode,
    /// Whether close was above line on the previous bar (used by CloseAbove/CloseBelow/WickReject).
    prev_close_above: Option<bool>,
    last_signal: i8,
    /// Stateful candle pattern detector used when mode == WithCandle.
    candle_detector: Option<crate::events::candle_pattern::CandlePatternDetector>,
}

impl fmt::Debug for PriceLineCross {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PriceLineCross")
            .field("line", &self.line)
            .field("mode", &self.mode)
            .field("prev_close_above", &self.prev_close_above)
            .field("last_signal", &self.last_signal)
            .finish()
    }
}

impl PriceLineCross {
    /// Construct with given line source and touch mode.
    pub fn new(line: LineSource, mode: TouchMode) -> Self {
        let candle_detector = if let TouchMode::WithCandle(kind) = mode {
            Some(crate::events::candle_pattern::CandlePatternDetector::new(kind))
        } else {
            None
        };
        Self {
            line,
            mode,
            prev_close_above: None,
            last_signal: 0,
            candle_detector,
        }
    }

    /// Feed one bar. Returns `Triple(line_val, close, signal)`.
    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> IndicatorValue {
        let line_val = match &mut self.line {
            LineSource::Indicator(b) => b.update_bar(open, high, low, close, volume, None).main(),
            LineSource::Constant(k) => *k,
        };

        let line_ready = match &self.line {
            LineSource::Indicator(b) => b.is_ready(),
            LineSource::Constant(_) => true,
        };

        let signal: i8 = if !line_ready {
            0
        } else {
            match self.mode {
                TouchMode::CloseAbove => {
                    match self.prev_close_above {
                        Some(false) if close > line_val => 1,
                        _ => 0,
                    }
                }
                TouchMode::CloseBelow => {
                    match self.prev_close_above {
                        Some(true) if close < line_val => -1,
                        _ => 0,
                    }
                }
                TouchMode::WickThrough => {
                    // Both wicks can cross simultaneously — prefer bullish if both.
                    if low <= line_val && high >= line_val {
                        // Bar straddles line — pick direction by close.
                        if close >= line_val { 1 } else { -1 }
                    } else if high > line_val && matches!(self.prev_close_above, Some(false)) {
                        1
                    } else if low < line_val && matches!(self.prev_close_above, Some(true)) {
                        -1
                    } else {
                        0
                    }
                }
                TouchMode::WickReject => {
                    // Bullish SFP: low swept below line, close back above (prev was above).
                    let bullish = low < line_val
                        && close > line_val
                        && matches!(self.prev_close_above, Some(true) | None);
                    // Bearish SFP: high swept above line, close back below (prev was below).
                    let bearish = high > line_val
                        && close < line_val
                        && matches!(self.prev_close_above, Some(false) | None);
                    if bullish { 1 } else if bearish { -1 } else { 0 }
                }
                TouchMode::Touch { tolerance } => {
                    let near_high = (high - line_val).abs() <= tolerance;
                    let near_low = (low - line_val).abs() <= tolerance;
                    if near_high || near_low { 1 } else { 0 }
                }
                TouchMode::WithCandle(_) => {
                    // Cross direction: CloseAbove semantics.
                    let crossed = match self.prev_close_above {
                        Some(false) if close > line_val => true,
                        Some(true) if close < line_val => true,
                        _ => false,
                    };
                    let pattern_match = if let Some(ref mut det) = self.candle_detector {
                        det.detect_from_values(open, high, low, close).is_some()
                    } else {
                        false
                    };
                    if crossed && pattern_match {
                        if close > line_val { 1 } else { -1 }
                    } else {
                        0
                    }
                }
            }
        };

        self.prev_close_above = Some(close > line_val);
        self.last_signal = signal;
        IndicatorValue::Triple(line_val, close, signal as f64)
    }

    /// Last computed value.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(0.0, 0.0, self.last_signal as f64)
    }

    /// True once the inner line indicator (if any) has warmed up.
    pub fn is_ready(&self) -> bool {
        match &self.line {
            LineSource::Indicator(b) => b.is_ready(),
            LineSource::Constant(_) => self.prev_close_above.is_some(),
        }
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        if let LineSource::Indicator(b) = &mut self.line {
            b.reset();
        }
        self.prev_close_above = None;
        self.last_signal = 0;
        if let Some(ref mut det) = self.candle_detector {
            det.reset();
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::bar_indicators::bar_indicator_id::BarIndicatorId;
    use crate::bar_indicators::instance_factory::{IndicatorConfig, IndicatorInstance};

    fn make_sma(period: usize) -> IndicatorInstance {
        let cfg = IndicatorConfig::new(BarIndicatorId::Sma, "Sma".into(), vec![period]);
        IndicatorInstance::create(&cfg).expect("SMA factory")
    }

    fn signal_of(v: IndicatorValue) -> f64 {
        match v {
            IndicatorValue::Triple(_, _, s) => s,
            _ => panic!("expected Triple"),
        }
    }

    fn feed(plc: &mut PriceLineCross, prices: &[f64]) {
        for &p in prices {
            plc.update_bar(p, p, p, p, 0.0);
        }
    }

    // ---- CloseAbove ----

    #[test]
    fn close_above_fires_on_transition() {
        let mut plc = PriceLineCross::new(LineSource::Constant(100.0), TouchMode::CloseAbove);
        feed(&mut plc, &[90.0, 90.0, 90.0]);
        // First bar that closes above 100.
        let v = plc.update_bar(100.0, 105.0, 99.0, 105.0, 0.0);
        assert_eq!(signal_of(v), 1.0, "CloseAbove must fire +1 on up-transition");
        // Stay above — no further signal.
        let v2 = plc.update_bar(105.0, 106.0, 104.0, 105.0, 0.0);
        assert_eq!(signal_of(v2), 0.0, "CloseAbove must not repeat while above");
    }

    #[test]
    fn close_above_no_signal_when_already_above() {
        let mut plc = PriceLineCross::new(LineSource::Constant(100.0), TouchMode::CloseAbove);
        feed(&mut plc, &[110.0; 5]);
        let v = plc.update_bar(110.0, 111.0, 109.0, 110.0, 0.0);
        assert_eq!(signal_of(v), 0.0, "CloseAbove: no signal when always above");
    }

    // ---- CloseBelow ----

    #[test]
    fn close_below_fires_on_transition() {
        let mut plc = PriceLineCross::new(LineSource::Constant(100.0), TouchMode::CloseBelow);
        feed(&mut plc, &[110.0, 110.0, 110.0]);
        let v = plc.update_bar(100.0, 101.0, 95.0, 95.0, 0.0);
        assert_eq!(signal_of(v), -1.0, "CloseBelow must fire -1 on down-transition");
        let v2 = plc.update_bar(95.0, 96.0, 94.0, 95.0, 0.0);
        assert_eq!(signal_of(v2), 0.0, "CloseBelow must not repeat while below");
    }

    // ---- WickThrough ----

    #[test]
    fn wick_through_high_fires() {
        let mut plc = PriceLineCross::new(LineSource::Constant(100.0), TouchMode::WickThrough);
        feed(&mut plc, &[90.0; 3]);
        // High pierces 100 but close stays below — wick through.
        let v = plc.update_bar(90.0, 110.0, 89.0, 91.0, 0.0);
        assert_eq!(signal_of(v), -1.0, "bar below with high spike: straddle resolves bearish (close < line)");
    }

    #[test]
    fn wick_through_low_fires() {
        let mut plc = PriceLineCross::new(LineSource::Constant(100.0), TouchMode::WickThrough);
        feed(&mut plc, &[110.0; 3]);
        // Low dips below 100 but close stays above — wick through.
        let v = plc.update_bar(110.0, 111.0, 95.0, 109.0, 0.0);
        assert_eq!(signal_of(v), 1.0, "bar above with low spike: straddle resolves bullish (close > line)");
    }

    // ---- WickReject ----

    #[test]
    fn wick_reject_bullish() {
        // Bullish SFP: prev close above, low sweeps below line, close back above.
        let mut plc = PriceLineCross::new(LineSource::Constant(100.0), TouchMode::WickReject);
        feed(&mut plc, &[110.0; 3]); // prev_close_above = Some(true)
        let v = plc.update_bar(108.0, 112.0, 95.0, 105.0, 0.0);
        assert_eq!(signal_of(v), 1.0, "bullish wick reject: low below line, close above");
    }

    #[test]
    fn wick_reject_bearish() {
        // Bearish SFP: prev close below, high sweeps above line, close back below.
        let mut plc = PriceLineCross::new(LineSource::Constant(100.0), TouchMode::WickReject);
        feed(&mut plc, &[90.0; 3]); // prev_close_above = Some(false)
        let v = plc.update_bar(92.0, 115.0, 91.0, 95.0, 0.0);
        assert_eq!(signal_of(v), -1.0, "bearish wick reject: high above line, close below");
    }

    // ---- Touch ----

    #[test]
    fn touch_fires_within_tolerance() {
        let mut plc = PriceLineCross::new(
            LineSource::Constant(100.0),
            TouchMode::Touch { tolerance: 2.0 },
        );
        // High is 101.5 — within 2.0 of 100.0.
        let v = plc.update_bar(98.0, 101.5, 97.0, 98.5, 0.0);
        assert_eq!(signal_of(v), 1.0, "Touch: bar within tolerance must fire");
    }

    #[test]
    fn touch_no_fire_outside_tolerance() {
        let mut plc = PriceLineCross::new(
            LineSource::Constant(100.0),
            TouchMode::Touch { tolerance: 1.0 },
        );
        // High is 97, low is 93 — both further than 1.0 from 100.
        let v = plc.update_bar(95.0, 97.0, 93.0, 94.0, 0.0);
        assert_eq!(signal_of(v), 0.0, "Touch: bar outside tolerance must not fire");
    }

    // ---- WithCandle ----

    #[test]
    fn with_candle_hammer_requires_cross() {
        let mut plc = PriceLineCross::new(
            LineSource::Constant(100.0),
            TouchMode::WithCandle(CandlePatternKind::Hammer),
        );
        // Set up: price was below 100.
        feed(&mut plc, &[90.0; 3]);
        // Hammer-shaped bar that crosses above 100:
        // open=99, close=100.5 (above), low=90 (big lower wick), high=100.6.
        let v = plc.update_bar(99.0, 100.6, 90.0, 100.5, 0.0);
        // Hammer: lower_wick=9.5, body=1.5, upper_wick=0.1 → qualifies.
        // Crossed: prev below, now above.
        assert_eq!(signal_of(v), 1.0, "WithCandle(Hammer): must fire +1 when hammer + cross");
    }

    #[test]
    fn with_candle_no_pattern_no_fire() {
        let mut plc = PriceLineCross::new(
            LineSource::Constant(100.0),
            TouchMode::WithCandle(CandlePatternKind::Hammer),
        );
        // Price crosses above but bar is not a hammer (large body, no lower wick).
        feed(&mut plc, &[90.0; 3]);
        // open=90, close=110 — full-body candle, not a hammer.
        let v = plc.update_bar(90.0, 110.0, 89.0, 110.0, 0.0);
        assert_eq!(signal_of(v), 0.0, "WithCandle: must not fire without pattern match");
    }

    // ---- with SMA line ----

    #[test]
    fn with_indicator_line_close_above() {
        let mut plc = PriceLineCross::new(
            LineSource::Indicator(Box::new(make_sma(5))),
            TouchMode::CloseAbove,
        );
        // Warm up SMA below 100.
        for _ in 0..10 {
            plc.update_bar(95.0, 95.0, 95.0, 95.0, 0.0);
        }
        // Jump well above — must produce CloseAbove signal at some point.
        let mut fired = false;
        for _ in 0..10 {
            let v = plc.update_bar(120.0, 120.0, 120.0, 120.0, 0.0);
            if signal_of(v) > 0.0 { fired = true; }
        }
        assert!(fired, "CloseAbove with SMA line: must fire when price surges above SMA");
    }

    // ---- Reset ----

    #[test]
    fn reset_clears_state() {
        let mut plc = PriceLineCross::new(LineSource::Constant(100.0), TouchMode::CloseAbove);
        feed(&mut plc, &[110.0; 5]);
        plc.reset();
        assert!(!plc.is_ready());
        assert_eq!(signal_of(plc.value()), 0.0);
    }
}
