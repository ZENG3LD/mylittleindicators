//! mylittleindicators — shared indicator + event factory.
//!
//! 480+ технических индикаторов (23 категории) + типы событий, composition,
//! role_kind — низкоуровневый клей для построения стратегий и
//! runtime детекторов в крейтах-потребителях (mylittlequant, mylittlechart).
//!
//! Здесь нет runtime-логики (детекторов, engine, рендера), нет defaults,
//! нет StrategySpec. Только индикаторы и пограничные типы событий.

// Multi-stream data loading for backtest pipeline.
pub mod data_loader;

// Bar indicators
pub mod bar_indicators;

// Event detectors (strategy primitives over indicators)
pub mod events;

// Catalog system (signatures, constraints, param values, indicator key)
pub mod catalog;

// Legacy re-export: old MLQ path `mlq_indicators::indicator_key::IndicatorKey`.
pub use catalog::indicator_key;

// All base types: market data (Bar/Tick/...), signal taxonomy, codegen AST.
pub mod core;

// Backwards-compat: `crate::types::*` was the old path before types/ moved into core/.
pub use core::types;

// Convenience re-exports
pub use bar_indicators::{
    bar_indicator_id::BarIndicatorId,
    indicator_value::{IndicatorValue, IndicatorValueKind},
    instance_factory::{IndicatorConfig, IndicatorInstance},
};

pub use catalog::{
    master_catalog::MasterIndicatorCatalog,
    indicator_signature::IndicatorSignature,
    constraints::ParamConstraint,
    param_value::ParamValue,
};

pub use core::types::{
    Bar, Tick, CalendarService, TimeService,
    OrderBook, OrderBookLevel, OrderbookDelta,
    FundingRate, MarkPrice, OpenInterest, Ticker,
    Liquidation, LiquidationSide,
    PublicTrade,
    // New stream event types
    AggTrade, AuctionEvent, Basis, BlockTrade, CompositeIndex,
    FundingSettlement, HistoricalVolatility, IndexPrice, InsuranceFund,
    LongShortRatio, MarketWarning, OptionGreeks,
    L3Action, OrderBookSide, OrderbookL3Event,
    PredictedFunding, RiskLimit, SettlementEvent, VolatilityIndex,
};
pub use bar_indicators::LiquidationConsumer;
pub use bar_indicators::liquidations::{LiquidationCascade, LiquidationRate, LiquidationVolumeImbalance};
pub use bar_indicators::TickConsumer;
pub use bar_indicators::TickerConsumer;
pub use bar_indicators::orderbook_delta_consumer::OrderbookDeltaConsumer;
pub use bar_indicators::funding_rate_consumer::FundingRateConsumer;
pub use bar_indicators::mark_price_consumer::MarkPriceConsumer;
pub use bar_indicators::open_interest_consumer::OpenInterestConsumer;
pub use bar_indicators::hybrid_tick_book_consumer::HybridTickBookConsumer;
// New stream event consumer traits
pub use bar_indicators::AggTradeConsumer;
pub use bar_indicators::AuctionEventConsumer;
pub use bar_indicators::BasisConsumer;
pub use bar_indicators::BlockTradeConsumer;
pub use bar_indicators::CompositeIndexConsumer;
pub use bar_indicators::FundingSettlementConsumer;
pub use bar_indicators::HistoricalVolatilityConsumer;
pub use bar_indicators::IndexPriceConsumer;
pub use bar_indicators::InsuranceFundConsumer;
pub use bar_indicators::LongShortRatioConsumer;
pub use bar_indicators::MarketWarningConsumer;
pub use bar_indicators::OptionGreeksConsumer;
pub use bar_indicators::OrderbookL3Consumer;
pub use bar_indicators::PredictedFundingConsumer;
pub use bar_indicators::RiskLimitConsumer;
pub use bar_indicators::SettlementEventConsumer;
pub use bar_indicators::VolatilityIndexConsumer;

// Signal taxonomy re-exports (runtime layer)
pub use core::signal::{
    SignalKind, SignalCategory,
    ThresholdSub, HistogramSub, ChannelSub, DivergenceSub, TrendSub,
    VolatilitySub, VolumeSub, StructureSub, PatternSub, CompositeSub,
    Direction, BarConfirmation,
};

// AST re-exports (codegen layer)
pub use core::ast::{
    Event, ZoneBounds, EventTrigger,
    OperatorClass, Strictness, strictness_for,
    Operand, BarField, AggregateOp, DerivedOp, ArithmeticOp,
    Window,
    CompositionSpec, Guard, CmpOp,
    RoleKind, role_kind_for,
};
