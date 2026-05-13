//! CandlePatternDetector — unified canon for all 34 candlestick patterns.
//!
//! Replaces 13 individual pattern files, `AdvancedPatternRecognition`, and
//! `CandlePatterns` (momentum). All algorithms ported from `AdvancedPatternRecognition`.
//!
//! Maps to `OperatorClass::CandlePattern`. Output: `IndicatorValue::Signal(i8)`.
//! +1 = bullish, -1 = bearish, 0 = no pattern.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{PatternSub, SignalKind};

/// Candlestick pattern variant — 34 canonical patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandlePatternKind {
    // ---- 1-bar ----
    /// Open ≈ Close, tiny body — indecision.
    Doji,
    /// Long upper wick, no lower wick, body at bottom — bearish.
    GravestoneDoji,
    /// Long lower wick, no upper wick, body at top — bullish.
    DragonflyDoji,
    /// Both wicks long relative to tiny body — indecision.
    LongLeggedDoji,
    /// Long lower wick, body in upper portion — bullish reversal.
    Hammer,
    /// Long upper wick, body in lower portion, bullish close — bullish reversal.
    InvertedHammer,
    /// Long upper wick, body in lower portion, bearish close — bearish reversal.
    ShootingStar,
    /// Long lower wick, body in upper portion, bearish close — bearish continuation.
    HangingMan,
    /// Large body covering nearly full range — strong momentum (generic).
    Marubozu,
    /// Bullish large body with no wicks — strong up.
    WhiteMarubozu,
    /// Bearish large body with no wicks — strong down.
    BlackMarubozu,
    /// Small body with moderate wicks on both sides — indecision.
    SpinningTop,

    // ---- 2-bar ----
    /// Current bullish body fully engulfs previous bearish body — bullish.
    BullishEngulfing,
    /// Current bearish body fully engulfs previous bullish body — bearish.
    BearishEngulfing,
    /// Current small body inside previous large bearish body — bullish reversal.
    BullishHarami,
    /// Current small body inside previous large bullish body — bearish reversal.
    BearishHarami,
    /// Bearish candle + bullish candle opening below, closing above midpoint — bullish.
    PiercingPattern,
    /// Bullish candle + bearish candle opening above, closing below midpoint — bearish.
    DarkCloudCover,
    /// Two candles with matching highs, opposing directions — bearish.
    TweezerTop,
    /// Two candles with matching lows, opposing directions — bullish.
    TweezerBottom,

    // ---- 3-bar ----
    /// Large bearish + small star + large bullish — bullish reversal.
    MorningStar,
    /// Large bullish + small star + large bearish — bearish reversal.
    EveningStar,
    /// Large bearish + doji star + large bullish — bullish reversal (stronger).
    MorningDojiStar,
    /// Large bullish + doji star + large bearish — bearish reversal (stronger).
    EveningDojiStar,
    /// Three consecutive rising bullish candles — strong bullish continuation.
    ThreeWhiteSoldiers,
    /// Three consecutive falling bearish candles — strong bearish continuation.
    ThreeBlackCrows,
    /// Harami (1+2) + confirming bullish bar — bullish.
    ThreeInsideUp,
    /// Harami (1+2) + confirming bearish bar — bearish.
    ThreeInsideDown,
    /// Engulfing (1+2) + confirming bullish bar — bullish.
    ThreeOutsideUp,
    /// Engulfing (1+2) + confirming bearish bar — bearish.
    ThreeOutsideDown,

    // ---- 5-bar ----
    /// Large bullish + 3 small retracements inside range + large bullish — bullish continuation.
    RisingThreeMethods,
    /// Large bearish + 3 small bounces inside range + large bearish — bearish continuation.
    FallingThreeMethods,
    /// Gap up + two bearish candles — bearish continuation.
    UpsideGapTwoCrows,
    /// Gap down + three bearish candles — bearish continuation.
    DownsideGapThreeMethods,
}

impl CandlePatternKind {
    /// Bars needed to detect this pattern.
    fn bars_needed(self) -> usize {
        match self {
            CandlePatternKind::Doji
            | CandlePatternKind::GravestoneDoji
            | CandlePatternKind::DragonflyDoji
            | CandlePatternKind::LongLeggedDoji
            | CandlePatternKind::Hammer
            | CandlePatternKind::InvertedHammer
            | CandlePatternKind::ShootingStar
            | CandlePatternKind::HangingMan
            | CandlePatternKind::Marubozu
            | CandlePatternKind::WhiteMarubozu
            | CandlePatternKind::BlackMarubozu
            | CandlePatternKind::SpinningTop => 1,

            CandlePatternKind::BullishEngulfing
            | CandlePatternKind::BearishEngulfing
            | CandlePatternKind::BullishHarami
            | CandlePatternKind::BearishHarami
            | CandlePatternKind::PiercingPattern
            | CandlePatternKind::DarkCloudCover
            | CandlePatternKind::TweezerTop
            | CandlePatternKind::TweezerBottom => 2,

            CandlePatternKind::MorningStar
            | CandlePatternKind::EveningStar
            | CandlePatternKind::MorningDojiStar
            | CandlePatternKind::EveningDojiStar
            | CandlePatternKind::ThreeWhiteSoldiers
            | CandlePatternKind::ThreeBlackCrows
            | CandlePatternKind::ThreeInsideUp
            | CandlePatternKind::ThreeInsideDown
            | CandlePatternKind::ThreeOutsideUp
            | CandlePatternKind::ThreeOutsideDown
            | CandlePatternKind::UpsideGapTwoCrows => 3,

            CandlePatternKind::RisingThreeMethods
            | CandlePatternKind::FallingThreeMethods
            | CandlePatternKind::DownsideGapThreeMethods => 5,
        }
    }
}

