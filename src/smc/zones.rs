//! Fair value gaps, order blocks, and displacement legs — Appendix A's
//! "FVG", "Order block", "Breaker block", and "Displacement leg" sections.

use super::atr::atr;
use super::config::{OrderBlockZone, SmcConfig};
use super::market_structure::structure;
use super::types::{DisplacementLeg, Direction, FvgZone, OrderBlock, SmcBar};
use crate::events::fvg_event_detector::fvg_direction;

/// Fair value gaps over three-bar triplets, with forward-scanned mitigation
/// and fill state.
///
/// Reuses `fvg_direction` — the exact predicate `FvgEventDetector` runs —
/// so the streaming and window-form detectors can never disagree about
/// what a FVG is. A gap is *mitigated* the first time price later trades
/// into the zone, and *filled* the first time price closes through the
/// zone's far edge (the edge furthest from the direction price was
/// travelling when it left the gap behind).
pub fn fair_value_gaps(bars: &[SmcBar], _cfg: &SmcConfig) -> Vec<FvgZone> {
    let mut out = Vec::new();
    if bars.len() < 3 {
        return out;
    }

    for i in 2..bars.len() {
        let older = bars[i - 2];
        let newer = bars[i];
        let (bull, bear) = fvg_direction(older.high, older.low, newer.high, newer.low);
        if !bull && !bear {
            continue;
        }

        let direction = if bull { Direction::Up } else { Direction::Down };
        let (low, high) = if bull { (older.high, newer.low) } else { (newer.high, older.low) };

        let mut mitigated_at = None;
        let mut filled_at = None;
        for (j, bar) in bars.iter().enumerate().skip(i + 1) {
            if mitigated_at.is_none() && bar.high >= low && bar.low <= high {
                mitigated_at = Some(j);
            }
            let filled_now = match direction {
                Direction::Up => bar.close <= low,
                Direction::Down => bar.close >= high,
            };
            if filled_now {
                filled_at = Some(j);
                if mitigated_at.is_none() {
                    mitigated_at = Some(j);
                }
                break;
            }
        }

        out.push(FvgZone {
            start_index: i - 2,
            end_index: i,
            low,
            high,
            direction,
            mitigated_at,
            filled_at,
        });
    }
    out
}

/// The last opposing-close candle before a displacement leg that broke
/// structure, per Appendix A's "Order block". Never emitted without the
/// structure break behind it: a `structure::structure` event counts only
/// when it falls INSIDE a same-direction `displacement_legs` leg.
///
/// The leg is matched by SPAN, not by its last bar. Requiring the break
/// bar to equal `leg.end_index` was the original reading and it made order
/// blocks essentially unfindable: a break is the bar whose CLOSE cleared
/// the swing, which normally happens partway through the impulse, while
/// `end_index` is wherever the same-direction run happens to stop. Measured
/// on BTCUSDT 1D over 89 bars: 5 displacement legs, 3 structure breaks, 0
/// order blocks — not one break landed on a leg's final bar. Containment is
/// also the definition traders use ("the candle before the impulse that
/// broke structure"), which says nothing about where the impulse ends.
///
/// `breaker_at` is filled by a forward scan for the first bar whose close
/// crosses decisively through the block — a state transition on this same
/// record, never a second standalone scan.
pub fn order_blocks(bars: &[SmcBar], cfg: &SmcConfig) -> Vec<OrderBlock> {
    let breaks = structure(bars, cfg);
    let legs = displacement_legs(bars, cfg);
    let mut out = Vec::new();

    for event in &breaks {
        let Some(leg) = legs.iter().find(|leg| {
            leg.direction == event.direction
                && leg.start_index <= event.index
                && event.index <= leg.end_index
        }) else {
            continue;
        };
        if leg.start_index == 0 {
            continue; // no bar exists before the leg to serve as the opposing candle
        }

        let Some(ob_index) = (0..leg.start_index).rev().find(|&j| is_opposing_close(bars[j], leg.direction))
        else {
            continue;
        };

        // One leg can contain more than one structure event (a long impulse
        // clearing two swings), and every one of them resolves back to the
        // SAME opposing candle. Keep the first — the earliest break is the
        // one that made the block — instead of emitting the same zone twice.
        if out.iter().any(|b: &OrderBlock| b.index == ob_index) {
            continue;
        }

        let candle = bars[ob_index];
        let (low, high) = order_block_zone(candle, cfg.order_block_zone);
        out.push(OrderBlock {
            index: ob_index,
            low,
            high,
            direction: leg.direction,
            broke_structure_at: event.index,
            breaker_at: find_breaker(bars, ob_index, low, high, leg.direction),
        });
    }
    out
}

fn is_opposing_close(bar: SmcBar, leg_direction: Direction) -> bool {
    match leg_direction {
        Direction::Up => bar.close < bar.open,
        Direction::Down => bar.close > bar.open,
    }
}

