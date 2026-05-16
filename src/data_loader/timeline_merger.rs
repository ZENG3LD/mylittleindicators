//! Multi-stream timestamp synchronization helpers.
//!
//! Merges events from multiple streams into a single timestamp-ordered timeline.
//! Used by `EnrichedDataLoader` when loading multiple streams for a single symbol,
//! and available as a standalone utility for downstream consumers.
//!
//! ## Functions
//!
//! - [`merge_sorted`] — merge N sorted streams into one timestamp-ordered vec.
//! - [`bar_boundaries`] — for each bar, find the index range of events within that bar.
//! - [`align_to_bars`] — apply a sampler function over events within each bar's window.

use super::TimedEvent;
use crate::core::types::Bar;

/// Merges multiple sorted event streams into a single timestamp-ordered `Vec`.
///
/// Each input `Vec` is expected to be sorted by `timestamp_ms()` ascending, but
/// the merge still produces a correct result if inputs are not pre-sorted (it falls
/// back to a full stable sort).
///
/// Stable sort: events with equal timestamps preserve their original stream order,
/// so bars always precede same-millisecond non-bar events when bars are added first.
///
/// # Example
/// ```
/// use mylittleindicators::data_loader::timeline_merger::merge_sorted;
/// use mylittleindicators::data_loader::TimedEvent;
/// use mylittleindicators::core::types::Bar;
///
/// let stream_a = vec![TimedEvent::Bar(Bar::new(1000, 1., 2., 0.5, 1.5, 100.))];
/// let stream_b = vec![TimedEvent::Bar(Bar::new(500,  1., 2., 0.5, 1.5, 100.))];
/// let merged = merge_sorted(vec![stream_a, stream_b]);
/// assert_eq!(merged[0].timestamp_ms(), 500);
/// assert_eq!(merged[1].timestamp_ms(), 1000);
/// ```
pub fn merge_sorted(streams: Vec<Vec<TimedEvent>>) -> Vec<TimedEvent> {
    let total = streams.iter().map(|s| s.len()).sum();
    let mut all: Vec<TimedEvent> = Vec::with_capacity(total);
    for s in streams {
        all.extend(s);
    }
    all.sort_by_key(|e| e.timestamp_ms());
    all
}

/// For each bar in `bars`, returns the index range of `events` falling in
/// `[bar.time, next_bar.time)`. The last bar's window extends to `i64::MAX`.
///
/// Assumes both `bars` and `events` are sorted by timestamp ascending.
/// Events at exactly `next_bar.time` belong to the *next* bar, not the current one.
///
/// Returns a `Vec<Range<usize>>` of length `bars.len()` (empty when `bars` is empty).
///
/// # Example
/// ```
/// use mylittleindicators::data_loader::timeline_merger::{bar_boundaries, merge_sorted};
/// use mylittleindicators::data_loader::TimedEvent;
/// use mylittleindicators::core::types::{Bar, FundingRate};
///
/// let bars = vec![
///     Bar::new(0,    1., 2., 0.5, 1.5, 100.),
///     Bar::new(1000, 1., 2., 0.5, 1.5, 100.),
/// ];
/// let events = vec![
///     TimedEvent::Funding(FundingRate { symbol: "X".into(), rate: 0.01, next_funding_time: None, timestamp: 500 }),
///     TimedEvent::Funding(FundingRate { symbol: "X".into(), rate: 0.01, next_funding_time: None, timestamp: 1500 }),
/// ];
/// let ranges = bar_boundaries(&bars, &events);
/// // bar 0 → events[0..1] (ts=500 in [0,1000))
/// assert_eq!(ranges[0], 0..1);
/// // bar 1 → events[1..2] (ts=1500 in [1000, MAX))
/// assert_eq!(ranges[1], 1..2);
/// ```
pub fn bar_boundaries(bars: &[Bar], events: &[TimedEvent]) -> Vec<std::ops::Range<usize>> {
    let mut result = Vec::with_capacity(bars.len());
    let mut event_idx = 0usize;

    for (i, _bar) in bars.iter().enumerate() {
        let next_bar_time = bars.get(i + 1).map(|b| b.time).unwrap_or(i64::MAX);
        let start = event_idx;
        while event_idx < events.len() && events[event_idx].timestamp_ms() < next_bar_time {
            event_idx += 1;
        }
        result.push(start..event_idx);
    }

    result
}

