//! Line × Line crossover detector.
//!
//! Both operands can be:
//! - `Box<IndicatorInstance>` — any indicator producing a scalar
//! - `f64` constant — fixed horizontal level
//!
//! Detects when operand A crosses operand B (either direction).
//! Mode controls signal stickiness:
//! - `Momentary` — non-zero only on crossover bar
//! - `Sticky` — holds direction sign until next crossover

use std::fmt;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::instance_factory::IndicatorInstance;

/// Source operand for `LineCross`.
pub enum LineSource {
    /// Any indicator whose `.value().main()` produces a scalar.
    Indicator(Box<IndicatorInstance>),
    /// Fixed horizontal level (e.g., 0.0, 50.0, a Fibo ratio).
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

/// Signal stickiness mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossMode {
    /// Non-zero only on the crossover bar itself.
    Momentary,
    /// Holds the last crossover sign (+1 or -1) until the next crossover.
    Sticky,
}

/// Two-line crossover detector.
///
/// Output: `IndicatorValue::Triple(left_value, right_value, signal_as_f64)`.
/// Signal: `+1.0` = left crossed above right, `-1.0` = left crossed below right, `0.0` = no event.
#[derive(Clone)]
pub struct LineCross {
    left: LineSource,
    right: LineSource,
    mode: CrossMode,
    prev_left: f64,
    prev_right: f64,
    has_prev: bool,
    last_sticky: i8,
}

impl fmt::Debug for LineCross {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LineCross")
            .field("left", &self.left)
            .field("right", &self.right)
            .field("mode", &self.mode)
            .field("prev_left", &self.prev_left)
            .field("prev_right", &self.prev_right)
            .field("has_prev", &self.has_prev)
            .field("last_sticky", &self.last_sticky)
            .finish()
    }
}

impl LineCross {
    /// Construct with explicit operands and mode.
    pub fn new(left: LineSource, right: LineSource, mode: CrossMode) -> Self {
        Self {
            left,
            right,
            mode,
            prev_left: 0.0,
            prev_right: 0.0,
            has_prev: false,
            last_sticky: 0,
        }
    }

    /// Feed one bar. Returns `Triple(left_val, right_val, signal)`.
    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> IndicatorValue {
        let a = match &mut self.left {
            LineSource::Indicator(b) => b.update_bar(open, high, low, close, volume, None).main(),
            LineSource::Constant(k) => *k,
        };
        let b = match &mut self.right {
            LineSource::Indicator(b) => b.update_bar(open, high, low, close, volume, None).main(),
            LineSource::Constant(k) => *k,
        };

        let crossover_signal: i8 = if self.has_prev {
            if self.prev_left <= self.prev_right && a > b {
                1
            } else if self.prev_left >= self.prev_right && a < b {
                -1
            } else {
                0
            }
        } else {
            0
        };

        self.prev_left = a;
        self.prev_right = b;
        self.has_prev = true;

        let out = match self.mode {
            CrossMode::Momentary => crossover_signal,
            CrossMode::Sticky => {
                if crossover_signal != 0 {
                    self.last_sticky = crossover_signal;
                }
                self.last_sticky
            }
        };

        IndicatorValue::Triple(a, b, out as f64)
    }

    /// Last computed value.
    pub fn value(&self) -> IndicatorValue {
        let signal = match self.mode {
            CrossMode::Momentary => 0i8,
            CrossMode::Sticky => self.last_sticky,
        };
        IndicatorValue::Triple(self.prev_left, self.prev_right, signal as f64)
    }

    /// True once both operands (if indicator-based) have warmed up.
    pub fn is_ready(&self) -> bool {
        let left_ready = match &self.left {
            LineSource::Indicator(b) => b.is_ready(),
            LineSource::Constant(_) => true,
        };
        let right_ready = match &self.right {
            LineSource::Indicator(b) => b.is_ready(),
            LineSource::Constant(_) => true,
        };
        self.has_prev && left_ready && right_ready
    }