fn order_block_zone(bar: SmcBar, zone: OrderBlockZone) -> (f64, f64) {
    match zone {
        OrderBlockZone::FullRange => (bar.low, bar.high),
        OrderBlockZone::BodyOnly => (bar.open.min(bar.close), bar.open.max(bar.close)),
    }
}

fn find_breaker(bars: &[SmcBar], from_index: usize, low: f64, high: f64, direction: Direction) -> Option<usize> {
    ((from_index + 1)..bars.len()).find(|&j| match direction {
        Direction::Up => bars[j].close < low,
        Direction::Down => bars[j].close > high,
    })
}

/// Maximal runs of same-direction candles (by open/close) whose total
/// range exceeds `k × ATR(period)` at the run's start AND which leave at
/// least one fair value gap fully inside the run — Appendix A's
/// "Displacement leg", both conditions required, not either.
pub fn displacement_legs(bars: &[SmcBar], cfg: &SmcConfig) -> Vec<DisplacementLeg> {
    let atr_values = atr(bars, cfg.atr_period);
    let gaps = fair_value_gaps(bars, cfg);
    let mut out = Vec::new();

    let mut run_start: Option<usize> = None;
    let mut run_dir: Option<Direction> = None;

    for i in 0..=bars.len() {
        let bar_dir = bars.get(i).and_then(candle_direction);
        let continues = matches!((run_dir, bar_dir), (Some(d), Some(b)) if d == b);
        if !continues {
            if let (Some(start), Some(direction)) = (run_start, run_dir) {
                if let Some(leg) = evaluate_run(bars, cfg, &atr_values, &gaps, start, i - 1, direction) {
                    out.push(leg);
                }
            }
            run_start = bar_dir.map(|_| i);
            run_dir = bar_dir;
        }
    }
    out
}

fn candle_direction(bar: &SmcBar) -> Option<Direction> {
    if bar.close > bar.open {
        Some(Direction::Up)
    } else if bar.close < bar.open {
        Some(Direction::Down)
    } else {
        None
    }
}

