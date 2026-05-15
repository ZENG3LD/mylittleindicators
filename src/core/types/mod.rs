//! Core types for the indicators library
//!
//! These types provide the foundation for all indicator calculations.

mod bar;
mod calendar;
mod time_service;
mod timeframe;

pub use bar::{Bar, Tick};
pub use calendar::CalendarService;
pub use time_service::TimeService;
pub use timeframe::ResearchTimeframe;

// Market data types — source of truth is digdigdig3
pub use digdigdig3::core::types::{
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
