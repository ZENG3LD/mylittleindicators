//! mylittleindicators — shared indicator + event factory.
//!
//! 480+ технических индикаторов (23 категории) + типы событий, composition,
//! role_kind — низкоуровневый клей для построения стратегий и
//! runtime детекторов в крейтах-потребителях (mylittlequant, mylittlechart).
//!
//! Здесь нет runtime-логики (детекторов, engine, рендера), нет defaults,
//! нет StrategySpec. Только индикаторы и пограничные типы событий.

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

pub use core::types::{Bar, Tick, CalendarService, TimeService};

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
