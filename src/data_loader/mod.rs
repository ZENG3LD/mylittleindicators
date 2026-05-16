//! Multi-stream data loading for backtest pipeline.
//!
//! ## Architecture
//!
//! ```text
//! ExchangeHubFetcher (REST)
//!   └── hub.rest(exchange) → CoreConnector trait methods
//!       MarketData: get_klines, get_orderbook, get_recent_trades
//!       MarketDataPublic: get_funding_rate_history, get_liquidation_history,
//!                         get_open_interest_history, get_long_short_ratio_history
//!
//! Subscriber (WS) [mli-collector crate]
//!   └── hub.ws(exchange, account_type) → WebSocketConnector::event_stream()
//!       StreamEvent → TimedEvent mapping (30+ variants)
//!
//! StorageRoot (накопление)
//!   └── Binary log per (exchange, symbol, stream_kind)
//!
//! EnrichedDataLoader (загрузка)
//!   └── Merges bars + N streams from any DataSource
//!
//! TimelineMerger (sync по timestamp)
//!   └── merge_sorted, bar_boundaries, align_to_bars
//! ```
//!
//! ## REST-capable StreamKinds (via ExchangeHubFetcher)
//!
//! | StreamKind     | Trait method                                     |
//! |----------------|--------------------------------------------------|
//! | Bar            | `MarketData::get_klines`                         |
//! | Tick           | `MarketDataPublic::get_recent_trades`            |
//! | OrderBook      | `MarketData::get_orderbook`                      |
//! | Funding        | `MarketDataPublic::get_funding_rate_history`     |
//! | Liquidation    | `MarketDataPublic::get_liquidation_history`      |
//! | OpenInterest   | `MarketDataPublic::get_open_interest_history`    |
//! | LongShortRatio | `MarketDataPublic::get_long_short_ratio_history` |
//!
//! ## WS-only StreamKinds (no REST history)
//!
//! OrderbookDelta, AggTrade, Ticker, MarkPrice, OptionGreeks, VolatilityIndex,
//! Basis, IndexPrice, CompositeIndex, InsuranceFund, Settlement, BlockTrade,
//! OrderbookL3, RiskLimit, PredictedFunding, FundingSettlement, Auction,
//! MarketWarning, HistoricalVolatility.
//!
//! ## Rule: ALL dig3 connections ONLY through ExchangeHub
//!
//! No direct `ConnectorFactory`, `BinanceConnector`, or `WebSocketPool` imports.
//! Entry point is always `ExchangeHub::rest(id)` or `ExchangeHub::ws(id, at)`.

pub mod data_source;
pub mod exchange_hub_fetcher;
pub mod enriched_history;
pub mod enriched_loader;
pub mod rest_fetcher;
pub mod storage;
pub mod stream_kind;
pub mod timed_event;
pub mod timeline_merger;

pub use data_source::DataSource;
pub use exchange_hub_fetcher::ExchangeHubFetcher;
pub use enriched_history::EnrichedHistory;
pub use enriched_loader::EnrichedDataLoader;
pub use rest_fetcher::RestFetcher;
pub use storage::StorageRoot;
pub use stream_kind::StreamKind;
pub use timed_event::TimedEvent;
pub use timeline_merger::{merge_sorted, bar_boundaries, align_to_bars};
