//! Delta divergence: price/series divergence measured between consecutive
//! swing extremes of the same kind, built entirely from
//! `smc::market_structure::swings` (the one swing definition) and
//! `events::divergence::divergence_signal` (the one four-way comparison).

use crate::events::divergence::{divergence_signal, DivergenceKind};
use crate::core::signal::direction::Direction;
use crate::smc::market_structure::swings;
use crate::smc::{SmcBar, SmcConfig, Swing, SwingKind};

/// One divergence between price and `series`, found by comparing a pair of
/// consecutive swing extremes of the same kind (both swing highs or both
/// swing lows).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeltaDivergenceHit {
    /// Bar index of the EARLIER swing in the pair.
    pub from_index: usize,
    /// Bar index of the LATER swing — the bar the divergence is reported on.
    pub to_index: usize,
    pub kind: DivergenceKind,
    pub direction: Direction,
    /// The two price extremes compared.
    pub price_from: f64,
    pub price_to: f64,
    /// The two series values compared, at the same two bars.
    pub series_from: f64,
    pub series_to: f64,
}

/// Thresholds for `delta_divergences`.
#[derive(Debug, Clone, PartialEq)]
pub struct DeltaDivergenceConfig {
    /// Fractal half-width handed straight through to
    /// `smc::market_structure::swings`. Default matches
    /// `SmcConfig::default().swing_n` — this module defines no swing rule
    /// of its own, so its default cannot drift from `smc`'s.
    pub swing_n: usize,
    /// Which `DivergenceKind`s to report. Default: both — a regular and a
    /// hidden divergence can both be true of the same swing pair (they are
    /// different classifications of the same geometry), so reporting both
    /// by default loses nothing a caller can't filter back out.
    pub kinds: Vec<DivergenceKind>,
}

impl Default for DeltaDivergenceConfig {
    fn default() -> Self {
        Self {
            swing_n: SmcConfig::default().swing_n,
            kinds: vec![DivergenceKind::Regular, DivergenceKind::Hidden],
        }
    }
}

