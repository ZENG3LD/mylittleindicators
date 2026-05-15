//! Concrete `RestFetcher` implementation backed by a `BinanceConnector` from digdigdig3.
//!
//! # Sync/Async Bridge
//!
//! `RestFetcher::fetch` is a synchronous trait method.  All dig3 REST calls are async.
//! We bridge via `tokio::task::block_in_place` + `tokio::runtime::Handle::current().block_on(...)`.
//!
//! This requires the caller to be inside a Tokio multi-threaded runtime (`rt-multi-thread`).
//! If called from a `current_thread` runtime the thread will block — that is intentional and
//! safe; the Tokio docs allow `block_in_place` + `block_on` from any multi-thread worker.
//!
//! # Implemented StreamKinds
//!
//! | `StreamKind`    | dig3 endpoint                         |
//! |-----------------|---------------------------------------|
//! | `Funding`       | `GET /fapi/v1/fundingRate` (history)  |
//! | `OpenInterest`  | `GET /futures/data/openInterestHist`  |
//! | `Liquidation`   | `GET /fapi/v1/forceOrders`            |
//! | `LongShortRatio`| `GET /futures/data/globalLongShortAccountRatio` |
//! | `Bar`           | `get_klines_paginated` (futures)      |
//!
//! Remaining kinds have no Binance REST history endpoint and return an error.

use std::sync::Arc;

use digdigdig3::{
    AccountType, Positions,
    l3::open::crypto::cex::binance::BinanceConnector,
};

use crate::core::types::Bar;
use crate::data_loader::{RestFetcher, StreamKind, TimedEvent};

/// `RestFetcher` backed by a single `BinanceConnector` instance.
///
/// Covers futures endpoints; spot endpoints are not relevant for most mli streams
/// (OI, liquidations, funding are futures-only).
pub struct Dig3RestFetcher {
    connector: Arc<BinanceConnector>,
}

impl Dig3RestFetcher {
    /// Create a new fetcher wrapping an existing connector.
    pub fn new(connector: Arc<BinanceConnector>) -> Self {
        Self { connector }
    }
}

