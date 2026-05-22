//! Collector configuration deserialized from TOML.
//!
//! Multi-exchange layout: each `ExchangeConfig` block has its own `id` + `account_types`
//! and a list of `SubscriptionConfig` entries (symbol + account_type + stream_type).
//!
//! Streams resolve to `digdigdig3_station::Stream` (9 kinds supported by Station:
//! Trade, AggTrade, Kline, Ticker, Orderbook, MarkPrice, FundingRate, OpenInterest,
//! Liquidation). Other StreamType variants from raw dig3 (CompositeIndex, OptionGreeks,
//! Basis, etc.) are not collected by daemon — use mli-collector-smoke for audit of
//! exotic streams.

use std::path::PathBuf;

use digdigdig3::{AccountType, ExchangeId};
use digdigdig3::core::websocket::KlineInterval;
use digdigdig3_station::Stream;
use serde::Deserialize;

// ─────────────────────────────────────────────────────────────────────────────
// Top-level config
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct CollectorConfig {
    /// Root directory where Station-managed binary stream files are written.
    pub storage_dir: PathBuf,
    /// Number of historical points to seed from disk (or REST fallback) on
    /// subscribe BEFORE live stream takes over. 0 disables warm-start.
    #[serde(default = "default_warm_start")]
    pub warm_start: usize,
    /// Per-exchange subscription blocks.
    pub exchanges: Vec<ExchangeConfig>,
}

fn default_warm_start() -> usize { 0 }

// ─────────────────────────────────────────────────────────────────────────────
// Per-exchange config
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ExchangeConfig {
    /// Exchange id as lowercase string, e.g. `"binance"`, `"bybit"`.
    pub id: ExchangeIdStr,
    /// Stream subscriptions for this exchange. Station auto-wires REST + WS
    /// per (exchange, account_type) on subscribe — no need to pre-declare
    /// account_types at the exchange level.
    #[serde(default)]
    pub subscriptions: Vec<SubscriptionConfig>,
}

impl ExchangeConfig {
    pub fn exchange_id(&self) -> Option<ExchangeId> {
        ExchangeId::from_str(&self.id.0)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-subscription config
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionConfig {
    pub symbol: String,
    pub account_type: AccountTypeStr,
    pub stream_type: StreamTypeStr,
}

impl SubscriptionConfig {
    pub fn parsed_account_type(&self) -> Option<AccountType> {
        self.account_type.parse()
    }

    pub fn parsed_stream(&self) -> Option<Stream> {
        self.stream_type.parse()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// String wrapper types with parse helpers
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ExchangeIdStr(pub String);

#[derive(Debug, Clone, Deserialize)]
pub struct AccountTypeStr(pub String);

impl AccountTypeStr {
    pub fn parse(&self) -> Option<AccountType> {
        match self.0.to_lowercase().as_str() {
            "spot" => Some(AccountType::Spot),
            "margin" => Some(AccountType::Margin),
            "futurescross" | "futures_cross" => Some(AccountType::FuturesCross),
            "futuresisolated" | "futures_isolated" => Some(AccountType::FuturesIsolated),
            "earn" => Some(AccountType::Earn),
            "lending" => Some(AccountType::Lending),
            "options" => Some(AccountType::Options),
            "convert" => Some(AccountType::Convert),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamTypeStr(pub String);

impl StreamTypeStr {
    /// Case-insensitive parse to Station `Stream`. Returns None for kinds
    /// not supported by Station (e.g. CompositeIndex, OptionGreeks, Basis).
    pub fn parse(&self) -> Option<Stream> {
        let s = self.0.to_lowercase();
        match s.as_str() {
            "ticker" => Some(Stream::Ticker),
            "trade" => Some(Stream::Trade),
            "orderbook" => Some(Stream::Orderbook),
            "markprice" | "mark_price" => Some(Stream::MarkPrice),
            "fundingrate" | "funding_rate" => Some(Stream::FundingRate),
            "liquidation" => Some(Stream::Liquidation),
            "openinterest" | "open_interest" => Some(Stream::OpenInterest),
            "aggtrade" | "agg_trade" => Some(Stream::AggTrade),
            "kline" => Some(Stream::Kline(KlineInterval::new("1m"))),
            _ if s.starts_with("kline:") => {
                let iv = s.trim_start_matches("kline:");
                Some(Stream::Kline(KlineInterval::new(iv)))
            }
            _ if s.starts_with("kline_") => {
                let iv = s.trim_start_matches("kline_");
                Some(Stream::Kline(KlineInterval::new(iv)))
            }
            _ => None,
        }
    }
}
