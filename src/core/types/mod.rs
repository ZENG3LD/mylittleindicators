//! Core types for the indicators library
//!
//! These types provide the foundation for all indicator calculations.

mod bar;
mod calendar;
mod funding_rate;
mod liquidation;
mod mark_price;
mod open_interest;
mod order_book;
mod orderbook_delta;
mod public_trade;
mod ticker;
mod time_service;
mod timeframe;

// New stream event types (mirrored from digdigdig3)
mod agg_trade;
mod auction_event;
mod basis;
mod block_trade;
mod composite_index;
mod funding_settlement;
mod historical_volatility;
mod index_price;
mod insurance_fund;
mod long_short_ratio;
mod market_warning;
mod option_greeks;
mod orderbook_l3;
mod predicted_funding;
mod risk_limit;
mod settlement_event;
mod volatility_index;

pub use bar::{Bar, Tick};
pub use calendar::CalendarService;
pub use funding_rate::FundingRate;
pub use liquidation::{Liquidation, LiquidationSide};
pub use mark_price::MarkPrice;
pub use open_interest::OpenInterest;
pub use order_book::{OrderBook, OrderBookLevel};
pub use orderbook_delta::OrderbookDelta;
pub use public_trade::PublicTrade;
pub use ticker::Ticker;
pub use time_service::TimeService;
pub use timeframe::ResearchTimeframe;

// New stream event re-exports
pub use agg_trade::AggTrade;
pub use auction_event::AuctionEvent;
pub use basis::Basis;
pub use block_trade::BlockTrade;
pub use composite_index::CompositeIndex;
pub use funding_settlement::FundingSettlement;
pub use historical_volatility::HistoricalVolatility;
pub use index_price::IndexPrice;
pub use insurance_fund::InsuranceFund;
pub use long_short_ratio::LongShortRatio;
pub use market_warning::MarketWarning;
pub use option_greeks::OptionGreeks;
pub use orderbook_l3::{L3Action, OrderBookSide, OrderbookL3Event};
pub use predicted_funding::PredictedFunding;
pub use risk_limit::RiskLimit;
pub use settlement_event::SettlementEvent;
pub use volatility_index::VolatilityIndex;
