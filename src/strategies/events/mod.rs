//! Events — что и как распознаётся.
//!
//! Объединяет MLI taxonomy событий (SignalKind с подтипами, BarConfirmation,
//! Direction, SignalSource) с MLQ codegen осями (Strictness, OperatorClass,
//! Operand, Window).
//!
//! Roadmap:
//! - `kind.rs`         ✅ SignalKind + 10 sub-enums (из MLI signals/catalog.rs)
//! - `direction.rs`    ✅ Direction + SignalSource (MLI)
//! - `confirmation.rs` ✅ BarConfirmation Pending/Closed/WickOnly (MLI)
//! - `strictness.rs`   TODO OnEdge/Persistent/FirstTime/NBarsConfirmed (MLQ axes)
//! - `operator.rs`     TODO OperatorClass: Cross/Threshold/ZoneExit/Divergence/NBarExtreme
//! - `operand.rs`      TODO Role/Constant/PriceField (MLQ axes)
//! - `window.rs`       TODO Window::Sliding/PivotLR (MLQ)

pub mod kind;
pub mod direction;
pub mod confirmation;

pub use kind::{
    SignalKind, SignalCategory,
    ThresholdSub, HistogramSub, ChannelSub, DivergenceSub, TrendSub,
    VolatilitySub, VolumeSub, StructureSub, PatternSub, CompositeSub,
};
pub use direction::{Direction, SignalSource};
pub use confirmation::BarConfirmation;
