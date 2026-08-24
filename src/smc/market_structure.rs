//! Swing points, market structure, BOS/CHoCH, trend, and the dealing range
//! — Appendix A's "Swing point (fractal)", "Market structure", "BOS",
//! "CHoCH", and "Dealing range / premium-discount" sections.

use super::config::SmcConfig;
use super::types::{
    DealingRange, Direction, SmcBar, Swing, SwingKind, StructureEvent, StructureEventKind, Trend,
};

/// Confirmed fractal swing points, sorted by index.
///
/// A swing high at `i` requires `bars[i].high` strictly greater than the
/// highs of `cfg.swing_n` bars on both sides; a swing low is the mirror on
/// lows. A swing is only ever produced once its right-side confirmation
/// window (`cfg.swing_n` bars after it) fully exists in `bars` — the
/// detector must never report a swing it cannot yet know, so the last
/// `cfg.swing_n` bars of the slice can never themselves be reported.
pub fn swings(bars: &[SmcBar], cfg: &SmcConfig) -> Vec<Swing> {
    let n = cfg.swing_n;
    let mut out = Vec::new();
    if n == 0 || bars.len() < 2 * n + 1 {
        return out;
    }

    for i in n..(bars.len() - n) {
        let high = bars[i].high;
        let low = bars[i].low;

        let is_swing_high = bars[i - n..i].iter().all(|b| b.high < high)
            && bars[i + 1..=i + n].iter().all(|b| b.high < high);
        if is_swing_high {
            out.push(Swing { index: i, price: high, kind: SwingKind::High });
        }

        let is_swing_low = bars[i - n..i].iter().all(|b| b.low > low)
            && bars[i + 1..=i + n].iter().all(|b| b.low > low);
        if is_swing_low {
            out.push(Swing { index: i, price: low, kind: SwingKind::Low });
        }
    }
    out
}

/// The last two confirmed swing highs and lows known as of a given bar —
/// the state Appendix A's trend rule reads ("the last two swing highs rise
/// AND the last two swing lows rise").
#[derive(Debug, Clone, Copy, Default)]
struct StructureState {
    prev_high: Option<Swing>,
    last_high: Option<Swing>,
    prev_low: Option<Swing>,
    last_low: Option<Swing>,
}

/// Replays confirmed-swing history up to and including `at_index` and
/// returns the last two confirmed swing highs/lows known at that point.
/// Shared by `trend_at` (a one-off query) and `structure` (which walks this
/// incrementally instead, for the same rule).
fn structure_state_at(swings: &[Swing], swing_n: usize, at_index: usize) -> StructureState {
    let mut state = StructureState::default();
    for s in swings {
        if s.index + swing_n > at_index {
            break; // `swings` is sorted by index, so confirmation bar is monotonic too.
        }
        match s.kind {
            SwingKind::High => {
                state.prev_high = state.last_high;
                state.last_high = Some(*s);
            }
            SwingKind::Low => {
                state.prev_low = state.last_low;
                state.last_low = Some(*s);
            }
        }
    }
    state
}

/// Appendix A's trend rule: "Uptrend = the last two swing highs rise AND
/// the last two swing lows rise; downtrend is the mirror; anything else is
/// unresolved."
fn trend_from_state(state: &StructureState) -> Trend {
    match (state.prev_high, state.last_high, state.prev_low, state.last_low) {
        (Some(ph), Some(lh), Some(pl), Some(ll)) => {
            if lh.price > ph.price && ll.price > pl.price {
                Trend::Up
            } else if lh.price < ph.price && ll.price < pl.price {
                Trend::Down
            } else {
                Trend::Unresolved
            }
        }
        _ => Trend::Unresolved,
    }
}

/// Prevailing market structure at `index`. `Trend::Unresolved` is a real,
/// reported answer — never defaulted to a direction.
pub fn trend_at(bars: &[SmcBar], cfg: &SmcConfig, index: usize) -> Trend {
    let points = swings(bars, cfg);
    let state = structure_state_at(&points, cfg.swing_n, index);
    trend_from_state(&state)
}