/// Detects any of 34 candlestick patterns from OHLC values.
///
/// Keeps a rolling `bars` buffer of at most 5 bars.
#[derive(Debug, Clone)]
pub struct CandlePatternDetector {
    kind: CandlePatternKind,
    bars: VecDeque<(f64, f64, f64, f64)>, // (o, h, l, c)
    last_signal: i8,
}

// Threshold constants (match AdvancedPatternRecognition defaults)
const DOJI_BODY_RATIO: f64 = 0.10;
const MARUBOZU_BODY_RATIO: f64 = 0.95;
const HAMMER_SHADOW_RATIO: f64 = 2.0;
const HAMMER_OPPOSITE_RATIO: f64 = 0.5;
const HAMMER_BODY_POS: f64 = 0.60;
const ENGULFING_MIN_RATIO: f64 = 1.2;
const SPINNING_TOP_MAX_BODY: f64 = 0.30;
const SOLDIERS_MIN_BODY: f64 = 0.60;
const SOLDIERS_PROGRESSION: f64 = 1.05;
const TWEEZER_TOLERANCE: f64 = 0.02;
const STAR_MAX_BODY: f64 = 0.30;

impl CandlePatternDetector {
    pub fn new(kind: CandlePatternKind) -> Self {
        let cap = kind.bars_needed();
        Self {
            kind,
            bars: VecDeque::with_capacity(cap),
            last_signal: 0,
        }
    }

    /// Feed one OHLC bar. Returns detected `(SignalKind, Direction)` or `None`.
    pub fn detect_from_values(
        &mut self,
        o: f64,
        h: f64,
        l: f64,
        c: f64,
    ) -> Option<(SignalKind, Direction)> {
        self.bars.push_back((o, h, l, c));
        let need = self.kind.bars_needed();
        while self.bars.len() > need {
            self.bars.pop_front();
        }
        if self.bars.len() < need {
            return None;
        }
        detect_pattern(self.kind, &self.bars)
    }

    /// Feed bar, return `IndicatorValue::Signal(+1/-1/0)`.
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, _v: f64) -> IndicatorValue {
        self.last_signal = match self.detect_from_values(o, h, l, c) {
            Some((_, Direction::Up)) => 1,
            Some((_, Direction::Down)) => -1,
            _ => 0,
        };
        IndicatorValue::Signal(self.last_signal)
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    pub fn is_ready(&self) -> bool {
        self.bars.len() >= self.kind.bars_needed()
    }

    pub fn reset(&mut self) {
        self.bars.clear();
        self.last_signal = 0;
    }
}

// ---------------------------------------------------------------------------
// Central dispatch
// ---------------------------------------------------------------------------

