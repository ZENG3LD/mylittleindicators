//! mylittleindicators — shared indicator + event factory.
//!
//! 480+ технических индикаторов (23 категории) + типы событий, conditions,
//! composition, role_kind — низкоуровневый клей для построения стратегий и
//! runtime детекторов в крейтах-потребителях (mylittlequant, mylittlechart).
//!
//! Здесь нет runtime-логики (детекторов, engine, рендера), нет defaults,
//! нет StrategySpec. Только индикаторы и пограничные типы событий.

// Bar indicators
pub mod bar_indicators;

// Catalog system (signatures, constraints, param values, indicator key)
pub mod catalog;

// Legacy re-export: old MLQ path `mlq_indicators::indicator_key::IndicatorKey`.
pub use catalog::indicator_key;

// All base types: market data (Bar/Tick/...), events, conditions, composition, shapes.
pub mod core;

// Backwards-compat: `crate::types::*` was the old path before types/ moved into core/.
pub use core::types;

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

pub use core::types::{Bar, Tick, CalendarService, TimeService};

// Event taxonomy re-exports
pub use core::events::{
    SignalKind, SignalCategory,
    ThresholdSub, HistogramSub, ChannelSub, DivergenceSub, TrendSub,
    VolatilitySub, VolumeSub, StructureSub, PatternSub, CompositeSub,
    Direction, SignalSource, BarConfirmation,
};
pub use core::conditions::{
    ThresholdCondition, CrossoverType, CompareCondition, TrendCondition,
    DivergenceType, ChannelPosition, PatternState, CandlePattern,
    VolatilityRegime, VolumeCharacter, LogicOp, ConfirmationRequirement,
};