/// Applies `sampler` over the slice of events within each bar's time window,
/// producing one output value per bar.
///
/// `bars` and `events` must be sorted by timestamp ascending. The sampler receives
/// a sub-slice of events whose timestamps fall in `[bar.time, next_bar.time)`.
///
/// # Example — compute average funding rate per bar
/// ```
/// use mylittleindicators::data_loader::timeline_merger::align_to_bars;
/// use mylittleindicators::data_loader::TimedEvent;
/// use mylittleindicators::core::types::{Bar, FundingRate};
///
/// let bars = vec![Bar::new(0, 1., 2., 0.5, 1.5, 100.)];
/// let events = vec![
///     TimedEvent::Funding(FundingRate { symbol: "X".into(), rate: 0.01, next_funding_time: None, timestamp: 100 }),
///     TimedEvent::Funding(FundingRate { symbol: "X".into(), rate: 0.03, next_funding_time: None, timestamp: 200 }),
/// ];
/// let averages: Vec<f64> = align_to_bars(&bars, &events, |evs| {
///     let rates: Vec<f64> = evs.iter().filter_map(|e| {
///         if let TimedEvent::Funding(f) = e { Some(f.rate) } else { None }
///     }).collect();
///     if rates.is_empty() { 0.0 } else { rates.iter().sum::<f64>() / rates.len() as f64 }
/// });
/// assert!((averages[0] - 0.02).abs() < 1e-10);
/// ```
pub fn align_to_bars<F, T>(bars: &[Bar], events: &[TimedEvent], mut sampler: F) -> Vec<T>
where
    F: FnMut(&[TimedEvent]) -> T,
{
    let boundaries = bar_boundaries(bars, events);
    boundaries
        .iter()
        .map(|r| sampler(&events[r.clone()]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{Bar, FundingRate};
    use crate::data_loader::TimedEvent;

    fn make_bar(t: i64) -> Bar {
        Bar::new(t, 1.0, 2.0, 0.5, 1.5, 100.0)
    }

    fn make_funding(ts: i64) -> TimedEvent {
        TimedEvent::Funding(FundingRate {
            symbol: "BTCUSDT".into(),
            rate: 0.0001,
            next_funding_time: None,
            timestamp: ts,
        })
    }

    #[test]
    fn empty_merge() {
        let result = merge_sorted(vec![]);
        assert!(result.is_empty());
    }

    #[test]
    fn single_stream_passthrough() {
        let stream = vec![make_bar(1000), make_bar(2000), make_bar(3000)]
            .into_iter()
            .map(TimedEvent::Bar)
            .collect::<Vec<_>>();
        let merged = merge_sorted(vec![stream.clone()]);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].timestamp_ms(), 1000);
        assert_eq!(merged[2].timestamp_ms(), 3000);
    }

    #[test]
    fn multi_stream_merge() {
        let bars = vec![make_bar(1000), make_bar(3000)]
            .into_iter()
            .map(TimedEvent::Bar)
            .collect::<Vec<_>>();
        let funding = vec![make_funding(500), make_funding(2000), make_funding(4000)];

        let merged = merge_sorted(vec![bars, funding]);
        let timestamps: Vec<i64> = merged.iter().map(|e| e.timestamp_ms()).collect();
        assert_eq!(timestamps, vec![500, 1000, 2000, 3000, 4000]);
    }

    #[test]
    fn stable_ordering_equal_timestamps() {
        // Two events at the same timestamp — original stream order must be preserved.
        let stream_a = vec![TimedEvent::Bar(make_bar(1000))];
        let stream_b = vec![make_funding(1000)];

        let merged = merge_sorted(vec![stream_a, stream_b]);
        assert_eq!(merged.len(), 2);
        // Both at 1000; stream_a (Bar) was added first → should remain first (stable sort).
        assert!(matches!(merged[0], TimedEvent::Bar(_)));
        assert!(matches!(merged[1], TimedEvent::Funding(_)));
    }

    #[test]
    fn bar_boundaries_basic() {
        let bars = vec![make_bar(0), make_bar(1000), make_bar(2000)];
        let events = vec![
            make_funding(100),  // → bar 0 [0, 1000)
            make_funding(500),  // → bar 0
            make_funding(1000), // → bar 1 [1000, 2000) — exactly at boundary belongs to bar 1
            make_funding(1500), // → bar 1
            make_funding(2500), // → bar 2 [2000, MAX)
        ];

        let ranges = bar_boundaries(&bars, &events);
        assert_eq!(ranges.len(), 3);
        assert_eq!(ranges[0], 0..2); // ts=100, ts=500
        assert_eq!(ranges[1], 2..4); // ts=1000, ts=1500
        assert_eq!(ranges[2], 4..5); // ts=2500
    }

    #[test]
    fn bar_boundaries_empty_bars() {
        let bars: Vec<Bar> = vec![];
        let events = vec![make_funding(100)];
        let ranges = bar_boundaries(&bars, &events);
        assert!(ranges.is_empty());
    }

    #[test]
    fn align_to_bars_count_per_window() {
        let bars = vec![make_bar(0), make_bar(1000)];
        let events = vec![make_funding(100), make_funding(200), make_funding(1500)];
        let counts: Vec<usize> = align_to_bars(&bars, &events, |evs| evs.len());
        assert_eq!(counts, vec![2, 1]);
    }
}