fn detect_pattern(
    kind: CandlePatternKind,
    bars: &VecDeque<(f64, f64, f64, f64)>,
) -> Option<(SignalKind, Direction)> {
    let sig = SignalKind::Pattern(PatternSub::Candle);
    match kind {
        // 1-bar
        CandlePatternKind::Doji => check_doji(bars[0], sig),
        CandlePatternKind::GravestoneDoji => check_gravestone_doji(bars[0], sig),
        CandlePatternKind::DragonflyDoji => check_dragonfly_doji(bars[0], sig),
        CandlePatternKind::LongLeggedDoji => check_long_legged_doji(bars[0], sig),
        CandlePatternKind::Hammer => check_hammer(bars[0], sig),
        CandlePatternKind::InvertedHammer => check_inverted_hammer(bars[0], sig),
        CandlePatternKind::ShootingStar => check_shooting_star(bars[0], sig),
        CandlePatternKind::HangingMan => check_hanging_man(bars[0], sig),
        CandlePatternKind::Marubozu => check_marubozu(bars[0], sig),
        CandlePatternKind::WhiteMarubozu => check_white_marubozu(bars[0], sig),
        CandlePatternKind::BlackMarubozu => check_black_marubozu(bars[0], sig),
        CandlePatternKind::SpinningTop => check_spinning_top(bars[0], sig),
        // 2-bar
        CandlePatternKind::BullishEngulfing => check_bullish_engulfing(bars[0], bars[1], sig),
        CandlePatternKind::BearishEngulfing => check_bearish_engulfing(bars[0], bars[1], sig),
        CandlePatternKind::BullishHarami => check_bullish_harami(bars[0], bars[1], sig),
        CandlePatternKind::BearishHarami => check_bearish_harami(bars[0], bars[1], sig),
        CandlePatternKind::PiercingPattern => check_piercing(bars[0], bars[1], sig),
        CandlePatternKind::DarkCloudCover => check_dark_cloud_cover(bars[0], bars[1], sig),
        CandlePatternKind::TweezerTop => check_tweezer_top(bars[0], bars[1], sig),
        CandlePatternKind::TweezerBottom => check_tweezer_bottom(bars[0], bars[1], sig),
        // 3-bar
        CandlePatternKind::MorningStar => {
            check_morning_star(bars[0], bars[1], bars[2], false, sig)
        }
        CandlePatternKind::EveningStar => {
            check_evening_star(bars[0], bars[1], bars[2], false, sig)
        }
        CandlePatternKind::MorningDojiStar => {
            check_morning_star(bars[0], bars[1], bars[2], true, sig)
        }
        CandlePatternKind::EveningDojiStar => {
            check_evening_star(bars[0], bars[1], bars[2], true, sig)
        }
        CandlePatternKind::ThreeWhiteSoldiers => {
            check_three_white_soldiers(bars[0], bars[1], bars[2], sig)
        }
        CandlePatternKind::ThreeBlackCrows => {
            check_three_black_crows(bars[0], bars[1], bars[2], sig)
        }
        CandlePatternKind::ThreeInsideUp => check_three_inside_up(bars[0], bars[1], bars[2], sig),
        CandlePatternKind::ThreeInsideDown => {
            check_three_inside_down(bars[0], bars[1], bars[2], sig)
        }
        CandlePatternKind::ThreeOutsideUp => {
            check_three_outside_up(bars[0], bars[1], bars[2], sig)
        }
        CandlePatternKind::ThreeOutsideDown => {
            check_three_outside_down(bars[0], bars[1], bars[2], sig)
        }
        CandlePatternKind::UpsideGapTwoCrows => {
            check_upside_gap_two_crows(bars[0], bars[1], bars[2], sig)
        }
        // 5-bar
        CandlePatternKind::RisingThreeMethods => check_rising_three_methods(bars, sig),
        CandlePatternKind::FallingThreeMethods => check_falling_three_methods(bars, sig),
        CandlePatternKind::DownsideGapThreeMethods => {
            check_downside_gap_three_methods(bars, sig)
        }
    }
}

// ---------------------------------------------------------------------------
// 1-bar helpers
// ---------------------------------------------------------------------------

#[inline]
fn body_range(o: f64, h: f64, l: f64, c: f64) -> (f64, f64, f64, f64, f64) {
    let body = (c - o).abs();
    let range = h - l;
    let body_top = c.max(o);
    let body_bot = c.min(o);
    let upper_wick = h - body_top;
    let lower_wick = body_bot - l;
    (body, range, upper_wick, lower_wick, body_top)
}

fn check_doji(bar: (f64, f64, f64, f64), sig: SignalKind) -> Option<(SignalKind, Direction)> {
    let (o, h, l, c) = bar;
    let (body, range, ..) = body_range(o, h, l, c);
    if range < f64::EPSILON {
        return None;
    }
    if body / range <= DOJI_BODY_RATIO {
        let dir = if c >= o { Direction::Up } else { Direction::Down };
        Some((sig, dir))
    } else {
        None
    }
}

fn check_gravestone_doji(
    bar: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (o, h, l, c) = bar;
    let (body, range, upper_wick, lower_wick, ..) = body_range(o, h, l, c);
    if range < f64::EPSILON {
        return None;
    }
    // doji + upper wick > 3× body + lower wick tiny
    if body / range <= DOJI_BODY_RATIO
        && upper_wick > 3.0 * body.max(f64::EPSILON)
        && lower_wick < body.max(f64::EPSILON)
    {
        Some((sig, Direction::Down))
    } else {
        None
    }
}

fn check_dragonfly_doji(
    bar: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (o, h, l, c) = bar;
    let (body, range, upper_wick, lower_wick, ..) = body_range(o, h, l, c);
    if range < f64::EPSILON {
        return None;
    }
    if body / range <= DOJI_BODY_RATIO
        && lower_wick > 3.0 * body.max(f64::EPSILON)
        && upper_wick < body.max(f64::EPSILON)
    {
        Some((sig, Direction::Up))
    } else {
        None
    }
}

