//! Collector configuration deserialized from TOML.
//!
//! Multi-exchange layout: each `ExchangeConfig` block has its own `id` + `account_types`
//! and a list of `SubscriptionConfig` entries (symbol + account_type + stream_type).
//!
//! `ExchangeId` and `AccountType` do NOT have built-in `serde::Deserialize` that maps
//! "Binance" → `ExchangeId::Binance` from TOML string literals. We use local wrapper
//! types that accept the string form and convert via `ExchangeId::from_str` at runtime.

use std::path::PathBuf;

use digdigdig3::{AccountType, ExchangeId, StreamType};
use serde::Deserialize;

// ─────────────────────────────────────────────────────────────────────────────
// Top-level config
// ─────────────────────────────────────────────────────────────────────────────

/// Top-level collector config.
#[derive(Debug, Clone, Deserialize)]
pub struct CollectorConfig {
    /// Root directory where binary stream files are written.
    pub storage_dir: PathBuf,
    /// Per-exchange subscription blocks.
    pub exchanges: Vec<ExchangeConfig>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-exchange config
// ─────────────────────────────────────────────────────────────────────────────

/// Config for one exchange — REST + WS account types + subscriptions.
#[derive(Debug, Clone, Deserialize)]
pub struct ExchangeConfig {
    /// Exchange id as lowercase string, e.g. `"binance"`, `"bybit"`.
    pub id: ExchangeIdStr,
    /// Account types to wire (REST + WS) on `connect_full`.
    pub account_types: Vec<AccountTypeStr>,
    /// Stream subscriptions for this exchange.
    pub subscriptions: Vec<SubscriptionConfig>,
}

impl ExchangeConfig {
    /// Parsed `ExchangeId`.
    pub fn exchange_id(&self) -> Option<ExchangeId> {
        ExchangeId::from_str(&self.id.0)
    }

    /// Parsed `AccountType` list (unknown strings are silently dropped).
    pub fn parsed_account_types(&self) -> Vec<AccountType> {
        self.account_types.iter().filter_map(|a| a.parse()).collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-subscription config
// ─────────────────────────────────────────────────────────────────────────────

/// Config for one WS subscription.
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionConfig {
    pub symbol: String,
    /// Account type for this subscription, e.g. `"FuturesCross"`.
    pub account_type: AccountTypeStr,
    /// Stream type, e.g. `"FundingRate"`, `"Liquidation"`.
    pub stream_type: StreamTypeStr,
}

impl SubscriptionConfig {
    pub fn parsed_account_type(&self) -> Option<AccountType> {
        self.account_type.parse()
    }

    pub fn parsed_stream_type(&self) -> Option<StreamType> {
        self.stream_type.parse()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// String wrapper types with parse helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Thin newtype over `String` for `ExchangeId` deserialization.
#[derive(Debug, Clone, Deserialize)]
pub struct ExchangeIdStr(pub String);

/// Thin newtype over `String` for `AccountType` deserialization.
#[derive(Debug, Clone, Deserialize)]
pub struct AccountTypeStr(pub String);

impl AccountTypeStr {
    /// Case-insensitive parse to `AccountType`.
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

/// Thin newtype over `String` for `StreamType` deserialization.
#[derive(Debug, Clone, Deserialize)]
pub struct StreamTypeStr(pub String);

impl StreamTypeStr {
    /// Case-insensitive parse to `StreamType`.
    pub fn parse(&self) -> Option<StreamType> {
        match self.0.to_lowercase().as_str() {
            "ticker" => Some(StreamType::Ticker),
            "trade" => Some(StreamType::Trade),
            "orderbook" => Some(StreamType::Orderbook),
            "orderbookdelta" | "orderbook_delta" => Some(StreamType::OrderbookDelta),
            "markprice" | "mark_price" => Some(StreamType::MarkPrice),
            "fundingrate" | "funding_rate" => Some(StreamType::FundingRate),
            "liquidation" => Some(StreamType::Liquidation),
            "openinterest" | "open_interest" => Some(StreamType::OpenInterest),
            "longshort" | "longshort_ratio" | "long_short_ratio" => Some(StreamType::LongShortRatio),
            "aggtrade" | "agg_trade" => Some(StreamType::AggTrade),
            "compositeindex" | "composite_index" => Some(StreamType::CompositeIndex),
            "indexPrice" | "index_price" => Some(StreamType::IndexPrice),
            "historicalvolatility" | "historical_volatility" => Some(StreamType::HistoricalVolatility),
            "insurancefund" | "insurance_fund" => Some(StreamType::InsuranceFund),
            "basis" => Some(StreamType::Basis),
            "optiongreeks" | "option_greeks" => Some(StreamType::OptionGreeks),
            "volatilityindex" | "volatility_index" => Some(StreamType::VolatilityIndex),
            "blocktrade" | "block_trade" => Some(StreamType::BlockTrade),
            "auctionevent" | "auction_event" => Some(StreamType::AuctionEvent),
            "marketwarning" | "market_warning" => Some(StreamType::MarketWarning),
            "orderbookl3" | "orderbook_l3" => Some(StreamType::OrderbookL3),
            "settlementevent" | "settlement_event" => Some(StreamType::SettlementEvent),
            "risklimit" | "risk_limit" => Some(StreamType::RiskLimit),
            "predictedfunding" | "predicted_funding" => Some(StreamType::PredictedFunding),
            "fundingsettlement" | "funding_settlement" => Some(StreamType::FundingSettlement),
            // Kline variants — require interval suffix, e.g. "kline:1m", "kline:5m".
            // Plain "kline" defaults to "1m".
            s if s == "kline" => Some(StreamType::Kline { interval: "1m".into() }),
            s if s.starts_with("kline:") => {
                let interval = s.trim_start_matches("kline:").to_string();
                Some(StreamType::Kline { interval })
            }
            s if s.starts_with("kline_") => {
                let interval = s.trim_start_matches("kline_").to_string();
                Some(StreamType::Kline { interval })
            }
            // Mark price kline, e.g. "markpricekline:1m".
            s if s == "markpricekline" || s == "mark_price_kline" => {
                Some(StreamType::MarkPriceKline { interval: "1m".into() })
            }
            s if s.starts_with("markpricekline:") || s.starts_with("mark_price_kline:") => {
                let interval = s.splitn(2, ':').nth(1).unwrap_or("1m").to_string();
                Some(StreamType::MarkPriceKline { interval })
            }
            // Index price kline, e.g. "indexpricekline:1m".
            s if s == "indexpricekline" || s == "index_price_kline" => {
                Some(StreamType::IndexPriceKline { interval: "1m".into() })
            }
            s if s.starts_with("indexpricekline:") || s.starts_with("index_price_kline:") => {
                let interval = s.splitn(2, ':').nth(1).unwrap_or("1m").to_string();
                Some(StreamType::IndexPriceKline { interval })
            }
            // Premium index kline, e.g. "premiumindexkline:1m".
            s if s == "premiumindexkline" || s == "premium_index_kline" => {
                Some(StreamType::PremiumIndexKline { interval: "1m".into() })
            }
            s if s.starts_with("premiumindexkline:") || s.starts_with("premium_index_kline:") => {
                let interval = s.splitn(2, ':').nth(1).unwrap_or("1m").to_string();
                Some(StreamType::PremiumIndexKline { interval })
            }
            _ => None,
        }
    }
}
