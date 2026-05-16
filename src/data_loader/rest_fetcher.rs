//! Trait for pluggable REST-based stream fetchers.
//!
//! Concrete implementations (e.g. via digdigdig3 `ExchangeHub`) are provided externally.
//! Pass an implementor via `EnrichedDataLoader::with_rest_fetcher(...)`.

use async_trait::async_trait;
use digdigdig3::{AccountType, ExchangeId};

use super::{StreamKind, TimedEvent};

/// Fetches historical events for a stream from a remote REST exchange API.
///
/// # Integration
/// Implement this trait against digdigdig3 `ExchangeHub` or any other HTTP client.
/// Then pass the implementor via `EnrichedDataLoader::with_rest_fetcher(...)`.
#[async_trait]
pub trait RestFetcher: Send + Sync {
    /// Fetch events for `exchange`/`account_type`/`symbol`/`kind` in the closed
    /// timestamp range `[from_ts, to_ts]` (milliseconds).
    ///
    /// Returns a vec of `TimedEvent`, not necessarily sorted.
    async fn fetch(
        &self,
        exchange: ExchangeId,
        account_type: AccountType,
        symbol: &str,
        kind: StreamKind,
        from_ts: i64,
        to_ts: i64,
    ) -> Result<Vec<TimedEvent>, String>;
}