fn check_long_legged_doji(
    bar: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (o, h, l, c) = bar;
    let (body, range, upper_wick, lower_wick, ..) = body_range(o, h, l, c);
    if range < f64::EPSILON {
        return None;
    }
    if body / range <= DOJI_BODY_RATIO
        && upper_wick > 2.0 * body.max(f64::EPSILON)
        && lower_wick > 2.0 * body.max(f64::EPSILON)
    {
        let dir = if c >= o { Direction::Up } else { Direction::Down };
        Some((sig, dir))
    } else {
        None
    }
}

fn check_hammer(bar: (f64, f64, f64, f64), sig: SignalKind) -> Option<(SignalKind, Direction)> {
    let (o, h, l, c) = bar;
    let (body, range, upper_wick, lower_wick, body_bot) = body_range(o, h, l, c);
    if range < f64::EPSILON {
        return None;
    }
    if lower_wick >= body.max(f64::EPSILON) * HAMMER_SHADOW_RATIO
        && upper_wick <= body.max(f64::EPSILON) * HAMMER_OPPOSITE_RATIO
        && (body_bot - l) / range >= HAMMER_BODY_POS
    {
        Some((sig, Direction::Up))
    } else {
        None
    }
}

fn check_inverted_hammer(
    bar: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (o, h, l, c) = bar;
    let (body, range, upper_wick, lower_wick, body_top) = body_range(o, h, l, c);
    if range < f64::EPSILON {
        return None;
    }
    // Inverted hammer: bullish close, long upper wick, tiny lower wick, body at bottom
    if c > o
        && upper_wick >= body.max(f64::EPSILON) * HAMMER_SHADOW_RATIO
        && lower_wick <= body.max(f64::EPSILON) * HAMMER_OPPOSITE_RATIO
        && (h - body_top) / range >= HAMMER_BODY_POS
    {
        Some((sig, Direction::Up))
    } else {
        None
    }
}

fn check_shooting_star(
    bar: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (o, h, l, c) = bar;
    let (body, range, upper_wick, lower_wick, body_top) = body_range(o, h, l, c);
    if range < f64::EPSILON {
        return None;
    }
    // Bearish close, long upper wick, body in lower portion
    if c < o
        && upper_wick >= body.max(f64::EPSILON) * HAMMER_SHADOW_RATIO
        && lower_wick <= body.max(f64::EPSILON) * HAMMER_OPPOSITE_RATIO
        && (h - body_top) / range >= HAMMER_BODY_POS
    {
        Some((sig, Direction::Down))
    } else {
        None
    }
}

fn check_hanging_man(
    bar: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (o, h, l, c) = bar;
    let (body, range, upper_wick, lower_wick, body_bot) = body_range(o, h, l, c);
    if range < f64::EPSILON {
        return None;
    }
    // Bearish close, hammer shape: long lower wick, body at top
    if c < o
        && lower_wick >= body.max(f64::EPSILON) * HAMMER_SHADOW_RATIO
        && upper_wick <= body.max(f64::EPSILON) * HAMMER_OPPOSITE_RATIO
        && (body_bot - l) / range >= HAMMER_BODY_POS
    {
        Some((sig, Direction::Down))
    } else {
        None
    }
}

fn check_marubozu(bar: (f64, f64, f64, f64), sig: SignalKind) -> Option<(SignalKind, Direction)> {
    let (o, _h, _l, c) = bar;
    let (body, range, ..) = body_range(o, _h, _l, c);
    if range < f64::EPSILON {
        return None;
    }
    if body / range >= MARUBOZU_BODY_RATIO {
        let dir = if c > o { Direction::Up } else { Direction::Down };
        Some((sig, dir))
    } else {
        None
    }
}

fn check_white_marubozu(
    bar: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (o, h, l, c) = bar;
    let (body, range, ..) = body_range(o, h, l, c);
    if range < f64::EPSILON {
        return None;
    }
    if c > o && body / range >= MARUBOZU_BODY_RATIO {
        Some((sig, Direction::Up))
    } else {
        None
    }
}

fn check_black_marubozu(
    bar: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (o, h, l, c) = bar;
    let (body, range, ..) = body_range(o, h, l, c);
    if range < f64::EPSILON {
        return None;
    }
    if c < o && body / range >= MARUBOZU_BODY_RATIO {
        Some((sig, Direction::Down))
    } else {
        None
    }
}