fn evaluate_run(
    bars: &[SmcBar],
    cfg: &SmcConfig,
    atr_values: &[Option<f64>],
    gaps: &[FvgZone],
    start: usize,
    end: usize,
    direction: Direction,
) -> Option<DisplacementLeg> {
    let atr_at_start = atr_values.get(start).copied().flatten()?;
    let high = bars[start..=end].iter().fold(f64::NEG_INFINITY, |m, b| m.max(b.high));
    let low = bars[start..=end].iter().fold(f64::INFINITY, |m, b| m.min(b.low));
    let range = high - low;
    if range <= cfg.displacement_atr_mult * atr_at_start {
        return None;
    }
    let has_gap = gaps
        .iter()
        .any(|g| g.direction == direction && g.start_index >= start && g.end_index <= end);
    if !has_gap {
        return None;
    }
    Some(DisplacementLeg { start_index: start, end_index: end, direction, range, atr_at_start })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(o: f64, h: f64, l: f64, c: f64) -> SmcBar {
        SmcBar { open: o, high: h, low: l, close: c }
    }

    #[test]
    fn bullish_fvg_has_exact_zone_bounds() {
        let bars = vec![
            bar(99.0, 100.0, 98.0, 99.5),  // 0 older
            bar(100.0, 108.0, 105.0, 107.0), // 1 middle/displacement
            bar(105.0, 112.0, 103.0, 110.0), // 2 newer: low(103) > high[0](100)
        ];
        let cfg = SmcConfig::default();
        let gaps = fair_value_gaps(&bars, &cfg);
        assert_eq!(gaps.len(), 1);
        let gap = &gaps[0];
        assert_eq!(gap.start_index, 0);
        assert_eq!(gap.end_index, 2);
        assert_eq!(gap.low, 100.0);
        assert_eq!(gap.high, 103.0);
        assert_eq!(gap.direction, Direction::Up);
    }

    #[test]
    fn bullish_fvg_mitigation_then_fill() {
        let bars = vec![
            bar(99.0, 100.0, 98.0, 99.5),    // 0 older
            bar(100.0, 108.0, 105.0, 107.0), // 1 middle
            bar(105.0, 112.0, 103.0, 110.0), // 2 newer — zone [100, 103]
            bar(110.0, 111.0, 107.5, 109.5), // 3 no overlap
            bar(109.0, 109.5, 101.0, 108.0), // 4 mitigated: low(101) trades into [100,103]
            bar(108.0, 108.5, 95.0, 96.0),   // 5 filled: close(96) <= low edge (100)
        ];
        let cfg = SmcConfig::default();
        let gaps = fair_value_gaps(&bars, &cfg);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].mitigated_at, Some(4));
        assert_eq!(gaps[0].filled_at, Some(5));
    }

    /// Builds an uptrend (swing_n=1) with a confirmed swing high at 26
    /// (index 7), a down-close candle at index 9 (the order block
    /// candidate), and an up displacement leg (indices 10..12) that leaves
    /// a bullish FVG and closes above 26 at its final bar — the structure
    /// break the order block requires behind it. Bar 13 later closes back
    /// below the order block's low, flipping it into a breaker.
    fn order_block_fixture() -> Vec<SmcBar> {
        vec![
            bar(20.0, 22.0, 20.0, 21.0),   // 0
            bar(18.0, 19.0, 17.0, 18.0),   // 1  swing low L1=17
            bar(20.0, 22.0, 20.0, 21.0),   // 2
            bar(23.0, 24.0, 22.0, 23.0),   // 3  swing high H1=24
            bar(21.0, 22.0, 20.0, 21.0),   // 4
            bar(19.0, 20.0, 18.0, 19.0),   // 5  swing low L2=18
            bar(21.0, 22.0, 20.0, 21.0),   // 6
            bar(25.0, 26.0, 24.0, 25.0),   // 7  swing high H2=26
            bar(21.0, 22.0, 20.0, 21.0),   // 8  H2 confirmed; trend Up
            bar(21.0, 22.0, 19.0, 20.0),   // 9  last opposing (down) close before the leg
            bar(20.0, 25.0, 19.0, 24.0),   // 10 leg start (up-close)
            bar(24.0, 25.8, 24.0, 25.5),   // 11 leg continues (up-close, still below 26)
            bar(30.0, 155.0, 30.0, 150.0), // 12 leg ends: close(150) > H2(26) — BOS; bullish FVG vs bar 10
            bar(15.0, 16.0, 8.0, 9.0),     // 13 breaker: close(9) < order block low(19)
        ]
    }

    #[test]
    fn order_block_has_structure_break_and_later_flips_to_breaker() {
        let bars = order_block_fixture();
        let cfg = SmcConfig { swing_n: 1, atr_period: 3, ..SmcConfig::default() };

        let legs = displacement_legs(&bars, &cfg);
        let leg = legs
            .iter()
            .find(|l| l.start_index == 10 && l.end_index == 12 && l.direction == Direction::Up)
            .expect("expected an up displacement leg over bars 10..=12");
        assert_eq!(leg.range, 155.0 - 19.0);

        let blocks = order_blocks(&bars, &cfg);
        let ob = blocks.iter().find(|b| b.index == 9).expect("expected an order block at bar 9");
        assert_eq!(ob.low, 19.0);
        assert_eq!(ob.high, 22.0);
        assert_eq!(ob.direction, Direction::Up);
        assert_eq!(ob.broke_structure_at, 12);
        assert_eq!(ob.breaker_at, Some(13), "close below the block's low at bar 13 must flip it to a breaker");
    }

    #[test]
    fn structure_break_inside_the_leg_still_makes_an_order_block() {
        // The break bar is bar 11 (close 27 > H2=26) and the up run keeps
        // going to bar 12 — the ordinary case on real data, and the one the
        // old `leg.end_index == event.index` reading dropped on the floor.
        let mut bars = order_block_fixture();
        bars[11] = bar(24.0, 28.0, 24.0, 27.0);   // BOS here, mid-leg
        bars[12] = bar(30.0, 155.0, 30.0, 150.0); // run continues past it
        let cfg = SmcConfig { swing_n: 1, atr_period: 3, ..SmcConfig::default() };

        let blocks = order_blocks(&bars, &cfg);
        let ob = blocks
            .iter()
            .find(|b| b.index == 9)
            .expect("a break inside the leg must still resolve the order block at bar 9");
        assert_eq!(ob.direction, Direction::Up);
        assert_eq!(ob.broke_structure_at, 11, "the break that made it, not the leg's last bar");
    }

    #[test]
    fn one_leg_clearing_two_swings_emits_one_order_block() {
        // Both bar 11 and bar 12 close past a swing inside the SAME leg;
        // both resolve back to the same opposing candle at bar 9, and the
        // zone must not be emitted twice.
        let mut bars = order_block_fixture();
        bars[11] = bar(24.0, 28.0, 24.0, 27.0);
        let cfg = SmcConfig { swing_n: 1, atr_period: 3, ..SmcConfig::default() };

        let blocks = order_blocks(&bars, &cfg);
        assert_eq!(
            blocks.iter().filter(|b| b.index == 9).count(),
            1,
            "one opposing candle, one order block, however many swings the leg cleared"
        );
    }

    #[test]
    fn order_block_requires_a_structure_break_behind_it() {
        // Same opposing candle (bar 9), but the run's final close never
        // exceeds the swing high (H2=26) — no structure break, so
        // `structure::structure` never emits an event at bar 12 for a leg
        // to be matched against, and therefore no order block, even though
        // bar 9 is still the last down-close candle before an up-close run.
        let mut bars = order_block_fixture();
        bars[12] = bar(30.0, 25.9, 24.0, 25.9); // stays below H2=26, no BOS
        let cfg = SmcConfig { swing_n: 1, atr_period: 3, ..SmcConfig::default() };
        let blocks = order_blocks(&bars, &cfg);
        assert!(blocks.iter().all(|b| b.index != 9), "no structure break behind it means no order block");
    }
}
