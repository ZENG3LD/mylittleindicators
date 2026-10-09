//! Core types for the indicators library
//!
//! These types provide the foundation for all indicator calculations.

mod bar;
mod calendar;
mod time_service;

pub use bar::{Bar, Tick};
pub use calendar::CalendarService;
pub use time_service::TimeService;
// `ResearchTimeframe` canonically lives in `crate::contract::timeframe` — the
// former `core/types/timeframe.rs` duplicate was removed 2026-07-06 (ms cutover).
pub use crate::contract::timeframe::ResearchTimeframe;

// Market data types — source of truth is digdigdig3-core (the light types crate)
pub use digdigdig3_core::core::types::{
    AggTrade,
    AuctionEvent,
    Basis,
    BlockTrade,
    CompositeIndex,
    FundingRate,
    FundingSettlement,
    HistoricalVolatility,
    IndexPrice,
    InsuranceFund,
    Kline,
    L3Action,
    Liquidation,
    LongShortRatio,
    MarkPrice,
    MarketWarning,
    OptionGreeks,
    OpenInterest,
    OrderBook,
    OrderBookLevel,
    OrderBookSide,
    OrderbookDelta,
    OrderbookL3Event,
    PredictedFunding,
    PublicTrade,
    RiskLimit,
    SettlementEvent,
    Ticker,
    TradeSide,
    VolatilityIndex,
};
