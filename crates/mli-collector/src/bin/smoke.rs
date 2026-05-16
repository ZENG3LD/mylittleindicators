//! Full diagnostic matrix for dig3 ExchangeHub — Phase 1 REST audit + Phase 2 WS audit.
//!
//! Usage:
//!   mli-collector-smoke [ws_duration_secs]
//!   Default ws_duration_secs = 120.
//!
//! Phase 1: REST audit — calls every applicable public REST method for each exchange in PARALLEL.
//!          Each call has a 10s timeout (5s for Lighter). Derivative methods skipped for Spot.
//! Phase 2: WS audit (ws_duration_secs) — subscribes all supported streams, counts events.
//!          Subscribe calls have a 5s timeout each.
//!
//! Output:
//!   EXCHANGE STATUS MATRIX (per exchange × account_type × method/stream)
//!   STREAM AVAILABILITY MATRIX (per stream type — working / silent / failed / skipped)
//!   JSON: ./smoke_data/smoke_report.json

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use digdigdig3::{
    AccountType, ExchangeId, StreamEvent, StreamType, SubscriptionRequest, Symbol,
    connector_manager::ExchangeHub,
};
use futures_util::StreamExt;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

// ─────────────────────────────────────────────────────────────────────────────
// Timeouts
// ─────────────────────────────────────────────────────────────────────────────

const REST_CALL_TIMEOUT_SECS: u64 = 10;
/// Lighter connector is known to hang — use reduced timeout.
const LIGHTER_REST_TIMEOUT_SECS: u64 = 5;
const WS_SUBSCRIBE_TIMEOUT_SECS: u64 = 5;
const CONNECT_FULL_TIMEOUT_SECS: u64 = 15;

// ─────────────────────────────────────────────────────────────────────────────
// Exchange + account_type configuration
// ─────────────────────────────────────────────────────────────────────────────

fn exchanges_under_test() -> Vec<(ExchangeId, Vec<AccountType>)> {
    vec![
        (ExchangeId::Binance,     vec![AccountType::Spot, AccountType::FuturesCross]),
        (ExchangeId::Bybit,       vec![AccountType::Spot, AccountType::FuturesCross]),
        (ExchangeId::OKX,         vec![AccountType::Spot, AccountType::FuturesCross]),
        (ExchangeId::KuCoin,      vec![AccountType::Spot, AccountType::FuturesCross]),
        (ExchangeId::Bitget,      vec![AccountType::Spot, AccountType::FuturesCross]),
        (ExchangeId::GateIO,      vec![AccountType::Spot, AccountType::FuturesCross]),
        (ExchangeId::HTX,         vec![AccountType::Spot, AccountType::FuturesCross]),
        (ExchangeId::Deribit,     vec![AccountType::Options]),
        (ExchangeId::HyperLiquid, vec![AccountType::FuturesCross]),
        (ExchangeId::BingX,       vec![AccountType::Spot, AccountType::FuturesCross]),
        (ExchangeId::Kraken,      vec![AccountType::Spot]),
        (ExchangeId::MEXC,        vec![AccountType::Spot, AccountType::FuturesCross]),
        (ExchangeId::Coinbase,    vec![AccountType::Spot]),
        (ExchangeId::CryptoCom,   vec![AccountType::Spot]),
        (ExchangeId::Bitfinex,    vec![AccountType::Spot]),
        (ExchangeId::Bitstamp,    vec![AccountType::Spot]),
        (ExchangeId::Gemini,      vec![AccountType::Spot]),
        (ExchangeId::Upbit,       vec![AccountType::Spot]),
        (ExchangeId::Dydx,        vec![AccountType::FuturesCross]),
        // Lighter excluded from REST audit due to known dig3 hang bug (ws only, 5s timeout).
        (ExchangeId::Lighter,     vec![AccountType::FuturesCross]),
    ]
}

