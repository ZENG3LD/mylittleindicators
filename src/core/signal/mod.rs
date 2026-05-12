//! Runtime taxonomy layer — what detectors emit on bars.
//!
//! Used by `mli/events/*` runtime detectors and consumers (e.g. mylittlechart).
//!
//! - `kind`      — SignalKind hierarchy (13 kinds + subtypes)
//! - `direction` — Direction (Up/Down/Neutral) + BarConfirmation

pub mod kind;
pub mod direction;

pub use kind::{
    SignalKind, SignalCategory,
    ThresholdSub, HistogramSub, ChannelSub, DivergenceSub, TrendSub,
    VolatilitySub, VolumeSub, StructureSub, PatternSub, CompositeSub,
};
pub use direction::{Direction, BarConfirmation};
