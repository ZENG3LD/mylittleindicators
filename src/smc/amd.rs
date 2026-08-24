//! Accumulation / manipulation / distribution — Appendix A's "AMD" section.
//!
//! Not a fourth detector: an ASSEMBLY of `liquidity::liquidity_sweeps` (the
//! manipulation phase) and `zones::displacement_legs` (the distribution
//! phase). Nothing here re-tests a wick, a close, or a range — every field
//! on the emitted `AmdCycle` is copied from a record one of those two
//! detectors already produced.

use super::config::SmcConfig;
use super::liquidity::liquidity_sweeps;
use super::types::{AmdCycle, Direction, SmcBar};
use super::zones::displacement_legs;

/// Assembles AMD cycles: for each liquidity sweep (the manipulation), the
/// first displacement leg starting at or after it whose direction is the
/// MIRROR of the sweep's direction (a sweep of a swing high is manipulation
/// to the upside, so genuine distribution runs down, and vice versa)
/// becomes that cycle's distribution leg. The accumulation phase is the
/// span from the swept swing's own formation to the sweep itself — the
/// consolidation that let the swept level's liquidity build up before it
/// was raided.
pub fn amd_cycles(bars: &[SmcBar], cfg: &SmcConfig) -> Vec<AmdCycle> {
    let sweeps = liquidity_sweeps(bars, cfg);
    let legs = displacement_legs(bars, cfg);
    let mut out = Vec::new();

    for (i, sweep) in sweeps.iter().enumerate() {
        let expected_distribution_dir = match sweep.direction {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        };
        // Two bounds, both of them about the same thing: a distribution leg
        // is the REACTION to this raid, so a leg that arrives long after it
        // — or after a LATER raid has already happened — belongs to some
        // other cycle, or to none. Without them the assembly would marry a
        // sweep to the next opposite-direction leg however many hundreds of
        // bars away it sat, and report the pair as one cycle.
        let next_sweep_index = sweeps.get(i + 1).map(|s| s.index).unwrap_or(usize::MAX);
        let lag_limit = sweep.index.saturating_add(cfg.amd_max_distribution_lag);
        let distribution = legs.iter().find(|leg| {
            leg.direction == expected_distribution_dir
                && leg.start_index >= sweep.index
                && leg.start_index <= lag_limit
                && leg.start_index < next_sweep_index
        });

        if let Some(distribution) = distribution {
            out.push(AmdCycle {
                accumulation: sweep.swept_swing_index..sweep.index,
                manipulation: *sweep,
                distribution: *distribution,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(o: f64, h: f64, l: f64, c: f64) -> SmcBar {
        SmcBar { open: o, high: h, low: l, close: c }
    }

    /// Builds an uptrend (swing_n=1) with a confirmed swing high at 26
    /// (index 7), a liquidity sweep of that high (index 9: wick to 27,
    /// close back to 25.5), and then a displacement leg down (indices
    /// 10..12) that leaves a bearish FVG — the manipulation and
    /// distribution phases an AMD cycle assembles from the two detectors
    /// that already found them.
    fn amd_fixture() -> Vec<SmcBar> {
        vec![
            bar(20.0, 22.0, 20.0, 21.0), // 0
            bar(18.0, 19.0, 17.0, 18.0), // 1  swing low L1=17
            bar(20.0, 22.0, 20.0, 21.0), // 2
            bar(23.0, 24.0, 22.0, 23.0), // 3  swing high H1=24
            bar(21.0, 22.0, 20.0, 21.0), // 4
            bar(19.0, 20.0, 18.0, 19.0), // 5  swing low L2=18
            bar(21.0, 22.0, 20.0, 21.0), // 6
            bar(25.0, 26.0, 24.0, 25.0), // 7  swing high H2=26
            bar(21.0, 22.0, 20.0, 21.0), // 8  H2 confirmed; trend Up
            bar(25.0, 27.0, 24.0, 25.5), // 9  manipulation: sweep of H2 (wick 27, close 25.5)
            bar(25.0, 25.5, 19.0, 20.0), // 10 distribution leg starts (down-close)
            bar(20.0, 20.5, 9.0, 10.0),  // 11 distribution continues
            bar(10.0, 10.5, 0.5, 1.0),   // 12 distribution ends — bearish FVG vs bar 10
        ]
    }

    #[test]
    fn amd_cycle_parts_are_identical_to_the_underlying_detector_outputs() {
        let bars = amd_fixture();
        let cfg = SmcConfig { swing_n: 1, atr_period: 3, ..SmcConfig::default() };

        let sweeps = liquidity_sweeps(&bars, &cfg);
        let sweep = *sweeps.iter().find(|s| s.index == 9).expect("expected a sweep at bar 9");

        let legs = displacement_legs(&bars, &cfg);
        let leg = *legs
            .iter()
            .find(|l| l.start_index == 10 && l.direction == Direction::Down)
            .expect("expected a down displacement leg starting at bar 10");

        let cycles = amd_cycles(&bars, &cfg);
        let cycle = cycles
            .iter()
            .find(|c| c.manipulation.index == 9)
            .expect("expected an AMD cycle built from the bar-9 sweep");

        assert_eq!(cycle.manipulation, sweep, "manipulation must be identical to the sweep detector's own output");
        assert_eq!(cycle.distribution, leg, "distribution must be identical to the displacement detector's own output");
        assert_eq!(cycle.accumulation, 7..9);
    }

    /// A displacement leg that arrives after the lag bound is not this
    /// sweep's distribution — the cycle is simply not assembled, rather
    /// than assembled from whatever leg happened to come next.
    #[test]
    fn a_late_displacement_leg_does_not_become_this_sweeps_distribution() {
        let bars = amd_fixture();
        let cfg = SmcConfig { swing_n: 1, atr_period: 3, ..SmcConfig::default() };

        // The fixture's distribution leg starts at bar 10, one bar after the
        // bar-9 sweep, so any lag bound below 1 must reject it.
        let tight = SmcConfig { amd_max_distribution_lag: 0, ..cfg };
        assert!(
            !amd_cycles(&bars, &tight).iter().any(|c| c.manipulation.index == 9),
            "a leg starting one bar past the sweep must not be claimed under a zero-bar lag bound"
        );
        assert!(
            amd_cycles(&bars, &cfg).iter().any(|c| c.manipulation.index == 9),
            "the same leg is claimed under the default bound"
        );
    }
}