/// Break of structure (continuation) and change of character (reversal),
/// walked bar by bar.
///
/// BOS: in an uptrend, the first close above the most recent confirmed
/// swing high; in a downtrend, the first close below the most recent
/// confirmed swing low. CHoCH: the FIRST close beyond the structure in the
/// direction OPPOSITE the prevailing trend — in an uptrend, a close below
/// the most recent confirmed swing low (the "higher low" that was
/// supposed to hold). Both are evaluated only while a trend is resolved:
/// Appendix A defines BOS as continuing a trend and CHoCH as reversing one,
/// and neither concept has a referent while structure is unresolved.
///
/// Each level (a specific confirmed swing) fires at most once — the first
/// close that breaks it — until a new swing of that kind confirms and
/// becomes the new reference level.
pub fn structure(bars: &[SmcBar], cfg: &SmcConfig) -> Vec<StructureEvent> {
    let points = swings(bars, cfg);
    let mut out = Vec::new();

    let mut cursor = 0usize;
    let mut state = StructureState::default();
    let mut last_high_broken = false;
    let mut last_low_broken = false;

    for t in 0..bars.len() {
        while cursor < points.len() && points[cursor].index + cfg.swing_n <= t {
            let s = points[cursor];
            match s.kind {
                SwingKind::High => {
                    state.prev_high = state.last_high;
                    state.last_high = Some(s);
                    last_high_broken = false;
                }
                SwingKind::Low => {
                    state.prev_low = state.last_low;
                    state.last_low = Some(s);
                    last_low_broken = false;
                }
            }
            cursor += 1;
        }

        let close = bars[t].close;
        match trend_from_state(&state) {
            Trend::Up => {
                if let Some(lh) = state.last_high {
                    if !last_high_broken && close > lh.price {
                        out.push(StructureEvent {
                            index: t,
                            kind: StructureEventKind::Bos,
                            direction: Direction::Up,
                            broken_swing_index: lh.index,
                            level: lh.price,
                        });
                        last_high_broken = true;
                    }
                }
                if let Some(ll) = state.last_low {
                    if !last_low_broken && close < ll.price {
                        out.push(StructureEvent {
                            index: t,
                            kind: StructureEventKind::Choch,
                            direction: Direction::Down,
                            broken_swing_index: ll.index,
                            level: ll.price,
                        });
                        last_low_broken = true;
                    }
                }
            }
            Trend::Down => {
                if let Some(ll) = state.last_low {
                    if !last_low_broken && close < ll.price {
                        out.push(StructureEvent {
                            index: t,
                            kind: StructureEventKind::Bos,
                            direction: Direction::Down,
                            broken_swing_index: ll.index,
                            level: ll.price,
                        });
                        last_low_broken = true;
                    }
                }
                if let Some(lh) = state.last_high {
                    if !last_high_broken && close > lh.price {
                        out.push(StructureEvent {
                            index: t,
                            kind: StructureEventKind::Choch,
                            direction: Direction::Up,
                            broken_swing_index: lh.index,
                            level: lh.price,
                        });
                        last_high_broken = true;
                    }
                }
            }
            Trend::Unresolved => {}
        }
    }
    out
}

