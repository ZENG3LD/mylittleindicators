//! Merged historical timeline for backtest warmup.

use super::TimedEvent;
use crate::core::types::Bar;

/// Historical data needed to run backtest on multi-stream indicators.
///
/// Contains OHLCV bars (for bar-alignment / snapshot boundaries) plus
/// timestamp-ordered events from all requested streams.
#[derive(Debug, Clone)]
pub struct EnrichedHistory {
    /// OHLCV bars in chronological order.
    pub bars: Vec<Bar>,
    /// All events from all streams (incl. bars again as `TimedEvent::Bar`),
    /// sorted by timestamp ascending. Stable sort: events at the same timestamp
    /// preserve insertion order (so a bar boundary always comes before tick
    /// events in the same millisecond).
    pub events: Vec<TimedEvent>,
}

impl EnrichedHistory {
    /// Create a new `EnrichedHistory`.
    ///
    /// `events` must be sorted by timestamp ascending before calling this
    /// constructor (the loader does this automatically).
    pub fn new(bars: Vec<Bar>, events: Vec<TimedEvent>) -> Self {
        Self { bars, events }
    }

    /// Number of OHLCV bars.
    pub fn bar_count(&self) -> usize {
        self.bars.len()
    }

    /// Total number of events across all streams.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Returns the timestamp range `[first_ts, last_ts]` over bars, or `None` when empty.
    ///
    /// Timestamps are the `time` field of `Bar` in milliseconds.
    pub fn time_range(&self) -> Option<(i64, i64)> {
        let first = self.bars.first()?.time;
        let last = self.bars.last()?.time;
        Some((first, last))
    }
}

#[cfg(test)]
mod tests {
    use super::EnrichedHistory;
    use crate::core::types::Bar;
    use crate::data_loader::TimedEvent;

    fn make_bar(t: i64) -> Bar {
        Bar::new(t, 1.0, 2.0, 0.5, 1.5, 100.0)
    }

    #[test]
    fn empty_time_range_is_none() {
        let h = EnrichedHistory::new(vec![], vec![]);
        assert!(h.time_range().is_none());
    }

    #[test]
    fn counts_and_range() {
        let bars: Vec<Bar> = (0..5).map(|i| make_bar(i * 1000)).collect();
        let events: Vec<TimedEvent> = bars.iter().cloned().map(TimedEvent::Bar).collect();
        let h = EnrichedHistory::new(bars, events);
        assert_eq!(h.bar_count(), 5);
        assert_eq!(h.event_count(), 5);
        assert_eq!(h.time_range(), Some((0, 4000)));
    }
}
