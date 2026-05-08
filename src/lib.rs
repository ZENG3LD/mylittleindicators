//! mylittleindicators — shared indicator + event factory.
//!
//! 480+ технических индикаторов (23 категории) + типы событий, conditions,
//! composition, shapes, spec — основа для построения стратегий и runtime
//! детекторов в крейтах-потребителях (mylittlequant, mylittlechart).
//!
//! Здесь нет runtime-логики (детекторов, signal-engine, рендера) и нет
//! domain-данных (defaults). Это чистая фабрика типов и индикаторов.

// Core types (bar, tick, time, calendar, timeframe)
pub mod types;

// Bar indicators
pub mod bar_indicators;

// Catalog system (signatures, constraints, param values, indicator key)
pub mod catalog;

// Legacy re-export: old MLQ path was `mlq_indicators::indicator_key::IndicatorKey`.
// Keep the old path working so warmup callers don't need import rewrites.
pub use catalog::indicator_key;

// Strategies — taxonomy of events / conditions / composition / shapes / spec.
// Pure types, no runtime. Consumers (mlq, mlc) build their own detectors,
// engines, and codegen on top of these.
pub mod strategies;

// Convenience re-exports
pub use bar_indicators::{
    bar_indicator_id::BarIndicatorId,
    indicator_value::IndicatorValue,
    instance_factory::{IndicatorConfig, IndicatorInstance},
};

pub use catalog::{
    master_catalog::MasterIndicatorCatalog,
    indicator_signature::IndicatorSignature,
    constraints::ParamConstraint,
    param_value::ParamValue,
};

pub use types::{Bar, Tick, CalendarService, TimeService};

// Strategy / event taxonomy re-exports
pub use strategies::events::{
    SignalKind, SignalCategory,
    ThresholdSub, HistogramSub, ChannelSub, DivergenceSub, TrendSub,
    VolatilitySub, VolumeSub, StructureSub, PatternSub, CompositeSub,
    Direction, SignalSource, BarConfirmation,
};
pub use strategies::conditions::{
    ThresholdCondition, CrossoverType, CompareCondition, TrendCondition,
    DivergenceType, ChannelPosition, PatternState, CandlePattern,
    VolatilityRegime, VolumeCharacter, LogicOp, ConfirmationRequirement,
};