/// The dealing range taken from the last confirmed swing pair (Appendix A,
/// "Dealing range / premium-discount"): pure geometry, `cfg` is read only
/// to find the confirmed swings, not for any threshold of its own.
pub fn dealing_range(bars: &[SmcBar], cfg: &SmcConfig) -> Option<DealingRange> {
    let points = swings(bars, cfg);
    let last_high = points.iter().rev().find(|s| s.kind == SwingKind::High)?;
    let last_low = points.iter().rev().find(|s| s.kind == SwingKind::Low)?;

    let (low, low_index, high, high_index) = if last_low.price <= last_high.price {
        (last_low.price, last_low.index, last_high.price, last_high.index)
    } else {
        (last_high.price, last_high.index, last_low.price, last_low.index)
    };
    Some(DealingRange { low, high, low_index, high_index })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(o: f64, h: f64, l: f64, c: f64) -> SmcBar {
        SmcBar { open: o, high: h, low: l, close: c }
    }

    /// idx2 is a confirmed swing high (n=2 bars strictly lower on both
    /// sides), idx4 a confirmed swing low. idx5 has the largest high in the
    /// whole slice but only 1 bar follows it (index6) — one short of the
    /// n=2 right-side window it would need — so it must NOT be reported.
    fn swing_fixture() -> Vec<SmcBar> {
        vec![
            bar(10.0, 10.0, 8.0, 9.0),   // 0
            bar(12.0, 12.0, 9.0, 11.0),  // 1
            bar(15.0, 15.0, 11.0, 13.0), // 2  swing high (15)
            bar(11.0, 11.0, 7.0, 9.0),   // 3
            bar(9.0, 9.0, 6.0, 8.0),     // 4  swing low (6)
            bar(20.0, 20.0, 15.0, 18.0), // 5  NOT confirmed (only 1 bar after it)
            bar(14.0, 14.0, 10.0, 12.0), // 6
        ]
    }

    #[test]
    fn swing_high_and_low_confirmed_at_known_indices() {
        let bars = swing_fixture();
        let cfg = SmcConfig::default();
        let points = swings(&bars, &cfg);
        assert_eq!(
            points,
            vec![
                Swing { index: 2, price: 15.0, kind: SwingKind::High },
                Swing { index: 4, price: 6.0, kind: SwingKind::Low },
            ]
        );
    }

    #[test]
    fn swing_missing_right_side_is_not_reported() {
        let bars = swing_fixture();
        let cfg = SmcConfig::default();
        let points = swings(&bars, &cfg);
        assert!(
            points.iter().all(|s| s.index != 5),
            "index 5 lacks a full right-side confirmation window and must not appear: {points:?}"
        );
    }

    /// Builds an uptrend with two confirmed swing highs (30, 40) and two
    /// confirmed swing lows (10, 20), using `swing_n = 1` so each pivot
    /// needs only one bar of margin on each side — this test is about BOS
    /// vs CHoCH classification, not fractal confirmation lag (covered
    /// above with the default `swing_n = 2`).
    fn structure_fixture() -> Vec<SmcBar> {
        vec![
            bar(20.0, 22.0, 20.0, 21.0), // 0
            bar(10.0, 12.0, 10.0, 11.0), // 1  swing low L1=10
            bar(20.0, 22.0, 20.0, 21.0), // 2
            bar(30.0, 30.0, 28.0, 29.0), // 3  swing high H1=30
            bar(21.0, 22.0, 21.0, 21.0), // 4
            bar(20.0, 22.0, 20.0, 21.0), // 5  swing low L2=20
            bar(25.0, 27.0, 25.0, 26.0), // 6
            bar(40.0, 40.0, 38.0, 39.0), // 7  swing high H2=40
            bar(25.0, 27.0, 25.0, 26.0), // 8  H2 confirmed here; trend becomes Up
            bar(25.0, 45.0, 24.0, 45.0), // 9  BOS Up: close(45) > H2(40)
            bar(25.0, 50.0, 5.0, 5.0),   // 10 CHoCH Down: close(5) < L2(20)
        ]
    }

    #[test]
    fn bos_and_choch_get_different_kinds_on_the_same_series() {
        let bars = structure_fixture();
        let cfg = SmcConfig { swing_n: 1, ..SmcConfig::default() };

        let events = structure(&bars, &cfg);
        let bos = events.iter().find(|e| e.index == 9).expect("expected a structure event at bar 9");
        assert_eq!(bos.kind, StructureEventKind::Bos);
        assert_eq!(bos.direction, Direction::Up);
        assert_eq!(bos.level, 40.0);

        let choch = events.iter().find(|e| e.index == 10).expect("expected a structure event at bar 10");
        assert_eq!(choch.kind, StructureEventKind::Choch);
        assert_eq!(choch.direction, Direction::Down);
        assert_eq!(choch.level, 20.0);

        assert_ne!(bos.kind, choch.kind, "BOS and CHoCH must be separate rule ids");
    }

    #[test]
    fn trend_is_unresolved_before_two_highs_and_two_lows_confirm() {
        let bars = structure_fixture();
        let cfg = SmcConfig { swing_n: 1, ..SmcConfig::default() };
        assert_eq!(trend_at(&bars, &cfg, 3), Trend::Unresolved);
        assert_eq!(trend_at(&bars, &cfg, 8), Trend::Up);
    }

    #[test]
    fn dealing_range_uses_last_confirmed_swing_pair() {
        let bars = structure_fixture();
        let cfg = SmcConfig { swing_n: 1, ..SmcConfig::default() };
        let range = dealing_range(&bars, &cfg).expect("expected a dealing range");
        assert_eq!(range.low, 20.0);
        assert_eq!(range.high, 40.0);
        assert_eq!(range.low_index, 5);
        assert_eq!(range.high_index, 7);
        assert_eq!(range.equilibrium(), 30.0);
        assert!(range.is_premium(35.0));
        assert!(range.is_discount(25.0));
    }
}
