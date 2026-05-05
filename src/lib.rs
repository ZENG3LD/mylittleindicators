//! mylittleindicators — shared indicator library extracted from mylittlechart.
//!
//! Provides 480+ technical indicators across 23 categories, plus catalog metadata,
//! signal primitives, and core types (Bar, Tick, time/calendar services).
//!
//! Render layer (`managers/`) intentionally not included — that lives in mylittlechart.

// Core types (bar, tick, time, calendar)
pub mod types;

// Core indicator types
pub mod bar_indicators;

// Catalog system
pub mod catalog;

// Legacy re-export: old MLQ path was `mlq_indicators::indicator_key::IndicatorKey`.
// MLC layout moved it to `catalog::indicator_key`. Keep the old path working
// so 12+ warmup callers don't need import rewrites.
pub use catalog::indicator_key;

// Signal system — signal types, conditions, detectors
pub mod signals;

// Re-exports for convenience
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

// Signal system re-exports
pub use signals::{
    SignalKind, SignalCategory,
    ThresholdSub, HistogramSub, ChannelSub, DivergenceSub, TrendSub,
    VolatilitySub, VolumeSub, StructureSub, PatternSub, CompositeSub,
    ThresholdCondition, CrossoverType, CompareCondition, TrendCondition,
    DivergenceType, ChannelPosition, PatternState, CandlePattern,
    VolatilityRegime, VolumeCharacter, LogicOp, ConfirmationRequirement,
    CrossoverDetector, ThresholdMonitor, ZeroCrossDetector, HistogramDetector,
    ChannelDetector, DivergenceDetector, TrendDetector, VolatilityDetector,
    VolumeDetector, SwingDetector, MultiSignalDetector,
    Signal, Direction, BarConfirmation, SignalSource,
};
