//! Trait for pluggable REST-based stream fetchers.
//!
//! Concrete implementations (e.g. via digdigdig3) are provided externally.
//! Pass an implementor via `EnrichedDataLoader::with_rest_fetcher(...)`.

use super::{StreamKind, TimedEvent};

/// Fetches historical events for a stream from a remote REST exchange API.
///
/// # Integration
/// Implement this trait against digdigdig3 or any other HTTP client.
/// Then pass the implementor via `EnrichedDataLoader::with_rest_fetcher(...)`.
///
/// # Current state
/// No built-in implementation exists — digdigdig3 integration is a future step.
pub trait RestFetcher: Send + Sync {
    /// Fetch events for `symbol`/`kind` in the closed timestamp range
    /// `[from_ts, to_ts]` (milliseconds).
    ///
    /// Returns a vec of `TimedEvent`, not necessarily sorted.
    fn fetch(
        &self,
        symbol: &str,
        kind: StreamKind,
        from_ts: i64,
        to_ts: i64,
    ) -> Result<Vec<TimedEvent>, String>;
}
