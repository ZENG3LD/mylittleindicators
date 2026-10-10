//! Liquidity sweeps and equal highs/lows — Appendix A's "Liquidity sweep /
//! stop hunt" and "Equal highs / equal lows" sections.

use super::atr::atr;
use super::config::SmcConfig;
use super::market_structure::swings;
use super::types::{Direction, EqualLevels, LiquiditySweep, SmcBar, Swing, SwingKind};

/// A wick beyond a prior confirmed swing extreme with a close back inside
/// it, within `cfg.sweep_recovery_bars` bars of the wick.
///
/// This is the wick counterpart of `structure::structure`'s BOS on the
/// same level: BOS is a CLOSE beyond the level that holds; a sweep is a
/// WICK beyond the level whose close comes back inside within the recovery
/// window. Same level, opposite verdict — conflating the two is the single
/// most common error in this family, so this detector reads only the wick
/// and never a close-beyond-and-stay.
///
/// Each confirmed swing yields at most one sweep: the first bar that wicks
/// beyond it settles the level, whether that wick recovers (a sweep) or
/// does not (a genuine break, left to `structure::structure` to call).
pub fn liquidity_sweeps(bars: &[SmcBar], cfg: &SmcConfig) -> Vec<LiquiditySweep> {
    let points = swings(bars, cfg);
    let mut out = Vec::new();

    for swing in &points {
        let mut j = swing.index + 1;
        while j < bars.len() {
            let wicked = match swing.kind {
                SwingKind::High => bars[j].high > swing.price,
                SwingKind::Low => bars[j].low < swing.price,
            };
            if wicked {
                let window_end = (j + cfg.sweep_recovery_bars).min(bars.len() - 1);
                let recovered = (j..=window_end).find(|&r| match swing.kind {
                    SwingKind::High => bars[r].close < swing.price,
                    SwingKind::Low => bars[r].close > swing.price,
                });
                if let Some(r) = recovered {
                    out.push(LiquiditySweep {
                        index: j,
                        swept_swing_index: swing.index,
                        level: swing.price,
                        direction: match swing.kind {
                            SwingKind::High => Direction::Up,
                            SwingKind::Low => Direction::Down,
                        },
                        recovered_within: r - j,
                    });
                }
                break; // the level is settled — swept, or a genuine break — either way, done
            }
            j += 1;
        }
    }

    out.sort_by_key(|s| s.index);
    out
}

/// Two or more confirmed swing extremes of the SAME kind within
/// `cfg.equal_level_atr_tol × ATR(cfg.atr_period)` of each other.
///
/// Swings of a kind are sorted by price; a new cluster starts whenever the
/// next swing (by price) drifts more than the tolerance — evaluated with
/// the ATR AT THAT SWING's own bar — from the cluster's reference (lowest
/// priced) member.
pub fn equal_levels(bars: &[SmcBar], cfg: &SmcConfig) -> Vec<EqualLevels> {
    let atr_values = atr(bars, cfg.atr_period);
    let points = swings(bars, cfg);
    let mut out = Vec::new();

    for kind in [SwingKind::High, SwingKind::Low] {
        let mut group: Vec<&Swing> = points.iter().filter(|s| s.kind == kind).collect();
        group.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal));

        let mut cluster: Vec<&Swing> = Vec::new();
        for swing in group.drain(..) {
            if let Some(anchor) = cluster.first() {
                match atr_values.get(swing.index).copied().flatten() {
                    Some(atr_here) => {
                        let tol = cfg.equal_level_atr_tol * atr_here;
                        if (swing.price - anchor.price).abs() > tol {
                            flush_cluster(&cluster, kind, &mut out);
                            cluster.clear();
                        }
                    }
                    None => {
                        flush_cluster(&cluster, kind, &mut out);
                        cluster.clear();
                    }
                }
            }
            cluster.push(swing);
        }
        flush_cluster(&cluster, kind, &mut out);
    }

    out.sort_by_key(|e| e.indices.iter().copied().min().unwrap_or(0));
    out
}

fn flush_cluster(cluster: &[&Swing], kind: SwingKind, out: &mut Vec<EqualLevels>) {
    if cluster.len() < 2 {
        return;
    }
    let mut indices: Vec<usize> = cluster.iter().map(|s| s.index).collect();
    indices.sort_unstable();
    let level = cluster.iter().map(|s| s.price).sum::<f64>() / cluster.len() as f64;
    out.push(EqualLevels { indices, level, kind });
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::market_structure::structure;
    use super::super::types::StructureEventKind;

    fn bar(o: f64, h: f64, l: f64, c: f64) -> SmcBar {
        SmcBar { open: o, high: h, low: l, close: c }
    }

    /// The same swing high that `structure::structure` would use as a BOS
    /// level (an uptrend closing above the last confirmed high) is instead
    /// only WICKED through, with the close recovering back inside —
    /// asserting this is a sweep, not a BOS, on that exact level.
    fn sweep_fixture() -> Vec<SmcBar> {
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
            bar(25.0, 45.0, 24.0, 35.0), // 9  wick(45) beyond H2(40), close(35) back inside
            bar(25.0, 27.0, 22.0, 26.0), // 10
        ]
    }

    #[test]
    fn liquidity_sweep_is_not_a_bos_on_the_same_level() {
        let bars = sweep_fixture();
        let cfg = SmcConfig { swing_n: 1, ..SmcConfig::default() };

        let sweeps = liquidity_sweeps(&bars, &cfg);
        let sweep = sweeps.iter().find(|s| s.index == 9).expect("expected a sweep at bar 9");
        assert_eq!(sweep.swept_swing_index, 7);
        assert_eq!(sweep.level, 40.0);
        assert_eq!(sweep.direction, Direction::Up);
        assert_eq!(sweep.recovered_within, 0);

        let events = structure(&bars, &cfg);
        assert!(
            !events.iter().any(|e| e.index == 9 && e.kind == StructureEventKind::Bos),
            "bar 9's wick-then-recovery must not also register as a BOS: {events:?}"
        );
    }

    #[test]
    fn two_equal_highs_within_tolerance_form_one_group() {
        // Two swing highs at 100 and 100.5, well within tolerance once ATR
        // has expanded from the spike itself; a run of flat filler bars
        // warms up the ATR window before either swing forms.
        let mut bars = vec![bar(50.0, 50.0, 49.0, 49.5); 5]; // ATR warmup filler
        bars.extend(vec![
            bar(50.0, 52.0, 50.0, 51.0),
            bar(50.0, 100.0, 50.0, 60.0), // swing high #1 = 100
            bar(50.0, 52.0, 50.0, 51.0),
            bar(50.0, 52.0, 50.0, 51.0),
            bar(50.0, 100.5, 50.0, 60.0), // swing high #2 = 100.5
            bar(50.0, 52.0, 50.0, 51.0),
        ]);
        let cfg = SmcConfig { swing_n: 1, atr_period: 5, equal_level_atr_tol: 0.1, ..SmcConfig::default() };
        let groups = equal_levels(&bars, &cfg);
        let highs: Vec<&EqualLevels> = groups.iter().filter(|g| g.kind == SwingKind::High).collect();
        assert_eq!(highs.len(), 1, "the two close highs must form exactly one group: {groups:?}");
        assert_eq!(highs[0].indices.len(), 2);
    }
}