/// Price/series divergence between consecutive swing extremes.
///
/// `series` must be aligned 1:1 with `bars` (the chart's use case: `series`
/// is cumulative volume delta, one value per bar). A length mismatch
/// returns an empty vec rather than indexing past the end of either slice.
///
/// For each requested `DivergenceKind`, every consecutive pair of swing
/// highs is run through `divergence_signal` (the side that can yield a
/// bearish reading — price higher high vs. series lower high, or the
/// hidden-bearish mirror) and every consecutive pair of swing lows is run
/// the same way (the side that can yield a bullish reading). Pairs whose
/// price/series slopes agree — no divergence — are silently dropped by
/// `divergence_signal` returning `None`.
///
/// Results are sorted by `to_index`.
pub fn delta_divergences(
    bars: &[SmcBar],
    series: &[f64],
    cfg: &DeltaDivergenceConfig,
) -> Vec<DeltaDivergenceHit> {
    if series.len() != bars.len() {
        return Vec::new();
    }

    let smc_cfg = SmcConfig { swing_n: cfg.swing_n, ..SmcConfig::default() };
    let points = swings(bars, &smc_cfg);

    let mut out = Vec::new();
    for &kind in &cfg.kinds {
        for swing_kind in [SwingKind::High, SwingKind::Low] {
            let same_kind: Vec<&Swing> = points.iter().filter(|s| s.kind == swing_kind).collect();
            for pair in same_kind.windows(2) {
                let (from, to) = (pair[0], pair[1]);
                let Some(direction) = divergence_signal(
                    to.price,
                    from.price,
                    series[to.index],
                    series[from.index],
                    kind,
                ) else {
                    continue;
                };
                // A divergence is read off ONE side of the structure, and
                // which side is not a detail: the bearish readings — both
                // the regular one (price higher high, series lower high)
                // and the hidden one (price lower high, series higher high)
                // — are statements about swing HIGHS, and the bullish
                // readings are statements about swing LOWS. Running the
                // predicate over both sets and keeping whatever comes back
                // relabels each shape as its opposite: a lower high against
                // a higher high is hidden BEARISH, but read on the highs
                // under `Regular` the raw predicate answers "bullish".
                let side_agrees = match swing_kind {
                    SwingKind::High => direction == Direction::Down,
                    SwingKind::Low => direction == Direction::Up,
                };
                if !side_agrees {
                    continue;
                }
                out.push(DeltaDivergenceHit {
                    from_index: from.index,
                    to_index: to.index,
                    kind,
                    direction,
                    price_from: from.price,
                    price_to: to.price,
                    series_from: series[from.index],
                    series_to: series[to.index],
                });
            }
        }
    }

    out.sort_by_key(|h| h.to_index);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(o: f64, h: f64, l: f64, c: f64) -> SmcBar {
        SmcBar { open: o, high: h, low: l, close: c }
    }

    /// Two confirmed swing highs (idx1=20, idx3=25, rising) with one
    /// confirmed swing low (idx2=5) between them — `swing_n = 1`.
    fn rising_highs_fixture() -> Vec<SmcBar> {
        vec![
            bar(10.0, 10.0, 8.0, 9.0),   // 0 context
            bar(15.0, 20.0, 15.0, 18.0), // 1 swing high (20)
            bar(8.0, 10.0, 5.0, 7.0),    // 2 swing low (5)
            bar(15.0, 25.0, 15.0, 20.0), // 3 swing high (25)
            bar(10.0, 10.0, 8.0, 9.0),   // 4 context (confirms idx3)
        ]
    }

    /// Two confirmed swing lows (idx1=5, idx3=2, falling) with one
    /// confirmed swing high (idx2=25) between them — `swing_n = 1`.
    fn falling_lows_fixture() -> Vec<SmcBar> {
        vec![
            bar(20.0, 20.0, 10.0, 15.0), // 0 context
            bar(15.0, 15.0, 5.0, 10.0),  // 1 swing low (5)
            bar(20.0, 25.0, 10.0, 22.0), // 2 swing high (25)
            bar(15.0, 15.0, 2.0, 10.0),  // 3 swing low (2)
            bar(20.0, 20.0, 10.0, 15.0), // 4 context (confirms idx3)
        ]
    }

    #[test]
    fn regular_bearish_divergence_on_rising_swing_highs() {
        let bars = rising_highs_fixture();
        // Series falls while price's swing highs rise → regular bearish.
        let series = vec![0.0, 100.0, 0.0, 50.0, 0.0];
        let cfg = DeltaDivergenceConfig { swing_n: 1, kinds: vec![DivergenceKind::Regular] };

        let hits = delta_divergences(&bars, &series, &cfg);
        assert_eq!(hits.len(), 1, "expected exactly one regular divergence: {hits:?}");
        let hit = hits[0];
        assert_eq!(hit.from_index, 1);
        assert_eq!(hit.to_index, 3);
        assert_eq!(hit.kind, DivergenceKind::Regular);
        assert_eq!(hit.direction, Direction::Down);
        assert_eq!(hit.price_from, 20.0);
        assert_eq!(hit.price_to, 25.0);
        assert_eq!(hit.series_from, 100.0);
        assert_eq!(hit.series_to, 50.0);
    }

    #[test]
    fn regular_bullish_divergence_on_falling_swing_lows() {
        let bars = falling_lows_fixture();
        // Series rises while price's swing lows fall → regular bullish.
        let series = vec![0.0, 10.0, 0.0, 50.0, 0.0];
        let cfg = DeltaDivergenceConfig { swing_n: 1, kinds: vec![DivergenceKind::Regular] };

        let hits = delta_divergences(&bars, &series, &cfg);
        assert_eq!(hits.len(), 1, "expected exactly one regular divergence: {hits:?}");
        let hit = hits[0];
        assert_eq!(hit.from_index, 1);
        assert_eq!(hit.to_index, 3);
        assert_eq!(hit.kind, DivergenceKind::Regular);
        assert_eq!(hit.direction, Direction::Up);
        assert_eq!(hit.price_from, 5.0);
        assert_eq!(hit.price_to, 2.0);
        assert_eq!(hit.series_from, 10.0);
        assert_eq!(hit.series_to, 50.0);
    }

    /// Two confirmed swing lows (idx1=2, idx3=5, RISING) — the shape a
    /// hidden bullish divergence is read on, when the series makes the
    /// lower low instead.
    fn rising_lows_fixture() -> Vec<SmcBar> {
        vec![
            bar(20.0, 20.0, 10.0, 15.0), // 0 context
            bar(15.0, 15.0, 2.0, 10.0),  // 1 swing low (2)
            bar(20.0, 25.0, 10.0, 22.0), // 2 swing high (25)
            bar(15.0, 15.0, 5.0, 10.0),  // 3 swing low (5)
            bar(20.0, 20.0, 10.0, 15.0), // 4 context (confirms idx3)
        ]
    }

    /// A hidden bullish divergence is a statement about swing LOWS: price
    /// makes a HIGHER low while the series makes a LOWER low. The same pair
    /// of slopes read on the HIGHS is not hidden anything — it is a regular
    /// bearish divergence, and reporting it twice under two names would put
    /// one shape on the chart as two contradictory marks.
    #[test]
    fn hidden_divergence_is_read_off_the_lows() {
        let bars = rising_lows_fixture();
        // Series makes a LOWER low while price's swing lows rise.
        let series = vec![0.0, 100.0, 0.0, 50.0, 0.0];
        let cfg = DeltaDivergenceConfig { swing_n: 1, kinds: vec![DivergenceKind::Hidden] };

        let hits = delta_divergences(&bars, &series, &cfg);
        assert_eq!(hits.len(), 1, "expected exactly one hidden divergence: {hits:?}");
        let hit = hits[0];
        assert_eq!(hit.kind, DivergenceKind::Hidden);
        assert_eq!(hit.direction, Direction::Up);
        assert_eq!(hit.from_index, 1);
        assert_eq!(hit.to_index, 3);

        // The regular-bearish fixture (price highs rising, series falling)
        // is NOT a hidden divergence — scanned for Hidden it reports
        // nothing at all rather than the same shape under a second name.
        let regular_shape = rising_highs_fixture();
        let regular_series = vec![0.0, 100.0, 0.0, 50.0, 0.0];
        let none = delta_divergences(&regular_shape, &regular_series, &cfg);
        assert!(none.is_empty(), "a regular bearish shape is not also a hidden one: {none:?}");
    }

    #[test]
    fn series_shorter_than_bars_returns_empty_without_panic() {
        let bars = rising_highs_fixture();
        let series = vec![0.0, 100.0, 0.0]; // shorter than bars
        let cfg = DeltaDivergenceConfig::default();
        assert_eq!(delta_divergences(&bars, &series, &cfg), Vec::new());
    }

    #[test]
    fn fewer_than_two_swings_of_a_kind_produces_no_hit_for_that_kind() {
        // Only one swing high in the whole slice (idx1) — the swing low at
        // idx2 has no second low to pair with, and the single high has no
        // second high to pair with either: no pairs, no hits.
        let bars = vec![
            bar(10.0, 10.0, 8.0, 9.0),  // 0 context
            bar(15.0, 20.0, 15.0, 18.0), // 1 swing high (20)
            bar(8.0, 10.0, 5.0, 7.0),   // 2 swing low (5)
            bar(10.0, 10.0, 8.0, 9.0),  // 3 context
        ];
        let series = vec![0.0, 100.0, 50.0, 0.0];
        let cfg = DeltaDivergenceConfig { swing_n: 1, ..DeltaDivergenceConfig::default() };
        assert_eq!(delta_divergences(&bars, &series, &cfg), Vec::new());
    }

    /// Two confirmed swing highs (idx1=25, idx3=20, FALLING) — the shape
    /// a hidden bearish divergence is read on, when the series makes the
    /// higher high instead.
    fn falling_highs_fixture() -> Vec<SmcBar> {
        vec![
            bar(10.0, 10.0, 8.0, 9.0),   // 0 context
            bar(15.0, 25.0, 15.0, 20.0), // 1 swing high (25)
            bar(8.0, 10.0, 5.0, 7.0),    // 2 swing low (5)
            bar(15.0, 20.0, 15.0, 18.0), // 3 swing high (20)
            bar(10.0, 10.0, 8.0, 9.0),   // 4 context (confirms idx3)
        ]
    }

    /// A bearish shape read on swing HIGHS must never be reported as
    /// bullish. Price makes a LOWER high while the series makes a HIGHER
    /// high — hidden bearish. Fed to the bare predicate under `Regular`,
    /// that same pair of slopes answers "bullish", so the side check is
    /// what keeps the label honest: nothing is reported for `Regular`
    /// here, and the `Hidden` reading comes back as `Down`.
    #[test]
    fn a_bearish_shape_on_the_highs_is_never_labelled_bullish() {
        let bars = falling_highs_fixture();
        // Series makes a HIGHER high while price's swing highs fall.
        let series = vec![0.0, 50.0, 0.0, 100.0, 0.0];

        let regular = delta_divergences(
            &bars,
            &series,
            &DeltaDivergenceConfig { swing_n: 1, kinds: vec![DivergenceKind::Regular] },
        );
        assert!(
            regular.is_empty(),
            "a lower high against a higher series high is not a regular divergence at all: {regular:?}"
        );

        let hidden = delta_divergences(
            &bars,
            &series,
            &DeltaDivergenceConfig { swing_n: 1, kinds: vec![DivergenceKind::Hidden] },
        );
        assert_eq!(hidden.len(), 1, "exactly the hidden bearish reading: {hidden:?}");
        assert_eq!(hidden[0].direction, Direction::Down);
        assert_eq!(hidden[0].from_index, 1);
        assert_eq!(hidden[0].to_index, 3);
    }
}