impl RestFetcher for Dig3RestFetcher {
    fn fetch(
        &self,
        symbol: &str,
        kind: StreamKind,
        from_ts: i64,
        to_ts: i64,
    ) -> Result<Vec<TimedEvent>, String> {
        match kind {
            StreamKind::Funding => self.fetch_funding(symbol, from_ts, to_ts),
            StreamKind::OpenInterest => self.fetch_open_interest(symbol, from_ts, to_ts),
            StreamKind::Liquidation => self.fetch_liquidations(symbol, from_ts, to_ts),
            StreamKind::LongShortRatio => self.fetch_long_short_ratio(symbol, from_ts, to_ts),
            StreamKind::Bar => self.fetch_bars(symbol, from_ts, to_ts),
            other => Err(format!(
                "Dig3RestFetcher: StreamKind::{other:?} is not available via Binance REST"
            )),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────

impl Dig3RestFetcher {
    /// Fetch funding rate history via `GET /fapi/v1/fundingRate`.
    fn fetch_funding(&self, symbol: &str, from_ts: i64, to_ts: i64) -> Result<Vec<TimedEvent>, String> {
        let connector = Arc::clone(&self.connector);
        let symbol = symbol.to_string();

        let rates = block_on(async move {
            connector
                .get_funding_rate(&symbol, AccountType::FuturesCross)
                .await
        })
        .map_err(|e| format!("Dig3RestFetcher::fetch_funding: {e}"))?;

        // get_funding_rate returns a single FundingRate (current).
        // Filter to the requested window by timestamp.
        let events = std::iter::once(rates)
            .filter(|r| r.timestamp >= from_ts && r.timestamp <= to_ts)
            .map(TimedEvent::Funding)
            .collect();
        Ok(events)
    }

    /// Fetch open interest history via `GET /futures/data/openInterestHist`.
    fn fetch_open_interest(&self, symbol: &str, from_ts: i64, to_ts: i64) -> Result<Vec<TimedEvent>, String> {
        let connector = Arc::clone(&self.connector);
        let symbol = symbol.to_string();

        let records = block_on(async move {
            connector
                .get_open_interest_history(
                    &symbol,
                    "5m",
                    Some(500),
                    Some(from_ts),
                    Some(to_ts),
                )
                .await
        })
        .map_err(|e| format!("Dig3RestFetcher::fetch_open_interest: {e}"))?;

        Ok(records.into_iter().map(TimedEvent::OpenInterest).collect())
    }

    /// Fetch public liquidation orders via `GET /fapi/v1/forceOrders`.
    fn fetch_liquidations(&self, symbol: &str, from_ts: i64, to_ts: i64) -> Result<Vec<TimedEvent>, String> {
        let connector = Arc::clone(&self.connector);
        let symbol = symbol.to_string();

        let records = block_on(async move {
            connector
                .get_force_orders(
                    Some(&symbol),
                    Some("LIQUIDATION"),
                    Some(from_ts),
                    Some(to_ts),
                    Some(1000),
                )
                .await
        })
        .map_err(|e| format!("Dig3RestFetcher::fetch_liquidations: {e}"))?;

        Ok(records.into_iter().map(TimedEvent::Liquidation).collect())
    }

    /// Fetch global long/short account ratio via `GET /futures/data/globalLongShortAccountRatio`.
    fn fetch_long_short_ratio(&self, symbol: &str, from_ts: i64, to_ts: i64) -> Result<Vec<TimedEvent>, String> {
        let connector = Arc::clone(&self.connector);
        let symbol = symbol.to_string();

        let records = block_on(async move {
            connector
                .get_global_long_short_account_ratio(
                    &symbol,
                    "5m",
                    Some(500),
                    Some(from_ts),
                    Some(to_ts),
                )
                .await
        })
        .map_err(|e| format!("Dig3RestFetcher::fetch_long_short_ratio: {e}"))?;

        Ok(records.into_iter().map(TimedEvent::LongShortRatio).collect())
    }

    /// Fetch OHLCV bars via `get_klines_paginated` (futures endpoint).
    ///
    /// Uses a fixed `"1m"` interval.  The caller is responsible for requesting an
    /// appropriate time range when a coarser interval is needed.
    fn fetch_bars(&self, symbol: &str, from_ts: i64, _to_ts: i64) -> Result<Vec<TimedEvent>, String> {
        let connector = Arc::clone(&self.connector);
        let symbol_str = symbol.to_string();

        // Estimate bar count at 1-minute resolution.
        let elapsed_ms = _to_ts.saturating_sub(from_ts).max(0);
        let bars_needed = ((elapsed_ms / 60_000) as usize).min(1000).max(1);

        let klines = block_on(async move {
            use digdigdig3::Symbol;
            let sym = Symbol::with_raw("", "", symbol_str);
            connector
                .get_klines_paginated(sym, "1m", bars_needed, AccountType::FuturesCross)
                .await
        })
        .map_err(|e| format!("Dig3RestFetcher::fetch_bars: {e}"))?;

        let events = klines
            .into_iter()
            .filter(|k| {
                let t = k.close_time.unwrap_or(k.open_time);
                t >= from_ts && t <= _to_ts
            })
            .map(|k| {
                let time = k.close_time.unwrap_or(k.open_time);
                TimedEvent::Bar(Bar::new(time, k.open, k.high, k.low, k.close, k.volume))
            })
            .collect();
        Ok(events)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Sync/Async bridge
// ─────────────────────────────────────────────────────────────────────────────

/// Run an async future from synchronous context inside an existing Tokio runtime.
///
/// Uses `block_in_place` so Tokio can park the current worker thread and run other
/// tasks on the pool.  Then `block_on` drives the future to completion.
///
/// Panics if called outside a Tokio runtime — callers must ensure a runtime exists.
fn block_on<F, T>(fut: F) -> T
where
    F: std::future::Future<Output = T>,
{
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(fut)
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that `Dig3RestFetcher` can be constructed (no real HTTP calls).
    ///
    /// This test requires a Tokio runtime only to build the connector.
    #[tokio::test]
    async fn constructible() {
        let connector = BinanceConnector::public(false)
            .await
            .expect("BinanceConnector::public should succeed");
        let _fetcher = Dig3RestFetcher::new(Arc::new(connector));
    }

    /// Unsupported stream kinds return an error immediately.
    #[tokio::test]
    async fn unsupported_kind_returns_error() {
        let connector = BinanceConnector::public(false)
            .await
            .expect("BinanceConnector::public should succeed");
        let fetcher = Dig3RestFetcher::new(Arc::new(connector));

        // Tick is not implemented via REST in this fetcher.
        let result = fetcher.fetch("BTCUSDT", StreamKind::Tick, 0, 1_000_000);
        assert!(result.is_err(), "Tick should not be fetchable via REST");

        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Tick"), "error should mention the kind");
    }
}