    /// Reset all state.
    pub fn reset(&mut self) {
        if let LineSource::Indicator(b) = &mut self.left {
            b.reset();
        }
        if let LineSource::Indicator(b) = &mut self.right {
            b.reset();
        }
        self.prev_left = 0.0;
        self.prev_right = 0.0;
        self.has_prev = false;
        self.last_sticky = 0;
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

    fn feed(lc: &mut LineCross, prices: &[f64]) {
        for &p in prices {
            lc.update_bar(p, p, p, p, 0.0);
        }
    }

    fn signal_of(v: IndicatorValue) -> f64 {
        match v {
            IndicatorValue::Triple(_, _, s) => s,
            _ => panic!("expected Triple"),
        }
    }

    #[test]
    fn two_constants_no_cross() {
        // 5.0 vs 10.0 — A is always below B, never crosses.
        let mut lc = LineCross::new(
            LineSource::Constant(5.0),
            LineSource::Constant(10.0),
            CrossMode::Momentary,
        );
        for _ in 0..10 {
            let v = lc.update_bar(0.0, 0.0, 0.0, 0.0, 0.0);
            assert_eq!(signal_of(v), 0.0, "constants never cross");
        }
    }

    #[test]
    fn constant_vs_indicator_cross_up() {
        // Constant(100) vs SMA(3) that starts below 100 then goes above.
        let mut lc = LineCross::new(
            LineSource::Indicator(Box::new(make_sma(3))),
            LineSource::Constant(100.0),
            CrossMode::Momentary,
        );
        // SMA warms up below 100.
        feed(&mut lc, &[90.0, 92.0, 94.0]);
        // Push prices above 100 sharply.
        let mut saw_cross = false;
        for _ in 0..10 {
            let v = lc.update_bar(120.0, 120.0, 120.0, 120.0, 0.0);
            if signal_of(v) > 0.0 {
                saw_cross = true;
            }
        }
        assert!(saw_cross, "SMA must cross above constant 100 when prices surge to 120");
    }

    #[test]
    fn sticky_mode_holds_sign() {
        let mut lc = LineCross::new(
            LineSource::Indicator(Box::new(make_sma(3))),
            LineSource::Constant(100.0),
            CrossMode::Sticky,
        );
        // Warm up below.
        feed(&mut lc, &[90.0, 90.0, 90.0, 90.0]);
        // Cross up — sticky should become +1 and stay.
        feed(&mut lc, &[120.0, 120.0]);
        let v = lc.update_bar(120.0, 120.0, 120.0, 120.0, 0.0);
        assert_eq!(signal_of(v), 1.0, "sticky should hold +1 after up-cross");
        // Now cross down.
        feed(&mut lc, &[50.0, 50.0, 50.0, 50.0, 50.0]);
        let v2 = lc.update_bar(50.0, 50.0, 50.0, 50.0, 0.0);
        assert_eq!(signal_of(v2), -1.0, "sticky should flip to -1 after down-cross");
    }

    #[test]
    fn momentary_zeroes_after_cross() {
        let mut lc = LineCross::new(
            LineSource::Indicator(Box::new(make_sma(3))),
            LineSource::Constant(100.0),
            CrossMode::Momentary,
        );
        // Cross up.
        feed(&mut lc, &[90.0, 90.0, 90.0]);
        feed(&mut lc, &[120.0, 120.0]);
        // Stay above — subsequent bars should be 0.
        let v = lc.update_bar(120.0, 120.0, 120.0, 120.0, 0.0);
        assert_eq!(signal_of(v), 0.0, "momentary returns 0 bars after the crossover");
    }

    #[test]
    fn reset_clears_state() {
        let mut lc = LineCross::new(
            LineSource::Indicator(Box::new(make_sma(3))),
            LineSource::Constant(50.0),
            CrossMode::Sticky,
        );
        feed(&mut lc, &[80.0; 10]);
        lc.reset();
        assert!(!lc.is_ready());
        assert_eq!(signal_of(lc.value()), 0.0);
    }
}
