//! Events — что и как распознаётся.
//!
//! Объединяет MLI taxonomy событий (SignalKind с подтипами, BarConfirmation,
//! Direction, SignalSource) с MLQ codegen осями (Strictness, OperatorClass,
//! Operand, Window).
//!
//! Roadmap:
//! - `kind.rs`         — SignalKind + 10 sub-enums (из MLI signals/catalog.rs)
//! - `strictness.rs`   — OnEdge/Persistent/FirstTime/NBarsConfirmed (MLQ axes)
//! - `confirmation.rs` — BarConfirmation Pending/Closed/WickOnly (MLI)
//! - `direction.rs`    — Direction + SignalSource (MLI)
//! - `operator.rs`     — OperatorClass: Cross/Threshold/ZoneExit/Divergence/NBarExtreme
//! - `operand.rs`      — Role/Constant/PriceField (MLQ axes)
//! - `window.rs`       — Window::Sliding/PivotLR (MLQ)
