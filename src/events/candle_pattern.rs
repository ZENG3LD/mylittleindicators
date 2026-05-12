//! CandlePatternDetector primitive — detects basic candlestick patterns.
//!
//! Implements 4 patterns: Doji, Hammer, ShootingStar, Engulfing.
//! Each fires `SignalKind::Pattern(PatternSub::Candle)`.
//!
//! Maps to `OperatorClass::CandlePattern`.

use std::collections::VecDeque;

use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{PatternSub, SignalKind};

/// Candlestick pattern variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandlePatternKind {
    /// Small body relative to range — indecision.
    Doji,
    /// Small body at top, long lower wick — bullish reversal.
    Hammer,
    /// Small body at bottom, long upper wick — bearish reversal.
    ShootingStar,
    /// Current body fully engulfs previous body.
    Engulfing,
}

/// Detects basic candlestick patterns from OHLC values.
#[derive(Debug, Clone)]
pub struct CandlePatternDetector {
    pattern: CandlePatternKind,
    /// Rolling bar buffer `(o, h, l, c)` — at most 2 bars needed.
    bars: VecDeque<(f64, f64, f64, f64)>,
}

impl CandlePatternDetector {
    pub fn new(pattern: CandlePatternKind) -> Self {
        Self {
            pattern,
            bars: VecDeque::with_capacity(2),
        }
    }

    /// Detect pattern from raw OHLC values (slice-based hot loop).
    ///
    /// Returns `Some((SignalKind::Pattern(PatternSub::Candle), Direction::Up))`
    /// for bullish patterns, `Direction::Down` for bearish, `None` when no pattern.
    pub fn detect_from_values(
        &mut self,
        o: f64,
        h: f64,
        l: f64,
        c: f64,
    ) -> Option<(SignalKind, Direction)> {
        self.bars.push_back((o, h, l, c));
        if self.bars.len() > 2 {
            self.bars.pop_front();
        }

        match self.pattern {
            CandlePatternKind::Doji => self.detect_doji(o, h, l, c),
            CandlePatternKind::Hammer => self.detect_hammer(o, h, l, c),
            CandlePatternKind::ShootingStar => self.detect_shooting_star(o, h, l, c),
            CandlePatternKind::Engulfing => {
                if self.bars.len() < 2 {
                    return None;
                }
                let prev = self.bars[0];
                self.detect_engulfing(prev, (o, h, l, c))
            }
        }
    }

    /// Reset buffer.
    pub fn reset(&mut self) {
        self.bars.clear();
    }

    // ---- internal pattern logic ----

    fn detect_doji(&self, o: f64, h: f64, l: f64, c: f64) -> Option<(SignalKind, Direction)> {
        let range = h - l;
        if range < f64::EPSILON {
            return None;
        }
        let body = (c - o).abs();
        // Doji: body ≤ 10% of range.
        if body / range <= 0.10 {
            // Neutral — emit Up if close >= open (micro-bullish).
            let dir = if c >= o {
                Direction::Up
            } else {
                Direction::Down
            };
            Some((SignalKind::Pattern(PatternSub::Candle), dir))
        } else {
            None
        }
    }

    fn detect_hammer(&self, o: f64, h: f64, l: f64, c: f64) -> Option<(SignalKind, Direction)> {
        let range = h - l;
        if range < f64::EPSILON {
            return None;
        }
        let body = (c - o).abs();
        let body_top = c.max(o);
        let body_bot = c.min(o);
        let lower_wick = body_bot - l;
        let upper_wick = h - body_top;
        // Hammer: lower wick ≥ 2× body, upper wick ≤ body, body in upper 1/3.
        if lower_wick >= 2.0 * body.max(f64::EPSILON)
            && upper_wick <= body.max(f64::EPSILON)
            && body_bot >= l + range * 0.5
        {
            Some((SignalKind::Pattern(PatternSub::Candle), Direction::Up))
        } else {
            None
        }
    }

    fn detect_shooting_star(
        &self,
        o: f64,
        h: f64,
        l: f64,
        c: f64,
    ) -> Option<(SignalKind, Direction)> {
        let range = h - l;
        if range < f64::EPSILON {
            return None;
        }
        let body = (c - o).abs();
        let body_top = c.max(o);
        let body_bot = c.min(o);
        let upper_wick = h - body_top;
        let lower_wick = body_bot - l;
        // Shooting star: upper wick ≥ 2× body, lower wick ≤ body, body in lower 1/3.
        if upper_wick >= 2.0 * body.max(f64::EPSILON)
            && lower_wick <= body.max(f64::EPSILON)
            && body_top <= l + range * 0.5
        {
            Some((SignalKind::Pattern(PatternSub::Candle), Direction::Down))
        } else {
            None
        }
    }

    fn detect_engulfing(
        &self,
        prev: (f64, f64, f64, f64),
        curr: (f64, f64, f64, f64),
    ) -> Option<(SignalKind, Direction)> {
        let (po, _ph, _pl, pc) = prev;
        let (co, _ch, _cl, cc) = curr;
        let prev_body_top = po.max(pc);
        let prev_body_bot = po.min(pc);
        let curr_body_top = co.max(cc);
        let curr_body_bot = co.min(cc);

        if curr_body_top > prev_body_top && curr_body_bot < prev_body_bot {
            // Bullish engulfing: prev bearish, current bullish.
            if pc < po && cc > co {
                Some((SignalKind::Pattern(PatternSub::Candle), Direction::Up))
            } else if pc > po && cc < co {
                // Bearish engulfing: prev bullish, current bearish.
                Some((SignalKind::Pattern(PatternSub::Candle), Direction::Down))
            } else {
                None
            }
        } else {
            None
        }
    }
}
