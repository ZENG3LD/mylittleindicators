//! Events — что и как распознаётся.
//!
//! Объединяет MLI taxonomy событий (SignalKind с подтипами, BarConfirmation,
//! Direction, SignalSource) с MLQ codegen осями (OperatorClass, Operand,
//! Window, EventDirection, и сам Event как атом стратегии).

pub mod kind;
pub mod direction;
pub mod confirmation;
pub mod operator;
pub mod operand;
pub mod window;
pub mod event_direction;
pub mod event;
pub mod signal_type;

pub use kind::{
    SignalKind, SignalCategory,
    ThresholdSub, HistogramSub, ChannelSub, DivergenceSub, TrendSub,
    VolatilitySub, VolumeSub, StructureSub, PatternSub, CompositeSub,
};
pub use direction::{Direction, SignalSource};
pub use confirmation::BarConfirmation;
pub use operator::{OperatorClass, Strictness, strictness_for};
pub use operand::{Operand, BarField, AggregateOp, DerivedOp, ArithmeticOp};
pub use window::Window;
pub use event_direction::EventDirection;
pub use event::Event;
pub use signal_type::{SignalType, signal_type_for};