fn all_public_stream_types() -> Vec<StreamType> {
    vec![
        StreamType::Ticker,
        StreamType::Trade,
        StreamType::Orderbook,
        StreamType::OrderbookDelta,
        StreamType::Kline { interval: "1m".to_string() },
        StreamType::MarkPrice,
        StreamType::FundingRate,
        StreamType::Liquidation,
        StreamType::OpenInterest,
        StreamType::LongShortRatio,
        StreamType::AggTrade,
        StreamType::CompositeIndex,
        StreamType::MarkPriceKline { interval: "1m".to_string() },
        StreamType::IndexPriceKline { interval: "1m".to_string() },
        StreamType::PremiumIndexKline { interval: "1m".to_string() },
        StreamType::IndexPrice,
        StreamType::HistoricalVolatility,
        StreamType::InsuranceFund,
        StreamType::Basis,
        StreamType::OptionGreeks,
        StreamType::VolatilityIndex,
        StreamType::BlockTrade,
        StreamType::AuctionEvent,
        StreamType::MarketWarning,
        StreamType::OrderbookL3,
        StreamType::SettlementEvent,
        StreamType::RiskLimit,
        StreamType::PredictedFunding,
        StreamType::FundingSettlement,
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Symbol selection
//
// Returns Vec<Symbol> using Symbol::new(base, quote) — connectors reconstruct
// the exchange-native string via format_symbol(base, quote, account_type).
// Multiple candidates are used by the WS phase as fallback attempts.
// ─────────────────────────────────────────────────────────────────────────────

fn pick_symbols(exchange: ExchangeId, account: AccountType) -> Vec<Symbol> {
    match (exchange, account) {
        // Spot pairs — BTC/USDT
        (ExchangeId::Binance, AccountType::Spot) => {
            vec![Symbol::new("BTC", "USDT"), Symbol::new("ETH", "USDT")]
        }
        (ExchangeId::Binance, _) => vec![Symbol::new("BTC", "USDT")],
        (ExchangeId::Bybit, AccountType::Spot) => {
            vec![Symbol::new("BTC", "USDT"), Symbol::new("ETH", "USDT")]
        }
        (ExchangeId::Bybit, _) => vec![Symbol::new("BTC", "USDT")],
        (ExchangeId::OKX, _) => vec![Symbol::new("BTC", "USDT")],
        (ExchangeId::KuCoin, _) => vec![Symbol::new("BTC", "USDT")],
        // Coinbase — BTC/USD
        (ExchangeId::Coinbase, _) => vec![Symbol::new("BTC", "USD"), Symbol::new("ETH", "USD")],
        // Bitfinex — BTC/USD
        (ExchangeId::Bitfinex, _) => vec![Symbol::new("BTC", "USD"), Symbol::new("ETH", "USD")],
        // Kraken — BTC/USD
        (ExchangeId::Kraken, _) => vec![Symbol::new("BTC", "USD")],
        // Deribit — BTC perpetual (connector formats BTC-PERPETUAL from base=BTC quote=USD)
        (ExchangeId::Deribit, _) => vec![Symbol::new("BTC", "USD")],
        // HyperLiquid — bare base asset
        (ExchangeId::HyperLiquid, _) => vec![Symbol::new("BTC", "USD")],
        // Lighter — BTC/USDC (connector formats as bare base)
        (ExchangeId::Lighter, _) => vec![Symbol::new("BTC", "USDC")],
        // dYdX — BTC/USD
        (ExchangeId::Dydx, _) => vec![Symbol::new("BTC", "USD")],
        // Bitstamp — BTC/USD
        (ExchangeId::Bitstamp, _) => vec![Symbol::new("BTC", "USD")],
        // Gemini — BTC/USD
        (ExchangeId::Gemini, _) => vec![Symbol::new("BTC", "USD")],
        // Upbit — BTC/KRW
        (ExchangeId::Upbit, _) => vec![Symbol::new("BTC", "KRW")],
        // MEXC
        (ExchangeId::MEXC, _) => vec![Symbol::new("BTC", "USDT")],
        // HTX
        (ExchangeId::HTX, _) => vec![Symbol::new("BTC", "USDT")],
        // GateIO
        (ExchangeId::GateIO, _) => vec![Symbol::new("BTC", "USDT")],
        // Bitget
        (ExchangeId::Bitget, _) => vec![Symbol::new("BTC", "USDT")],
        // BingX
        (ExchangeId::BingX, _) => vec![Symbol::new("BTC", "USDT")],
        // CryptoCom
        (ExchangeId::CryptoCom, _) => vec![Symbol::new("BTC", "USDT")],
        // Default
        _ => vec![Symbol::new("BTC", "USDT")],
    }
}

fn primary_symbol(exchange: ExchangeId, account: AccountType) -> Symbol {
    pick_symbols(exchange, account)
        .into_iter()
        .next()
        .unwrap_or_else(|| Symbol::new("BTC", "USDT"))
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-account REST method whitelist
//
// Derivative methods (funding, OI, liquidation, long/short, premium, mark)
// are semantically impossible on Spot/Margin accounts — skip without network call.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RestMethodKind {
    /// Available on all account types
    Universal,
    /// Only makes sense on futures/options/derivatives
    DerivativeOnly,
}

fn method_kind(method: &str) -> RestMethodKind {
    match method {
        "get_klines"
        | "get_ticker"
        | "get_orderbook"
        | "get_recent_trades" => RestMethodKind::Universal,
        _ => RestMethodKind::DerivativeOnly,
    }
}

fn is_derivative_account(account_type: AccountType) -> bool {
    matches!(
        account_type,
        AccountType::FuturesCross
            | AccountType::FuturesIsolated
            | AccountType::Options
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Capability check
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum CapabilityResult {
    Supported,
    NotSupported,
    Unknown,
}

fn capability_for_stream(
    hub: &ExchangeHub,
    id: ExchangeId,
    account: AccountType,
    st: &StreamType,
) -> (bool, CapabilityResult, Option<String>) {
    let is_futures_stream = matches!(
        st,
        StreamType::FundingRate
            | StreamType::MarkPrice
            | StreamType::Liquidation
            | StreamType::OpenInterest
            | StreamType::LongShortRatio
            | StreamType::CompositeIndex
            | StreamType::MarkPriceKline { .. }
            | StreamType::IndexPriceKline { .. }
            | StreamType::PremiumIndexKline { .. }
            | StreamType::IndexPrice
            | StreamType::InsuranceFund
            | StreamType::Basis
            | StreamType::OptionGreeks
            | StreamType::VolatilityIndex
            | StreamType::RiskLimit
            | StreamType::PredictedFunding
            | StreamType::FundingSettlement
            | StreamType::AuctionEvent
            | StreamType::SettlementEvent
    );
    if is_futures_stream && account == AccountType::Spot {
        return (false, CapabilityResult::NotSupported, Some("futures_stream_on_spot".into()));
    }

    let is_options_stream = matches!(
        st,
        StreamType::OptionGreeks | StreamType::VolatilityIndex | StreamType::HistoricalVolatility
    );
    if is_options_stream && !matches!(id, ExchangeId::Deribit) {
        return (false, CapabilityResult::NotSupported, Some("options_stream_non_deribit".into()));
    }

    let caps = match hub.capabilities(id) {
        Some(c) => c,
        None => return (true, CapabilityResult::Unknown, None),
    };

    let (cap_flag, field_name) = match st {
        StreamType::Trade => (Some(caps.has_ws_trades), "has_ws_trades"),
        StreamType::AggTrade => (Some(caps.has_ws_trades), "has_ws_trades"),
        StreamType::Orderbook | StreamType::OrderbookDelta | StreamType::OrderbookL3 => {
            (Some(caps.has_ws_orderbook), "has_ws_orderbook")
        }
        StreamType::Ticker => (Some(caps.has_ws_ticker), "has_ws_ticker"),
        StreamType::Kline { .. } => (Some(caps.has_ws_klines), "has_ws_klines"),
        StreamType::MarkPrice => (Some(caps.has_ws_mark_price), "has_ws_mark_price"),
        StreamType::FundingRate => (Some(caps.has_ws_funding_rate), "has_ws_funding_rate"),
        _ => (None, ""),
    };

    match cap_flag {
        Some(true) => (true, CapabilityResult::Supported, None),
        Some(false) => (
            false,
            CapabilityResult::NotSupported,
            Some(format!("capability_flag_{field_name}_false")),
        ),
        None => (true, CapabilityResult::Unknown, None),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// REST audit types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
enum RestStatus {
    Ok { item_count: usize },
    Empty,
    UnsupportedOperation,
    /// Method skipped — semantically impossible for this account type (e.g. funding on Spot).
    SkippedSpotNoDerivative,
    HttpError { code: Option<i64>, msg: String },
    Timeout,
    ParseError { msg: String },
    Other { msg: String },
}

fn classify_exchange_error(e: &digdigdig3::core::types::ExchangeError) -> RestStatus {
    use digdigdig3::core::types::ExchangeError;
    match e {
        ExchangeError::UnsupportedOperation(_) | ExchangeError::NotSupported(_) => {
            RestStatus::UnsupportedOperation
        }
        ExchangeError::Timeout(_) => RestStatus::Timeout,
        ExchangeError::Parse(msg) | ExchangeError::ParseError(msg) => {
            RestStatus::ParseError { msg: msg.clone() }
        }
        ExchangeError::Http(msg) => RestStatus::HttpError { code: None, msg: msg.clone() },
        ExchangeError::Api { code, message } => RestStatus::HttpError {
            code: Some(*code as i64),
            msg: message.clone(),
        },
        ExchangeError::RateLimitExceeded { message, .. } => RestStatus::HttpError {
            code: Some(429),
            msg: message.clone(),
        },
        other => RestStatus::Other { msg: format!("{other}") },
    }
}

#[derive(Debug, Clone, Serialize)]
struct RestEndpointResult {
    exchange: String,
    account_type: String,
    method: String,
    symbol: String,
    #[serde(flatten)]
    status: RestStatus,
    duration_ms: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// REST audit — single exchange
// ─────────────────────────────────────────────────────────────────────────────

async fn run_rest_audit_for_exchange(
    hub: Arc<ExchangeHub>,
    exchange_id: ExchangeId,
    account_types: Vec<AccountType>,
) -> Vec<RestEndpointResult> {
    let mut results = Vec::new();

    // Lighter is known to hang in dig3 — skip REST audit entirely, log warning.
    if exchange_id == ExchangeId::Lighter {
        tracing::warn!(
            "[REST] lighter: skipped REST audit — known dig3 hang bug ({}s timeout would fire)",
            LIGHTER_REST_TIMEOUT_SECS
        );
        return results;
    }

    let rest = match hub.rest(exchange_id) {
        Some(r) => r,
        None => return results,
    };

    for account_type in account_types {
        let symbol = primary_symbol(exchange_id, account_type);
        // sym_str for logging and for &str-based methods (get_open_interest, get_mark_price)
        let sym_str = format!("{:?}", symbol);
        let ex_str = exchange_id.as_str().to_string();
        let at_str = account_type.as_key_str().to_string();
        let derivative = is_derivative_account(account_type);

        // Macro: Vec-returning REST call with timeout + whitelist check
        macro_rules! rest_timed {
            ($method:literal, $future:expr) => {{
                if method_kind($method) == RestMethodKind::DerivativeOnly && !derivative {
                    results.push(RestEndpointResult {
                        exchange: ex_str.clone(),
                        account_type: at_str.clone(),
                        method: $method.to_string(),
                        symbol: sym_str.clone(),
                        status: RestStatus::SkippedSpotNoDerivative,
                        duration_ms: 0,
                    });
                } else {
                    tracing::debug!("[REST] {}/{}.{}({:?}) starting", ex_str, at_str, $method, symbol);
                    let t0 = Instant::now();
                    let timed = tokio::time::timeout(
                        Duration::from_secs(REST_CALL_TIMEOUT_SECS),
                        $future,
                    ).await;
                    let duration_ms = t0.elapsed().as_millis() as u64;
                    let status = match timed {
                        Err(_elapsed) => RestStatus::Timeout,
                        Ok(res) => match res {
                            Ok(v) => {
                                let n: usize = v.len();
                                if n == 0 { RestStatus::Empty } else { RestStatus::Ok { item_count: n } }
                            }
                            Err(e) => classify_exchange_error(&e),
                        },
                    };
                    tracing::info!(
                        "[REST] {}/{}.{}({:?}) -> {:?} ({}ms)",
                        ex_str, at_str, $method, symbol, status, duration_ms
                    );
                    results.push(RestEndpointResult {
                        exchange: ex_str.clone(),
                        account_type: at_str.clone(),
                        method: $method.to_string(),
                        symbol: sym_str.clone(),
                        status,
                        duration_ms,
                    });
                }
            }};
        }

        // Macro: single-item REST call with timeout + whitelist check
        macro_rules! rest_timed_single {
            ($method:literal, $future:expr) => {{
                if method_kind($method) == RestMethodKind::DerivativeOnly && !derivative {
                    results.push(RestEndpointResult {
                        exchange: ex_str.clone(),
                        account_type: at_str.clone(),
                        method: $method.to_string(),
                        symbol: sym_str.clone(),
                        status: RestStatus::SkippedSpotNoDerivative,
                        duration_ms: 0,
                    });
                } else {
                    tracing::debug!("[REST] {}/{}.{}({:?}) starting", ex_str, at_str, $method, symbol);
                    let t0 = Instant::now();
                    let timed = tokio::time::timeout(
                        Duration::from_secs(REST_CALL_TIMEOUT_SECS),
                        $future,
                    ).await;
                    let duration_ms = t0.elapsed().as_millis() as u64;
                    let status = match timed {
                        Err(_elapsed) => RestStatus::Timeout,
                        Ok(res) => match res {
                            Ok(_) => RestStatus::Ok { item_count: 1 },
                            Err(e) => classify_exchange_error(&e),
                        },
                    };
                    tracing::info!(
                        "[REST] {}/{}.{}({:?}) -> {:?} ({}ms)",
                        ex_str, at_str, $method, symbol, status, duration_ms
                    );
                    results.push(RestEndpointResult {
                        exchange: ex_str.clone(),
                        account_type: at_str.clone(),
                        method: $method.to_string(),
                        symbol: sym_str.clone(),
                        status,
                        duration_ms,
                    });
                }
            }};
        }

        // &str representation for methods that take &str (get_open_interest, get_mark_price)
        let symbol_concat = symbol.to_concat();

        rest_timed!(
            "get_klines",
            rest.get_klines(symbol.clone(), "1m", Some(100), account_type, None)
        );

        rest_timed_single!(
            "get_ticker",
            rest.get_ticker(symbol.clone(), account_type)
        );

        rest_timed_single!(
            "get_orderbook",
            rest.get_orderbook(symbol.clone(), Some(20), account_type)
        );

        rest_timed!(
            "get_recent_trades",
            rest.get_recent_trades(&symbol, Some(100), account_type)
        );

        rest_timed!(
            "get_funding_rate_history",
            rest.get_funding_rate_history(&symbol, None, None, Some(10), account_type)
        );

        rest_timed!(
            "get_liquidation_history",
            rest.get_liquidation_history(Some(&symbol), None, None, Some(10), account_type)
        );

        rest_timed!(
            "get_open_interest_history",
            rest.get_open_interest_history(&symbol, "5m", None, None, Some(10), account_type)
        );

        rest_timed!(
            "get_long_short_ratio_history",
            rest.get_long_short_ratio_history(&symbol, "5m", None, None, Some(10), account_type)
        );

        rest_timed!(
            "get_premium_index",
            rest.get_premium_index(Some(&symbol), account_type)
        );

        rest_timed_single!(
            "get_open_interest",
            rest.get_open_interest(&symbol_concat, account_type)
        );

        rest_timed_single!(
            "get_mark_price",
            rest.get_mark_price(&symbol_concat)
        );
    }

    results
}

// ─────────────────────────────────────────────────────────────────────────────
// REST audit — all exchanges in parallel via JoinSet
// ─────────────────────────────────────────────────────────────────────────────

async fn run_rest_audit(
    hub: Arc<ExchangeHub>,
    exchanges: &[(ExchangeId, Vec<AccountType>)],
) -> Vec<RestEndpointResult> {
    println!(
        "=== PHASE 1: REST audit ({} exchanges in parallel, {}s timeout per call) ===",
        exchanges.len(),
        REST_CALL_TIMEOUT_SECS
    );

    let mut join_set: JoinSet<Vec<RestEndpointResult>> = JoinSet::new();

    for (ex_id, account_types) in exchanges {
        let hub_clone = Arc::clone(&hub);
        let ex_id = *ex_id;
        let account_types = account_types.clone();
        join_set.spawn(run_rest_audit_for_exchange(hub_clone, ex_id, account_types));
    }

    let mut all_results: Vec<RestEndpointResult> = Vec::new();
    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(results) => all_results.extend(results),
            Err(e) => tracing::warn!("REST audit task panicked: {e}"),
        }
    }

    all_results
}

// ─────────────────────────────────────────────────────────────────────────────
// WS audit types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "ws_status")]
enum WsStatus {
    Skipped { reason: String },
    SubscribeFailed { error: String },
    Subscribed { events_received: u64 },
    SilentNoEvents,
    ConnectionDropped { error: String },
}

#[derive(Debug, Clone, Serialize)]
struct WsSubscriptionDetail {
    exchange: String,
    account_type: String,
    stream_type: String,
    symbol: String,
    capability: CapabilityResult,
    #[serde(flatten)]
    status: WsStatus,
    subscribed_at_ms: Option<u64>,
    first_event_at_ms: Option<u64>,
    last_event_at_ms: Option<u64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// WS event tracking
// ─────────────────────────────────────────────────────────────────────────────

struct WsEvent {
    exchange: ExchangeId,
    account: AccountType,
    stream_label: String,
    ts_ms: u64,
}

fn stream_type_label(st: &StreamType) -> String {
    match st {
        StreamType::Ticker => "ticker".to_string(),
        StreamType::Trade => "trade".to_string(),
        StreamType::Orderbook => "orderbook".to_string(),
        StreamType::OrderbookDelta => "orderbook_delta".to_string(),
        StreamType::Kline { interval } => format!("kline:{interval}"),
        StreamType::MarkPrice => "mark_price".to_string(),
        StreamType::FundingRate => "funding_rate".to_string(),
        StreamType::Liquidation => "liquidation".to_string(),
        StreamType::OpenInterest => "open_interest".to_string(),
        StreamType::LongShortRatio => "long_short_ratio".to_string(),
        StreamType::AggTrade => "agg_trade".to_string(),
        StreamType::CompositeIndex => "composite_index".to_string(),
        StreamType::MarkPriceKline { interval } => format!("mark_price_kline:{interval}"),
        StreamType::IndexPriceKline { interval } => format!("index_price_kline:{interval}"),
        StreamType::PremiumIndexKline { interval } => format!("premium_index_kline:{interval}"),
        StreamType::IndexPrice => "index_price".to_string(),
        StreamType::HistoricalVolatility => "historical_volatility".to_string(),
        StreamType::InsuranceFund => "insurance_fund".to_string(),
        StreamType::Basis => "basis".to_string(),
        StreamType::OptionGreeks => "option_greeks".to_string(),
        StreamType::VolatilityIndex => "volatility_index".to_string(),
        StreamType::BlockTrade => "block_trade".to_string(),
        StreamType::AuctionEvent => "auction_event".to_string(),
        StreamType::MarketWarning => "market_warning".to_string(),
        StreamType::OrderbookL3 => "orderbook_l3".to_string(),
        StreamType::SettlementEvent => "settlement_event".to_string(),
        StreamType::RiskLimit => "risk_limit".to_string(),
        StreamType::PredictedFunding => "predicted_funding".to_string(),
        StreamType::FundingSettlement => "funding_settlement".to_string(),
        StreamType::OrderUpdate => "order_update".to_string(),
        StreamType::BalanceUpdate => "balance_update".to_string(),
        StreamType::PositionUpdate => "position_update".to_string(),
    }
}

fn stream_label_of_event(ev: &StreamEvent) -> &'static str {
    match ev {
        StreamEvent::Ticker(_) => "ticker",
        StreamEvent::Trade(_) => "trade",
        StreamEvent::OrderbookSnapshot(_) => "orderbook",
        StreamEvent::OrderbookDelta(_) => "orderbook_delta",
        StreamEvent::Kline(_) => "kline:1m",
        StreamEvent::MarkPrice { .. } => "mark_price",
        StreamEvent::FundingRate { .. } => "funding_rate",
        StreamEvent::Liquidation { .. } => "liquidation",
        StreamEvent::OpenInterestUpdate { .. } => "open_interest",
        StreamEvent::LongShortRatio { .. } => "long_short_ratio",
        StreamEvent::AggTrade { .. } => "agg_trade",
        StreamEvent::CompositeIndex { .. } => "composite_index",
        StreamEvent::MarkPriceKline { .. } => "mark_price_kline:1m",
        StreamEvent::IndexPriceKline { .. } => "index_price_kline:1m",
        StreamEvent::PremiumIndexKline { .. } => "premium_index_kline:1m",
        StreamEvent::IndexPrice { .. } => "index_price",
        StreamEvent::HistoricalVolatility { .. } => "historical_volatility",
        StreamEvent::InsuranceFund { .. } => "insurance_fund",
        StreamEvent::Basis { .. } => "basis",
        StreamEvent::OptionGreeks { .. } => "option_greeks",
        StreamEvent::VolatilityIndex { .. } => "volatility_index",
        StreamEvent::BlockTrade { .. } => "block_trade",
        StreamEvent::AuctionEvent { .. } => "auction_event",
        StreamEvent::MarketWarning { .. } => "market_warning",
        StreamEvent::OrderbookL3 { .. } => "orderbook_l3",
        StreamEvent::SettlementEvent { .. } => "settlement_event",
        StreamEvent::RiskLimit { .. } => "risk_limit",
        StreamEvent::PredictedFunding { .. } => "predicted_funding",
        StreamEvent::FundingSettlement { .. } => "funding_settlement",
        StreamEvent::OrderUpdate(_) => "order_update",
        StreamEvent::BalanceUpdate(_) => "balance_update",
        StreamEvent::PositionUpdate(_) => "position_update",
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─────────────────────────────────────────────────────────────────────────────
// WS subscribe phase — parallel across all (exchange, account_type) pairs
// ─────────────────────────────────────────────────────────────────────────────

async fn subscribe_all_streams(
    hub: &ExchangeHub,
    exchanges: &[(ExchangeId, Vec<AccountType>)],
    stream_types: &[StreamType],
) -> Vec<WsSubscriptionDetail> {
    let mut join_set: JoinSet<Vec<WsSubscriptionDetail>> = JoinSet::new();

    // Pre-collect all (exchange, account_type) data needed — no references escape into tasks
    struct PairConfig {
        exchange_id: ExchangeId,
        account_type: AccountType,
        sym_log: String,
        ex_str: String,
        at_str: String,
        symbols: Vec<Symbol>,
        stream_configs: Vec<(String, CapabilityResult, Option<String>, bool)>,
        stream_types_owned: Vec<StreamType>,
        ws: Option<Arc<dyn digdigdig3::core::traits::WebSocketConnector>>,
    }

    let mut pair_configs: Vec<PairConfig> = Vec::new();
    let mut no_ws_details: Vec<WsSubscriptionDetail> = Vec::new();

    for (exchange_id, account_types) in exchanges {
        for &account_type in account_types {
            let sym_log = format!("{:?}", primary_symbol(*exchange_id, account_type));
            let ex_str = exchange_id.as_str().to_string();
            let at_str = account_type.as_key_str().to_string();

            // Lighter WS: use reduced timeout — log warning
            if *exchange_id == ExchangeId::Lighter {
                tracing::warn!(
                    "[WS] lighter/{}: using {}s timeout due to known dig3 hang bug",
                    at_str, LIGHTER_REST_TIMEOUT_SECS
                );
            }

            let ws_opt = hub.ws(*exchange_id, account_type);

            if ws_opt.is_none() {
                for st in stream_types {
                    let stream_label = stream_type_label(st);
                    let (_, cap_res, _) = capability_for_stream(hub, *exchange_id, account_type, st);
                    no_ws_details.push(WsSubscriptionDetail {
                        exchange: ex_str.clone(),
                        account_type: at_str.clone(),
                        stream_type: stream_label,
                        symbol: sym_log.clone(),
                        capability: cap_res,
                        status: WsStatus::Skipped { reason: "no_ws_connector".into() },
                        subscribed_at_ms: None,
                        first_event_at_ms: None,
                        last_event_at_ms: None,
                    });
                }
                continue;
            }

            let stream_configs: Vec<(String, CapabilityResult, Option<String>, bool)> =
                stream_types.iter().map(|st| {
                    let label = stream_type_label(st);
                    let (should_try, cap_res, skip_reason) =
                        capability_for_stream(hub, *exchange_id, account_type, st);
                    (label, cap_res, skip_reason, should_try)
                }).collect();

            pair_configs.push(PairConfig {
                exchange_id: *exchange_id,
                account_type,
                sym_log,
                ex_str,
                at_str,
                symbols: pick_symbols(*exchange_id, account_type),
                stream_configs,
                stream_types_owned: stream_types.to_vec(),
                ws: ws_opt,
            });
        }
    }

    for cfg in pair_configs {
        let PairConfig {
            exchange_id,
            account_type,
            sym_log,
            ex_str,
            at_str,
            symbols,
            stream_configs,
            stream_types_owned,
            ws,
        } = cfg;

        let ws = match ws {
            Some(w) => w,
            None => continue,
        };

        // Lighter uses a reduced per-subscribe timeout
        let sub_timeout = if exchange_id == ExchangeId::Lighter {
            Duration::from_secs(LIGHTER_REST_TIMEOUT_SECS)
        } else {
            Duration::from_secs(WS_SUBSCRIBE_TIMEOUT_SECS)
        };

        join_set.spawn(async move {
            let mut local: Vec<WsSubscriptionDetail> = Vec::new();

            for (i, st) in stream_types_owned.iter().enumerate() {
                let (ref label, ref cap_res, ref skip_reason, should_try) = stream_configs[i];

                if !should_try {
                    let reason = skip_reason.clone().unwrap_or_else(|| "capability_check".into());
                    local.push(WsSubscriptionDetail {
                        exchange: ex_str.clone(),
                        account_type: at_str.clone(),
                        stream_type: label.clone(),
                        symbol: sym_log.clone(),
                        capability: cap_res.clone(),
                        status: WsStatus::Skipped { reason },
                        subscribed_at_ms: None,
                        first_event_at_ms: None,
                        last_event_at_ms: None,
                    });
                    continue;
                }

                let mut subscribed = false;
                let mut last_error = String::new();
                let mut used_symbol_log = sym_log.clone();

                for sym_candidate in &symbols {
                    let req = SubscriptionRequest {
                        symbol: sym_candidate.clone(),
                        stream_type: st.clone(),
                        account_type,
                        depth: None,
                        update_speed_ms: None,
                    };
                    let sub_result = tokio::time::timeout(
                        sub_timeout,
                        ws.subscribe(req),
                    ).await;

                    match sub_result {
                        Err(_elapsed) => {
                            last_error = format!("subscribe timeout ({:.0}s)", sub_timeout.as_secs_f32());
                            tracing::debug!(
                                "{:?}/{:?}/{}: subscribe timed out with {:?}",
                                exchange_id, account_type, label, sym_candidate
                            );
                            break;
                        }
                        Ok(Ok(_)) => {
                            subscribed = true;
                            used_symbol_log = format!("{:?}", sym_candidate);
                            break;
                        }
                        Ok(Err(e)) => {
                            last_error = format!("{e}");
                            tracing::debug!(
                                "{:?}/{:?}/{}: subscribe failed with {:?}: {}",
                                exchange_id, account_type, label, sym_candidate, e
                            );
                        }
                    }
                }

                let subscribed_at = if subscribed { Some(now_ms()) } else { None };
                let status = if subscribed {
                    WsStatus::Subscribed { events_received: 0 }
                } else {
                    WsStatus::SubscribeFailed { error: last_error }
                };

                local.push(WsSubscriptionDetail {
                    exchange: ex_str.clone(),
                    account_type: at_str.clone(),
                    stream_type: label.clone(),
                    symbol: used_symbol_log,
                    capability: cap_res.clone(),
                    status,
                    subscribed_at_ms: subscribed_at,
                    first_event_at_ms: None,
                    last_event_at_ms: None,
                });
            }

            local
        });
    }

    let mut all: Vec<WsSubscriptionDetail> = no_ws_details;
    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(v) => all.extend(v),
            Err(e) => tracing::warn!("WS subscribe task panicked: {e}"),
        }
    }
    all
}

// ─────────────────────────────────────────────────────────────────────────────
// WS audit runner
// ─────────────────────────────────────────────────────────────────────────────

async fn run_ws_audit(
    hub: &ExchangeHub,
    exchanges: &[(ExchangeId, Vec<AccountType>)],
    stream_types: &[StreamType],
    ws_duration_secs: u64,
) -> Vec<WsSubscriptionDetail> {
    // Key: (exchange_str, account_str) → error message
    let mut ws_connect_failures: HashMap<(String, String), String> = HashMap::new();

    for (exchange_id, account_types) in exchanges {
        for &account_type in account_types {
            let ex_str = exchange_id.as_str().to_string();
            let at_str = account_type.as_key_str().to_string();
            if let Some(ws) = hub.ws(*exchange_id, account_type) {
                let connect_result = tokio::time::timeout(
                    Duration::from_secs(CONNECT_FULL_TIMEOUT_SECS),
                    ws.connect(account_type),
                ).await;
                match connect_result {
                    Err(_) => {
                        let msg = format!("ws.connect timed out ({}s)", CONNECT_FULL_TIMEOUT_SECS);
                        tracing::warn!("{}/{}: {}", ex_str, at_str, msg);
                        ws_connect_failures.insert((ex_str, at_str), msg);
                    }
                    Ok(Err(e)) => {
                        let msg = format!("{e}");
                        tracing::warn!("{}/{}: ws.connect failed: {}", ex_str, at_str, msg);
                        ws_connect_failures.insert((ex_str, at_str), msg);
                    }
                    Ok(Ok(_)) => tracing::debug!("{}/{}: ws connected", ex_str, at_str),
                }
            }
        }
    }

    // Subscribe all streams in parallel (with per-call timeout)
    let mut details = subscribe_all_streams(hub, exchanges, stream_types).await;

    // Override subscribe results with ConnectionDropped for pairs whose connect failed
    for d in &mut details {
        let key = (d.exchange.clone(), d.account_type.clone());
        if let Some(err) = ws_connect_failures.get(&key) {
            if !matches!(d.status, WsStatus::Skipped { .. }) {
                d.status = WsStatus::ConnectionDropped { error: err.clone() };
            }
        }
    }

    // Build key-to-idx map for event routing
    type SubKey = (String, String, String);
    let mut key_to_idx: HashMap<SubKey, usize> = HashMap::new();
    for (idx, d) in details.iter().enumerate() {
        let key = (d.exchange.clone(), d.account_type.clone(), d.stream_type.clone());
        key_to_idx.insert(key, idx);
    }

    tracing::info!(
        "WS subscriptions: {} total ({} subscribed, {} skipped, {} failed)",
        details.len(),
        details.iter().filter(|d| matches!(d.status, WsStatus::Subscribed { .. })).count(),
        details.iter().filter(|d| matches!(d.status, WsStatus::Skipped { .. })).count(),
        details.iter().filter(|d| matches!(d.status, WsStatus::SubscribeFailed { .. })).count(),
    );

    // Spawn event listeners for each connected WS
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<WsEvent>();

    for (exchange_id, account_types) in exchanges {
        for &account_type in account_types {
            if let Some(ws) = hub.ws(*exchange_id, account_type) {
                let tx = event_tx.clone();
                let id = *exchange_id;
                let at = account_type;
                let mut stream = ws.event_stream();
                tokio::spawn(async move {
                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(event) => {
                                let stream_label = stream_label_of_event(&event).to_string();
                                let ts = now_ms();
                                if tx
                                    .send(WsEvent {
                                        exchange: id,
                                        account: at,
                                        stream_label,
                                        ts_ms: ts,
                                    })
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::debug!("ws error {:?}/{:?}: {}", id, at, e);
                            }
                        }
                    }
                });
            }
        }
    }
    drop(event_tx);

    // Collect events for the given duration with periodic [WS Xsec] progress logs
    let deadline = tokio::time::Instant::now() + Duration::from_secs(ws_duration_secs);
    let mut report_interval = tokio::time::interval(Duration::from_secs(10));
    report_interval.tick().await; // skip immediate first tick

    let mut per_exchange_events: HashMap<String, u64> = HashMap::new();

    loop {
        tokio::select! {
            maybe_ev = event_rx.recv() => {
                let ev = match maybe_ev {
                    Some(e) => e,
                    None => break,
                };
                let key = (
                    ev.exchange.as_str().to_string(),
                    ev.account.as_key_str().to_string(),
                    ev.stream_label.clone(),
                );
                if let Some(&idx) = key_to_idx.get(&key) {
                    let d = &mut details[idx];
                    if d.first_event_at_ms.is_none() {
                        d.first_event_at_ms = Some(ev.ts_ms);
                    }
                    d.last_event_at_ms = Some(ev.ts_ms);
                    if let WsStatus::Subscribed { events_received } = &mut d.status {
                        *events_received += 1;
                    }
                }
                *per_exchange_events.entry(ev.exchange.as_str().to_string()).or_insert(0) += 1;
            }
            _ = report_interval.tick() => {
                let remaining = deadline.duration_since(tokio::time::Instant::now());
                let elapsed_secs = ws_duration_secs.saturating_sub(remaining.as_secs());
                let active = details.iter().filter(|d| {
                    matches!(&d.status, WsStatus::Subscribed { events_received } if *events_received > 0)
                }).count();

                let mut ex_parts: Vec<String> = per_exchange_events
                    .iter()
                    .map(|(ex, cnt)| format!("{} {}", ex, cnt))
                    .collect();
                ex_parts.sort();

                let silent_exs: Vec<String> = details
                    .iter()
                    .filter(|d| matches!(d.status, WsStatus::Subscribed { events_received: 0 }))
                    .map(|d| format!("{}:{}", d.exchange, d.account_type))
                    .collect::<std::collections::HashSet<_>>()
                    .into_iter()
                    .collect();

                tracing::info!(
                    "[WS {}s] received: {} | streams_with_data={} | silent={}",
                    elapsed_secs,
                    ex_parts.join(", "),
                    active,
                    silent_exs.join(", "),
                );
            }
            _ = tokio::time::sleep_until(deadline) => {
                tracing::info!("WS audit duration elapsed ({}s)", ws_duration_secs);
                break;
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Ctrl-C received during WS audit");
                break;
            }
        }
    }

    // Convert SilentNoEvents for subscribed streams with 0 events
    for d in &mut details {
        if let WsStatus::Subscribed { events_received: 0 } = d.status {
            d.status = WsStatus::SilentNoEvents;
        }
    }

    details
}

// ─────────────────────────────────────────────────────────────────────────────
// Report printing
// ─────────────────────────────────────────────────────────────────────────────

fn print_exchange_matrix(
    rest_results: &[RestEndpointResult],
    ws_details: &[WsSubscriptionDetail],
    exchanges: &[(ExchangeId, Vec<AccountType>)],
) {
    println!("\n{}", "=".repeat(70));
    println!("EXCHANGE STATUS MATRIX");
    println!("{}", "=".repeat(70));

    for (exchange_id, account_types) in exchanges {
        let ex_str = exchange_id.as_str();

        for &account_type in account_types {
            let at_str = account_type.as_key_str();
            println!("\n{}:{}", ex_str, at_str);

            let rest_rows: Vec<&RestEndpointResult> = rest_results
                .iter()
                .filter(|r| r.exchange == ex_str && r.account_type == at_str)
                .collect();

            if !rest_rows.is_empty() {
                println!("  REST:");
                for r in &rest_rows {
                    let detail = match &r.status {
                        RestStatus::Ok { item_count } => format!("{} items, {}ms", item_count, r.duration_ms),
                        RestStatus::Empty => format!("0 items, {}ms", r.duration_ms),
                        RestStatus::UnsupportedOperation => "unsupported".to_string(),
                        RestStatus::SkippedSpotNoDerivative => "skipped (spot, no derivative)".to_string(),
                        RestStatus::HttpError { code, msg } => {
                            let code_str = code.map(|c| format!("HTTP {c} ")).unwrap_or_default();
                            format!("{}{}", code_str, truncate(msg, 60))
                        }
                        RestStatus::Timeout => format!("TIMEOUT ({}s)", REST_CALL_TIMEOUT_SECS),
                        RestStatus::ParseError { msg } => format!("PARSE: {}", truncate(msg, 60)),
                        RestStatus::Other { msg } => truncate(msg, 60).to_string(),
                    };
                    println!("    {:<40} {}", r.method, detail);
                }
            }

            let ws_rows: Vec<&WsSubscriptionDetail> = ws_details
                .iter()
                .filter(|d| d.exchange == ex_str && d.account_type == at_str)
                .collect();

            if !ws_rows.is_empty() {
                println!("  WS subscriptions:");
                for d in &ws_rows {
                    let detail = match &d.status {
                        WsStatus::Subscribed { events_received } => {
                            format!("Subscribed, {} events", events_received)
                        }
                        WsStatus::SilentNoEvents => {
                            format!(
                                "SILENT (subscribed, 0 events) [cap: {:?}]",
                                d.capability
                            )
                        }
                        WsStatus::Skipped { reason } => format!("Skipped: {reason}"),
                        WsStatus::SubscribeFailed { error } => {
                            format!("FAILED: {}", truncate(error, 60))
                        }
                        WsStatus::ConnectionDropped { error } => {
                            format!("DROPPED: {}", truncate(error, 60))
                        }
                    };
                    println!("    {:<40} {}", d.stream_type, detail);
                }
            }
        }
    }
}

fn print_stream_availability_matrix(
    ws_details: &[WsSubscriptionDetail],
    stream_types: &[StreamType],
) {
    println!("\n{}", "=".repeat(70));
    println!("STREAM AVAILABILITY MATRIX");
    println!("{}", "=".repeat(70));

    for st in stream_types {
        let label = stream_type_label(st);

        let working: Vec<String> = ws_details
            .iter()
            .filter(|d| {
                d.stream_type == label
                    && matches!(&d.status, WsStatus::Subscribed { events_received } if *events_received > 0)
            })
            .map(|d| format!("{}:{}", d.exchange, d.account_type))
            .collect();

        let silent: Vec<String> = ws_details
            .iter()
            .filter(|d| {
                d.stream_type == label
                    && matches!(d.status, WsStatus::SilentNoEvents)
            })
            .map(|d| format!("{}:{}", d.exchange, d.account_type))
            .collect();

        let failed: Vec<String> = ws_details
            .iter()
            .filter(|d| {
                d.stream_type == label
                    && matches!(d.status, WsStatus::SubscribeFailed { .. })
            })
            .map(|d| format!("{}:{}", d.exchange, d.account_type))
            .collect();

        let skipped: Vec<String> = ws_details
            .iter()
            .filter(|d| {
                d.stream_type == label
                    && matches!(d.status, WsStatus::Skipped { .. })
            })
            .map(|d| format!("{}:{}", d.exchange, d.account_type))
            .collect();

        if working.is_empty() && silent.is_empty() && failed.is_empty() && skipped.is_empty() {
            continue;
        }

        println!("\n{label}:");
        if !working.is_empty() {
            println!("  Working (events): {}", working.join(", "));
        }
        if !silent.is_empty() {
            println!("  Silent (0 events): {}", silent.join(", "));
        }
        if !failed.is_empty() {
            println!("  Failed: {}", failed.join(", "));
        }
        if !skipped.is_empty() {
            println!("  Skipped: {}", skipped.join(", "));
        }
    }
    println!();
}

fn print_summary(
    exchanges: &[(ExchangeId, Vec<AccountType>)],
    connect_failures: &HashMap<String, String>,
    rest_results: &[RestEndpointResult],
    ws_details: &[WsSubscriptionDetail],
    rest_duration_secs: u64,
    ws_duration_secs_actual: u64,
) {
    let total_secs = rest_duration_secs + ws_duration_secs_actual;
    let rest_ok = rest_results.iter().filter(|r| matches!(r.status, RestStatus::Ok { .. })).count();
    let rest_unsupported = rest_results.iter().filter(|r| matches!(r.status, RestStatus::UnsupportedOperation)).count();
    let rest_skipped = rest_results.iter().filter(|r| matches!(r.status, RestStatus::SkippedSpotNoDerivative)).count();
    let rest_errors = rest_results.iter().filter(|r| !matches!(
        r.status,
        RestStatus::Ok { .. } | RestStatus::Empty | RestStatus::UnsupportedOperation | RestStatus::SkippedSpotNoDerivative
    )).count();

    let ws_subscribed = ws_details.iter().filter(|d| matches!(d.status, WsStatus::Subscribed { .. } | WsStatus::SilentNoEvents)).count();
    let ws_with_data = ws_details.iter().filter(|d| matches!(&d.status, WsStatus::Subscribed { events_received } if *events_received > 0)).count();
    let ws_silent = ws_details.iter().filter(|d| matches!(d.status, WsStatus::SilentNoEvents)).count();
    let ws_failed = ws_details.iter().filter(|d| matches!(d.status, WsStatus::SubscribeFailed { .. })).count();
    let ws_skipped = ws_details.iter().filter(|d| matches!(d.status, WsStatus::Skipped { .. })).count();
    let total_events: u64 = ws_details.iter().filter_map(|d| {
        if let WsStatus::Subscribed { events_received } = &d.status { Some(*events_received) } else { None }
    }).sum();

    println!("{}", "=".repeat(70));
    println!("SUMMARY");
    println!("{}", "=".repeat(70));
    println!("Total duration: {}s (REST {}s + WS {}s)", total_secs, rest_duration_secs, ws_duration_secs_actual);
    println!("Exchanges: {}/{} connected", exchanges.len() - connect_failures.len(), exchanges.len());
    if !connect_failures.is_empty() {
        println!("Connect failures:");
        for (ex, err) in connect_failures {
            println!("  {ex}: {}", truncate(err, 80));
        }
    }
    println!(
        "REST endpoints: {} total, {} OK, {} unsupported, {} skipped (spot/no-derivative), {} errors",
        rest_results.len(), rest_ok, rest_unsupported, rest_skipped, rest_errors
    );
    println!("WS subscriptions: {} total", ws_details.len());
    println!("  {} subscribed ({} with data, {} silent)", ws_subscribed, ws_with_data, ws_silent);
    println!("  {} failed, {} skipped", ws_failed, ws_skipped);
    println!("  {} total events", total_events);
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// JSON report
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct SmokeReport {
    duration_secs: u64,
    phase1_rest_duration_secs: u64,
    phase2_ws_duration_secs: u64,
    exchanges: HashMap<String, ExchangeReport>,
    stream_availability: HashMap<String, StreamAvailability>,
}

#[derive(Serialize)]
struct ExchangeReport {
    connected: bool,
    connect_error: Option<String>,
    account_types: Vec<String>,
    rest: HashMap<String, HashMap<String, serde_json::Value>>,
    ws: HashMap<String, HashMap<String, serde_json::Value>>,
}

#[derive(Serialize)]
struct StreamAvailability {
    working: Vec<String>,
    silent: Vec<String>,
    failed: Vec<String>,
    skipped: Vec<String>,
}

fn build_json_report(
    rest_results: &[RestEndpointResult],
    ws_details: &[WsSubscriptionDetail],
    exchanges: &[(ExchangeId, Vec<AccountType>)],
    connect_failures: &HashMap<String, String>,
    stream_types: &[StreamType],
    total_duration_secs: u64,
    rest_duration_secs: u64,
    ws_duration_secs: u64,
) -> SmokeReport {
    let mut exchanges_map: HashMap<String, ExchangeReport> = HashMap::new();

    for (exchange_id, account_types) in exchanges {
        let ex_str = exchange_id.as_str().to_string();
        let connected = !connect_failures.contains_key(&ex_str);
        let connect_error = connect_failures.get(&ex_str).cloned();

        let mut rest_map: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();
        let mut ws_map: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();

        for &account_type in account_types {
            let at_str = account_type.as_key_str().to_string();

            let mut rest_at: HashMap<String, serde_json::Value> = HashMap::new();
            for r in rest_results.iter().filter(|r| r.exchange == ex_str && r.account_type == at_str) {
                rest_at.insert(r.method.clone(), serde_json::to_value(&r.status).unwrap_or_default());
            }
            rest_map.insert(at_str.clone(), rest_at);

            let mut ws_at: HashMap<String, serde_json::Value> = HashMap::new();
            for d in ws_details.iter().filter(|d| d.exchange == ex_str && d.account_type == at_str) {
                ws_at.insert(d.stream_type.clone(), serde_json::to_value(&d.status).unwrap_or_default());
            }
            ws_map.insert(at_str.clone(), ws_at);
        }

        exchanges_map.insert(
            ex_str,
            ExchangeReport {
                connected,
                connect_error,
                account_types: account_types.iter().map(|at| at.as_key_str().to_string()).collect(),
                rest: rest_map,
                ws: ws_map,
            },
        );
    }

    let mut stream_avail: HashMap<String, StreamAvailability> = HashMap::new();
    for st in stream_types {
        let label = stream_type_label(st);
        let working = ws_details
            .iter()
            .filter(|d| {
                d.stream_type == label
                    && matches!(&d.status, WsStatus::Subscribed { events_received } if *events_received > 0)
            })
            .map(|d| format!("{}:{}", d.exchange, d.account_type))
            .collect();
        let silent = ws_details
            .iter()
            .filter(|d| d.stream_type == label && matches!(d.status, WsStatus::SilentNoEvents))
            .map(|d| format!("{}:{}", d.exchange, d.account_type))
            .collect();
        let failed = ws_details
            .iter()
            .filter(|d| d.stream_type == label && matches!(d.status, WsStatus::SubscribeFailed { .. }))
            .map(|d| format!("{}:{}", d.exchange, d.account_type))
            .collect();
        let skipped = ws_details
            .iter()
            .filter(|d| d.stream_type == label && matches!(d.status, WsStatus::Skipped { .. }))
            .map(|d| format!("{}:{}", d.exchange, d.account_type))
            .collect();
        stream_avail.insert(label, StreamAvailability { working, silent, failed, skipped });
    }

    SmokeReport {
        duration_secs: total_duration_secs,
        phase1_rest_duration_secs: rest_duration_secs,
        phase2_ws_duration_secs: ws_duration_secs,
        exchanges: exchanges_map,
        stream_availability: stream_avail,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Full smoke run
// ─────────────────────────────────────────────────────────────────────────────

struct SmokeResults {
    connect_failures: HashMap<String, String>,
    rest_results: Vec<RestEndpointResult>,
    ws_details: Vec<WsSubscriptionDetail>,
    rest_duration_secs: u64,
    ws_duration_secs_actual: u64,
}

async fn run_full_smoke(
    hub: Arc<ExchangeHub>,
    exchanges: Vec<(ExchangeId, Vec<AccountType>)>,
    stream_types: Vec<StreamType>,
    ws_duration_secs: u64,
) -> SmokeResults {
    let mut connect_failures: HashMap<String, String> = HashMap::new();

    // ── Phase 0: Connect all exchanges ──────────────────────────────────────
    println!("\n=== PHASE 0: Connecting exchanges ({} exchanges, {}s timeout per connect) ===",
        exchanges.len(), CONNECT_FULL_TIMEOUT_SECS);

    for (exchange_id, account_types) in &exchanges {
        let connect_result = tokio::time::timeout(
            Duration::from_secs(CONNECT_FULL_TIMEOUT_SECS),
            hub.connect_full(*exchange_id, account_types, false),
        ).await;

        match connect_result {
            Err(_) => {
                let msg = format!("connect_full timed out ({}s)", CONNECT_FULL_TIMEOUT_SECS);
                println!("  {:?}: TIMEOUT", exchange_id);
                connect_failures.insert(exchange_id.as_str().to_string(), msg);
            }
            Ok(Ok(_)) => {
                println!("  {:?}: connected", exchange_id);
            }
            Ok(Err(e)) => {
                println!("  {:?}: FAILED — {}", exchange_id, e);
                connect_failures.insert(exchange_id.as_str().to_string(), e.to_string());
            }
        }
    }

    println!(
        "Connected: {}/{}\n",
        exchanges.len() - connect_failures.len(),
        exchanges.len()
    );

    // ── Phase 1: REST audit (parallel) ──────────────────────────────────────
    let rest_t0 = Instant::now();
    let rest_results = run_rest_audit(Arc::clone(&hub), &exchanges).await;
    let rest_duration_secs = rest_t0.elapsed().as_secs();

    let rest_ok = rest_results.iter().filter(|r| matches!(r.status, RestStatus::Ok { .. })).count();
    let rest_unsupported = rest_results.iter().filter(|r| matches!(r.status, RestStatus::UnsupportedOperation)).count();
    let rest_errors = rest_results.iter().filter(|r| !matches!(
        r.status,
        RestStatus::Ok { .. } | RestStatus::Empty | RestStatus::UnsupportedOperation | RestStatus::SkippedSpotNoDerivative
    )).count();
    println!(
        "REST done: {} total, {} OK, {} unsupported, {} errors ({}s)\n",
        rest_results.len(), rest_ok, rest_unsupported, rest_errors, rest_duration_secs
    );

    // ── Phase 2: WS audit ───────────────────────────────────────────────────
    println!("=== PHASE 2: WS audit ({}s, subscribe timeout {}s per call) ===",
        ws_duration_secs, WS_SUBSCRIBE_TIMEOUT_SECS);
    let ws_t0 = Instant::now();
    let ws_details = run_ws_audit(&hub, &exchanges, &stream_types, ws_duration_secs).await;
    let ws_duration_secs_actual = ws_t0.elapsed().as_secs();

    // Shutdown
    for (id, _) in &exchanges {
        hub.shutdown(*id);
    }

    SmokeResults {
        connect_failures,
        rest_results,
        ws_details,
        rest_duration_secs,
        ws_duration_secs_actual,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let ws_duration_secs: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(120)
        .min(180);

    let storage_dir = PathBuf::from("./smoke_data");
    tracing::info!(
        "mli-collector-smoke: ws_duration={}s storage={}",
        ws_duration_secs,
        storage_dir.display()
    );

    let hub = Arc::new(ExchangeHub::new());
    let exchanges = exchanges_under_test();
    let stream_types = all_public_stream_types();

    let hub_for_smoke = Arc::clone(&hub);
    let exchanges_for_smoke = exchanges.clone();
    let stream_types_for_smoke = stream_types.clone();

    let results = tokio::select! {
        res = run_full_smoke(hub_for_smoke, exchanges_for_smoke, stream_types_for_smoke, ws_duration_secs) => {
            res
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::warn!("Ctrl-C received, printing partial report");
            SmokeResults {
                connect_failures: HashMap::new(),
                rest_results: Vec::new(),
                ws_details: Vec::new(),
                rest_duration_secs: 0,
                ws_duration_secs_actual: 0,
            }
        }
    };

    // ── Print full matrix ────────────────────────────────────────────────────
    print_exchange_matrix(&results.rest_results, &results.ws_details, &exchanges);
    print_stream_availability_matrix(&results.ws_details, &stream_types);
    print_summary(
        &exchanges,
        &results.connect_failures,
        &results.rest_results,
        &results.ws_details,
        results.rest_duration_secs,
        results.ws_duration_secs_actual,
    );

    // ── Save JSON ─────────────────────────────────────────────────────────────
    std::fs::create_dir_all(&storage_dir)?;
    let total_secs = results.rest_duration_secs + results.ws_duration_secs_actual;
    let report = build_json_report(
        &results.rest_results,
        &results.ws_details,
        &exchanges,
        &results.connect_failures,
        &stream_types,
        total_secs,
        results.rest_duration_secs,
        results.ws_duration_secs_actual,
    );
    let json = serde_json::to_string_pretty(&report)?;
    let json_path = storage_dir.join("smoke_report.json");
    std::fs::write(&json_path, &json)?;
    println!("\nJSON report saved to {}", json_path.display());

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exchanges_under_test_count() {
        assert_eq!(exchanges_under_test().len(), 20);
    }

    #[test]
    fn stream_types_public_only() {
        let types = all_public_stream_types();
        for st in &types {
            assert!(
                !matches!(st, StreamType::OrderUpdate | StreamType::BalanceUpdate | StreamType::PositionUpdate),
                "private stream in public list: {:?}",
                st
            );
        }
        assert!(types.len() >= 28);
    }

    #[test]
    fn stream_type_labels_nonempty() {
        for st in all_public_stream_types() {
            assert!(!stream_type_label(&st).is_empty());
        }
    }

    #[test]
    fn pick_symbols_returns_nonempty() {
        let pairs = exchanges_under_test();
        for (ex, ats) in &pairs {
            for &at in ats {
                let syms = pick_symbols(*ex, at);
                assert!(!syms.is_empty(), "no symbols for {:?}/{:?}", ex, at);
            }
        }
    }

    #[test]
    fn pick_symbols_uses_symbol_new() {
        // All symbols must have non-empty base — verifies Symbol::new is used correctly
        let pairs = exchanges_under_test();
        for (ex, ats) in &pairs {
            for &at in ats {
                for sym in pick_symbols(*ex, at) {
                    assert!(!sym.base.is_empty(), "empty base for {:?}/{:?}", ex, at);
                }
            }
        }
    }

    #[test]
    fn truncate_works() {
        assert_eq!(truncate("hello world", 5), "hello");
        assert_eq!(truncate("hi", 5), "hi");
    }
}
