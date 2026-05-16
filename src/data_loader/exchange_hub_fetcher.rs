//! High-level REST fetcher using digdigdig3 `ExchangeHub`.
//!
//! Replaces `Dig3RestFetcher` (Binance-only, sync bridge). Works with any exchange
//! registered in the hub via the unified trait API — `MarketData`, `MarketDataPublic`,
//! and `Positions`.

use std::sync::Arc;

use async_trait::async_trait;
use digdigdig3::{
    AccountType, ExchangeId, Symbol,
    connector_manager::ExchangeHub,
};

use crate::core::types::Bar;
use crate::data_loader::{RestFetcher, StreamKind, TimedEvent};

/// `RestFetcher` backed by a digdigdig3 `ExchangeHub`.
///
/// Calls the appropriate trait method on the connector registered for
/// `(exchange, account_type)`. All methods are dispatched via `CoreConnector`
/// (which combines `MarketData` + `MarketDataPublic` + `Positions`).
pub struct ExchangeHubFetcher {
    hub: Arc<ExchangeHub>,
}

impl ExchangeHubFetcher {
    pub fn new(hub: Arc<ExchangeHub>) -> Self {
        Self { hub }
    }
}

#[async_trait]
impl RestFetcher for ExchangeHubFetcher {
    async fn fetch(
        &self,
        exchange: ExchangeId,
        account_type: AccountType,
        symbol: &str,
        kind: StreamKind,
        from_ts: i64,
        to_ts: i64,
    ) -> Result<Vec<TimedEvent>, String> {
        let conn = self
            .hub
            .rest(exchange)
            .ok_or_else(|| format!("ExchangeHubFetcher: no REST connector for {:?}", exchange))?;

        let sym = Symbol::with_raw("", "", symbol.to_string());

        match kind {
            StreamKind::Funding => {
                let rates = conn
                    .get_funding_rate_history(&sym, Some(from_ts), Some(to_ts), Some(1000), account_type)
                    .await
                    .map_err(|e| format!("get_funding_rate_history: {e}"))?;
                Ok(rates
                    .into_iter()
                    .filter(|r| r.timestamp >= from_ts && r.timestamp <= to_ts)
                    .map(TimedEvent::Funding)
                    .collect())
            }

            StreamKind::Liquidation => {
                let liqs = conn
                    .get_liquidation_history(Some(&sym), Some(from_ts), Some(to_ts), Some(1000), account_type)
                    .await
                    .map_err(|e| format!("get_liquidation_history: {e}"))?;
                Ok(liqs
                    .into_iter()
                    .filter(|l| l.timestamp >= from_ts && l.timestamp <= to_ts)
                    .map(TimedEvent::Liquidation)
                    .collect())
            }

            StreamKind::OpenInterest => {
                let history = conn
                    .get_open_interest_history(&sym, "5m", Some(from_ts), Some(to_ts), Some(500), account_type)
                    .await
                    .map_err(|e| format!("get_open_interest_history: {e}"))?;
                Ok(history
                    .into_iter()
                    .filter(|oi| oi.timestamp >= from_ts && oi.timestamp <= to_ts)
                    .map(TimedEvent::OpenInterest)
                    .collect())
            }

            StreamKind::LongShortRatio => {
                let ratios = conn
                    .get_long_short_ratio_history(&sym, "5m", Some(from_ts), Some(to_ts), Some(500), account_type)
                    .await
                    .map_err(|e| format!("get_long_short_ratio_history: {e}"))?;
                Ok(ratios
                    .into_iter()
                    .filter(|r| r.timestamp >= from_ts && r.timestamp <= to_ts)
                    .map(TimedEvent::LongShortRatio)
                    .collect())
            }

            StreamKind::Bar => {
                // get_klines takes end_time as upper bound; walk from to_ts backwards.
                // Estimate bar count at 1-minute resolution (capped at 1500).
                let elapsed_ms = to_ts.saturating_sub(from_ts).max(0);
                let limit = u16::try_from((elapsed_ms / 60_000).min(1500).max(1))
                    .unwrap_or(1500);

                let klines = conn
                    .get_klines(sym, "1m", Some(limit), account_type, Some(to_ts))
                    .await
                    .map_err(|e| format!("get_klines: {e}"))?;

                Ok(klines
                    .into_iter()
                    .filter(|k| k.open_time >= from_ts && k.open_time <= to_ts)
                    .map(|k| {
                        TimedEvent::Bar(Bar::new(
                            k.open_time,
                            k.open,
                            k.high,
                            k.low,
                            k.close,
                            k.volume,
                        ))
                    })
                    .collect())
            }

            other => Err(format!(
                "ExchangeHubFetcher: StreamKind::{other:?} not available via REST"
            )),
        }
    }
}
