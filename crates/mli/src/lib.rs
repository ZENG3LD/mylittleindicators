//! mylittleindicators — shared indicator + event factory.
//!
//! 480+ технических индикаторов (23 категории) + типы событий, composition,
//! role_kind — низкоуровневый клей для построения стратегий и
//! runtime детекторов в крейтах-потребителях (mylittlequant, mylittlechart).
//!
//! Здесь нет runtime-логики (детекторов, engine, рендера), нет defaults,
//! нет StrategySpec. Только индикаторы и пограничные типы событий.

// The contract engine — ContractFactory + the typed id/value/field/stream surface it builds
// over + the per-stream consumer traits. The single registry for indicators AND detectors.
pub mod engine;

// Indicators (every category, incl. absorbed detectors) — impls only; the engine is `crate::engine`.
pub mod indicators;

// Indicator self-declaration contract (typed config + defaults + construction).
pub mod contract;

// The legacy `events/` factory (EventInstance / EventId / events_catalog /
// event_config) was ABSORBED into the main `contract_engine` ContractFactory —
// every detector is now an ordinary contract-backed indicator core (manifest
// member with `impl Indicator`/`Render` + `feed`). No separate events subsystem.

// Elliott Wave Analysis — absorbed from the former `mli-ewa` family crate.
pub mod ewa;

pub mod harmonic;

pub mod fib;

pub mod events;

// 0.1.8 paths. The cores are the contract cores. Not a second catalog.
pub mod bar_indicators;
pub mod catalog;
pub mod data_loader;

pub mod smc;

pub mod orderflow;

// All base types: market data (Bar/Tick/...), signal taxonomy, codegen AST.
pub mod core;

// Backwards-compat: `crate::types::*` was the old path before types/ moved into core/.
pub use core::types;

// Convenience re-exports
pub use engine::indicator_id::IndicatorId;

pub use core::types::{
    Bar, Tick, CalendarService, TimeService,
    OrderBook, OrderBookLevel, OrderbookDelta,
    FundingRate, MarkPrice, OpenInterest, Ticker,
    Liquidation, TradeSide,
    PublicTrade,
    // Stream event types
    AggTrade, AuctionEvent, Basis, BlockTrade, CompositeIndex,
    FundingSettlement, HistoricalVolatility, IndexPrice, InsuranceFund,
    Kline, LongShortRatio, MarketWarning, OptionGreeks,
    L3Action, OrderBookSide, OrderbookL3Event,
    PredictedFunding, RiskLimit, SettlementEvent, VolatilityIndex,
};
pub use engine::streams::LiquidationConsumer;
pub use indicators::liquidations::{LiquidationCascade, LiquidationRate, LiquidationVolumeImbalance};
pub use engine::streams::TickConsumer;
pub use engine::streams::TickerConsumer;
pub use engine::streams::orderbook_delta_consumer::OrderbookDeltaConsumer;
pub use engine::streams::funding_rate_consumer::FundingRateConsumer;
pub use engine::streams::mark_price_consumer::MarkPriceConsumer;
pub use engine::streams::open_interest_consumer::OpenInterestConsumer;
pub use engine::streams::hybrid_tick_book_consumer::HybridTickBookConsumer;
// New stream event consumer traits
pub use engine::streams::AggTradeConsumer;
pub use engine::streams::AuctionEventConsumer;
pub use engine::streams::BasisConsumer;
pub use engine::streams::BlockTradeConsumer;
pub use engine::streams::CompositeIndexConsumer;
pub use engine::streams::FundingSettlementConsumer;
pub use engine::streams::HistoricalVolatilityConsumer;
pub use engine::streams::IndexPriceConsumer;
pub use engine::streams::InsuranceFundConsumer;
pub use engine::streams::LongShortRatioConsumer;
pub use engine::streams::MarketWarningConsumer;
pub use engine::streams::OptionGreeksConsumer;
pub use engine::streams::OrderbookL3Consumer;
pub use engine::streams::PredictedFundingConsumer;
pub use engine::streams::RiskLimitConsumer;
pub use engine::streams::SettlementEventConsumer;
pub use engine::streams::VolatilityIndexConsumer;

// Signal taxonomy re-exports (runtime layer)
pub use core::signal::{
    SignalKind, SignalCategory,
    ThresholdSub, HistogramSub, ChannelSub, DivergenceSub, TrendSub,
    VolatilitySub, VolumeSub, StructureSub, PatternSub, CompositeSub,
    Direction, BarConfirmation,
};

// Strategy AST re-exports REMOVED — the AST (Event/Operand/OperatorClass/
// CompositionSpec/Guard/RoleKind) lives in the family crate `mli-strategies`.
// OSS consumers that need the strategy vocabulary depend on `mli-strategies`.