fn check_spinning_top(
    bar: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (o, h, l, c) = bar;
    let (body, range, upper_wick, lower_wick, ..) = body_range(o, h, l, c);
    if range < f64::EPSILON {
        return None;
    }
    let body_ratio = body / range;
    let shadow_ratio = (upper_wick + lower_wick) / range;
    if body_ratio < SPINNING_TOP_MAX_BODY
        && shadow_ratio > 0.6
        && upper_wick > body
        && lower_wick > body
    {
        let dir = if c >= o { Direction::Up } else { Direction::Down };
        Some((sig, dir))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// 2-bar helpers
// ---------------------------------------------------------------------------

fn check_bullish_engulfing(
    prev: (f64, f64, f64, f64),
    curr: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (po, _, _, pc) = prev;
    let (co, _, _, cc) = curr;
    let prev_body = (pc - po).abs();
    let curr_body = (cc - co).abs();
    if curr_body < prev_body * ENGULFING_MIN_RATIO {
        return None;
    }
    if pc < po && cc > co && co <= pc && cc >= po {
        Some((sig, Direction::Up))
    } else {
        None
    }
}

fn check_bearish_engulfing(
    prev: (f64, f64, f64, f64),
    curr: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (po, _, _, pc) = prev;
    let (co, _, _, cc) = curr;
    let prev_body = (pc - po).abs();
    let curr_body = (cc - co).abs();
    if curr_body < prev_body * ENGULFING_MIN_RATIO {
        return None;
    }
    if pc > po && cc < co && co >= pc && cc <= po {
        Some((sig, Direction::Down))
    } else {
        None
    }
}

fn check_bullish_harami(
    prev: (f64, f64, f64, f64),
    curr: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (po, _, _, pc) = prev;
    let (co, _, _, cc) = curr;
    let prev_top = po.max(pc);
    let prev_bot = po.min(pc);
    let curr_top = co.max(cc);
    let curr_bot = co.min(cc);
    // Previous was bearish, current body inside previous body
    if pc < po && curr_top <= prev_top && curr_bot >= prev_bot {
        Some((sig, Direction::Up))
    } else {
        None
    }
}

fn check_bearish_harami(
    prev: (f64, f64, f64, f64),
    curr: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (po, _, _, pc) = prev;
    let (co, _, _, cc) = curr;
    let prev_top = po.max(pc);
    let prev_bot = po.min(pc);
    let curr_top = co.max(cc);
    let curr_bot = co.min(cc);
    // Previous was bullish, current body inside previous body
    if pc > po && curr_top <= prev_top && curr_bot >= prev_bot {
        Some((sig, Direction::Down))
    } else {
        None
    }
}

fn check_piercing(
    prev: (f64, f64, f64, f64),
    curr: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (po, _, _, pc) = prev;
    let (co, _, _, cc) = curr;
    let prev_mid = (po + pc) / 2.0;
    // Bearish prev + bullish curr opening below prev close, closing above midpoint of prev
    if pc < po && cc > co && co < pc && cc > prev_mid && cc < po {
        Some((sig, Direction::Up))
    } else {
        None
    }
}

fn check_dark_cloud_cover(
    prev: (f64, f64, f64, f64),
    curr: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (po, _, _, pc) = prev;
    let (co, _, _, cc) = curr;
    let prev_mid = (po + pc) / 2.0;
    // Bullish prev + bearish curr opening above prev close, closing below midpoint of prev
    if pc > po && cc < co && co > pc && cc < prev_mid && cc > po {
        Some((sig, Direction::Down))
    } else {
        None
    }
}

fn check_tweezer_top(
    prev: (f64, f64, f64, f64),
    curr: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (po, ph, pl, pc) = prev;
    let (co, ch, cl, cc) = curr;
    let avg_range = ((ph - pl) + (ch - cl)) / 2.0;
    if avg_range < f64::EPSILON {
        return None;
    }
    let high_diff = (ph - ch).abs();
    if high_diff / avg_range < TWEEZER_TOLERANCE && pc > po && cc < co {
        Some((sig, Direction::Down))
    } else {
        None
    }
}

fn check_tweezer_bottom(
    prev: (f64, f64, f64, f64),
    curr: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (po, ph, pl, pc) = prev;
    let (co, ch, cl, cc) = curr;
    let avg_range = ((ph - pl) + (ch - cl)) / 2.0;
    if avg_range < f64::EPSILON {
        return None;
    }
    let low_diff = (pl - cl).abs();
    if low_diff / avg_range < TWEEZER_TOLERANCE && pc < po && cc > co {
        Some((sig, Direction::Up))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// 3-bar helpers
// ---------------------------------------------------------------------------

fn is_doji_bar(o: f64, h: f64, l: f64, c: f64) -> bool {
    let body = (c - o).abs();
    let range = h - l;
    range > f64::EPSILON && body / range <= DOJI_BODY_RATIO
}

fn check_morning_star(
    first: (f64, f64, f64, f64),
    second: (f64, f64, f64, f64),
    third: (f64, f64, f64, f64),
    require_doji: bool,
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (fo, fh, fl, fc) = first;
    let (so, sh, sl, sc) = second;
    let (to, th, tl, tc) = third;
    let second_body = (sc - so).abs();
    let avg_range = ((fh - fl) + (sh - sl) + (th - tl)) / 3.0;
    if avg_range < f64::EPSILON {
        return None;
    }
    let second_is_small = second_body / avg_range <= STAR_MAX_BODY;
    if require_doji && !is_doji_bar(so, sh, sl, sc) {
        return None;
    }
    if !second_is_small {
        return None;
    }
    // First bearish, third bullish, gap down for star, third closes above first midpoint
    if fc < fo
        && tc > to
        && sh < fc.min(fo)
        && tc > (fo + fc) / 2.0
    {
        Some((sig, Direction::Up))
    } else {
        None
    }
}

fn check_evening_star(
    first: (f64, f64, f64, f64),
    second: (f64, f64, f64, f64),
    third: (f64, f64, f64, f64),
    require_doji: bool,
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (fo, fh, fl, fc) = first;
    let (so, sh, sl, sc) = second;
    let (to, th, tl, tc) = third;
    let second_body = (sc - so).abs();
    let avg_range = ((fh - fl) + (sh - sl) + (th - tl)) / 3.0;
    if avg_range < f64::EPSILON {
        return None;
    }
    let second_is_small = second_body / avg_range <= STAR_MAX_BODY;
    if require_doji && !is_doji_bar(so, sh, sl, sc) {
        return None;
    }
    if !second_is_small {
        return None;
    }
    // First bullish, third bearish, gap up for star, third closes below first midpoint
    if fc > fo
        && tc < to
        && sl > fc.max(fo)
        && tc < (fo + fc) / 2.0
    {
        Some((sig, Direction::Down))
    } else {
        None
    }
}

fn check_three_white_soldiers(
    first: (f64, f64, f64, f64),
    second: (f64, f64, f64, f64),
    third: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let candles = [first, second, third];
    for i in 0..3 {
        let (o, h, l, c) = candles[i];
        let range = h - l;
        if range < f64::EPSILON {
            return None;
        }
        let body_ratio = (c - o).abs() / range;
        if c <= o || body_ratio < SOLDIERS_MIN_BODY {
            return None;
        }
        if i > 0 {
            let prev_c = candles[i - 1].3;
            if o <= prev_c || c <= candles[i - 1].3 * SOLDIERS_PROGRESSION {
                return None;
            }
        }
    }
    Some((sig, Direction::Up))
}

fn check_three_black_crows(
    first: (f64, f64, f64, f64),
    second: (f64, f64, f64, f64),
    third: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let candles = [first, second, third];
    for i in 0..3 {
        let (o, h, l, c) = candles[i];
        let range = h - l;
        if range < f64::EPSILON {
            return None;
        }
        let body_ratio = (c - o).abs() / range;
        if c >= o || body_ratio < SOLDIERS_MIN_BODY {
            return None;
        }
        if i > 0 {
            let prev_c = candles[i - 1].3;
            // Each opens below previous close and closes below previous close / progression
            if o >= prev_c || c >= candles[i - 1].3 / SOLDIERS_PROGRESSION {
                return None;
            }
        }
    }
    Some((sig, Direction::Down))
}

fn check_three_inside_up(
    first: (f64, f64, f64, f64),
    second: (f64, f64, f64, f64),
    third: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    // Harami(first, second) + third closes above first open
    check_bullish_harami(first, second, sig)?;
    let (fo, _, _, _) = first;
    let (_, _, _, tc) = third;
    if tc > fo {
        Some((sig, Direction::Up))
    } else {
        None
    }
}

fn check_three_inside_down(
    first: (f64, f64, f64, f64),
    second: (f64, f64, f64, f64),
    third: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    check_bearish_harami(first, second, sig)?;
    let (fo, _, _, _) = first;
    let (_, _, _, tc) = third;
    if tc < fo {
        Some((sig, Direction::Down))
    } else {
        None
    }
}

fn check_three_outside_up(
    first: (f64, f64, f64, f64),
    second: (f64, f64, f64, f64),
    third: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    check_bullish_engulfing(first, second, sig)?;
    let (_, _, _, sc) = second;
    let (_, _, _, tc) = third;
    if tc > sc {
        Some((sig, Direction::Up))
    } else {
        None
    }
}

fn check_three_outside_down(
    first: (f64, f64, f64, f64),
    second: (f64, f64, f64, f64),
    third: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    check_bearish_engulfing(first, second, sig)?;
    let (_, _, _, sc) = second;
    let (_, _, _, tc) = third;
    if tc < sc {
        Some((sig, Direction::Down))
    } else {
        None
    }
}

fn check_upside_gap_two_crows(
    first: (f64, f64, f64, f64),
    second: (f64, f64, f64, f64),
    third: (f64, f64, f64, f64),
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (fo, _, _, fc) = first;
    let (so, _sh, _sl, sc) = second;
    let (to, _th, _tl, tc) = third;
    // First bullish, second bearish gapping up, third bearish engulfing second
    if fc > fo
        && sc < so
        && so > fc
        && tc < to
        && to >= so
        && tc <= sc
    {
        Some((sig, Direction::Down))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// 5-bar helpers
// ---------------------------------------------------------------------------

fn check_rising_three_methods(
    bars: &VecDeque<(f64, f64, f64, f64)>,
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    // bars[0] = oldest, bars[4] = newest
    let (fo, fh, fl, fc) = bars[0];
    let (lo, lh, ll, lc) = bars[4];
    let first_range = fh - fl;
    let last_range = lh - ll;
    if first_range < f64::EPSILON || last_range < f64::EPSILON {
        return None;
    }
    // First and last are large bullish candles
    if !(fc > fo && (fc - fo) / first_range > SOLDIERS_MIN_BODY) {
        return None;
    }
    if !(lc > lo && (lc - lo) / last_range > SOLDIERS_MIN_BODY) {
        return None;
    }
    if lc <= fc {
        return None;
    }
    // Middle three stay within first candle's range
    for i in 1..4 {
        let (_, mh, ml, _) = bars[i];
        if mh > fh || ml < fl {
            return None;
        }
    }
    Some((sig, Direction::Up))
}

fn check_falling_three_methods(
    bars: &VecDeque<(f64, f64, f64, f64)>,
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    let (fo, fh, fl, fc) = bars[0];
    let (lo, lh, ll, lc) = bars[4];
    let first_range = fh - fl;
    let last_range = lh - ll;
    if first_range < f64::EPSILON || last_range < f64::EPSILON {
        return None;
    }
    if !(fc < fo && (fo - fc) / first_range > SOLDIERS_MIN_BODY) {
        return None;
    }
    if !(lc < lo && (lo - lc) / last_range > SOLDIERS_MIN_BODY) {
        return None;
    }
    if lc >= fc {
        return None;
    }
    for i in 1..4 {
        let (_, mh, ml, _) = bars[i];
        if mh > fh || ml < fl {
            return None;
        }
    }
    Some((sig, Direction::Down))
}

fn check_downside_gap_three_methods(
    bars: &VecDeque<(f64, f64, f64, f64)>,
    sig: SignalKind,
) -> Option<(SignalKind, Direction)> {
    // Classic: two bearish candles with a gap down between them + bearish follow-through
    // Using 5 bars: bars[0..1] = first bearish, bars[1..2] gap + second, bars[4] = confirm
    let (fo, _, _, fc) = bars[0];
    let (so, _, _, sc) = bars[1];
    let (_, _, _, lc) = bars[4];
    // First bearish, second gaps down (opens below first close), continues down
    if fc < fo && so < fc && sc < so && lc < sc {
        Some((sig, Direction::Down))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn signal(det: &mut CandlePatternDetector, o: f64, h: f64, l: f64, c: f64) -> i8 {
        match det.update_bar(o, h, l, c, 0.0) {
            IndicatorValue::Signal(s) => s,
            _ => 0,
        }
    }

    // ---- original 4-pattern regression ----

    #[test]
    fn doji_detected() {
        let mut d = CandlePatternDetector::new(CandlePatternKind::Doji);
        // open == close, range 4 → body 0 → 0%
        let s = signal(&mut d, 100.0, 102.0, 98.0, 100.0);
        assert_ne!(s, 0, "Doji must fire");
    }

    #[test]
    fn hammer_detected() {
        let mut d = CandlePatternDetector::new(CandlePatternKind::Hammer);
        // lower wick = 9, body = 1, upper wick = 0 → hammer
        let s = signal(&mut d, 100.0, 101.0, 90.0, 101.0);
        assert_eq!(s, 1, "Hammer must fire +1");
    }

    #[test]
    fn shooting_star_detected() {
        let mut d = CandlePatternDetector::new(CandlePatternKind::ShootingStar);
        // upper wick = 9, body = 1, lower wick = 0 → shooting star (bearish close)
        let s = signal(&mut d, 101.0, 110.0, 100.0, 100.0);
        assert_eq!(s, -1, "ShootingStar must fire -1");
    }

    #[test]
    fn bullish_engulfing_detected() {
        let mut d = CandlePatternDetector::new(CandlePatternKind::BullishEngulfing);
        // prev: bearish 105→100 (open>close)
        signal(&mut d, 105.0, 106.0, 99.0, 100.0);
        // curr: bullish engulfing: open=99, close=108 → engulfs 100..105
        let s = signal(&mut d, 99.0, 110.0, 98.0, 108.0);
        assert_eq!(s, 1, "BullishEngulfing must fire +1");
    }

    // ---- new 1-bar patterns ----

    #[test]
    fn marubozu_bullish() {
        let mut d = CandlePatternDetector::new(CandlePatternKind::WhiteMarubozu);
        // body = 9.5, range = 10 → 95%
        let s = signal(&mut d, 100.0, 110.0, 100.0, 109.5);
        assert_eq!(s, 1, "WhiteMarubozu must fire +1");
    }

    #[test]
    fn marubozu_generic_bearish() {
        let mut d = CandlePatternDetector::new(CandlePatternKind::Marubozu);
        // bearish: open 110, close 100.5, range 10 → body/range ≈ 95%
        let s = signal(&mut d, 110.0, 110.0, 100.0, 100.5);
        assert_eq!(s, -1, "Marubozu bearish must fire -1");
    }

    // ---- new 2-bar patterns ----

    #[test]
    fn bullish_harami_detected() {
        let mut d = CandlePatternDetector::new(CandlePatternKind::BullishHarami);
        // prev: large bearish 110→100
        signal(&mut d, 110.0, 112.0, 99.0, 100.0);
        // curr: small body inside 104..106
        let s = signal(&mut d, 104.0, 106.0, 103.0, 105.0);
        assert_eq!(s, 1, "BullishHarami must fire +1");
    }

    #[test]
    fn dark_cloud_cover_detected() {
        let mut d = CandlePatternDetector::new(CandlePatternKind::DarkCloudCover);
        // prev: bullish 100→110
        signal(&mut d, 100.0, 112.0, 99.0, 110.0);
        // curr: opens above 110 (at 112), closes at 104 (below midpoint 105)
        let s = signal(&mut d, 112.0, 113.0, 103.0, 104.0);
        assert_eq!(s, -1, "DarkCloudCover must fire -1");
    }

    // ---- 3-bar patterns ----

    #[test]
    fn morning_star_detected() {
        let mut d = CandlePatternDetector::new(CandlePatternKind::MorningStar);
        // bar1: large bearish 110→100
        signal(&mut d, 110.0, 111.0, 99.0, 100.0);
        // bar2: small star below — gaps down (high 98 < 100)
        signal(&mut d, 97.0, 98.0, 96.0, 97.5);
        // bar3: bullish, closes above midpoint of bar1 (105)
        let s = signal(&mut d, 99.0, 112.0, 98.0, 107.0);
        assert_eq!(s, 1, "MorningStar must fire +1");
    }

    #[test]
    fn three_white_soldiers_detected() {
        let mut d = CandlePatternDetector::new(CandlePatternKind::ThreeWhiteSoldiers);
        // bar1
        signal(&mut d, 100.0, 107.0, 99.0, 106.5);
        // bar2: opens above prev close (107), closes higher
        signal(&mut d, 107.0, 115.0, 106.0, 114.5);
        // bar3: opens above prev close (115), closes higher
        let s = signal(&mut d, 115.0, 124.0, 114.0, 123.5);
        assert_eq!(s, 1, "ThreeWhiteSoldiers must fire +1");
    }

    // ---- 5-bar patterns ----

    #[test]
    fn rising_three_methods_detected() {
        let mut d = CandlePatternDetector::new(CandlePatternKind::RisingThreeMethods);
        // bar1: large bullish 100→108 (range 10, body 8 = 80%)
        signal(&mut d, 100.0, 110.0, 100.0, 108.0);
        // bar2-4: small retracements inside 100..110
        signal(&mut d, 106.0, 109.0, 103.0, 104.0);
        signal(&mut d, 104.0, 107.0, 102.0, 103.0);
        signal(&mut d, 103.0, 108.0, 101.0, 105.0);
        // bar5: large bullish breaking out
        let s = signal(&mut d, 106.0, 120.0, 105.0, 119.0);
        assert_eq!(s, 1, "RisingThreeMethods must fire +1");
    }

    #[test]
    fn gravestone_doji_detected() {
        let mut d = CandlePatternDetector::new(CandlePatternKind::GravestoneDoji);
        // body tiny, large upper wick, no lower wick
        // o=100, c=100.1 (body 0.1), h=110 (upper=9.9), l=100 (lower=0)
        let s = signal(&mut d, 100.0, 110.0, 100.0, 100.1);
        assert_eq!(s, -1, "GravestoneDoji must fire -1");
    }

    #[test]
    fn tweezer_bottom_detected() {
        let mut d = CandlePatternDetector::new(CandlePatternKind::TweezerBottom);
        // bar1: bearish, low at 95
        signal(&mut d, 102.0, 103.0, 95.0, 98.0);
        // bar2: bullish, same low ~95
        let s = signal(&mut d, 97.0, 105.0, 95.1, 103.0);
        // avg_range ~ 9, diff 0.1/9 ≈ 0.011 < 0.02 ✓
        assert_eq!(s, 1, "TweezerBottom must fire +1");
    }

    #[test]
    fn reset_clears_buffer() {
        let mut d = CandlePatternDetector::new(CandlePatternKind::BullishEngulfing);
        signal(&mut d, 105.0, 106.0, 99.0, 100.0);
        d.reset();
        assert!(!d.is_ready());
        assert_eq!(d.value(), IndicatorValue::Signal(0));
    }
}
