//! Codegen AST layer — types for describing strategies and generating Rust code.
//!
//! Used by `mlq-strategies-codegen` and consumers that build strategy specs.
//!
//! - `event`       — Event, ZoneBounds, EventTrigger (renamed from EventDirection)
//! - `operator`    — OperatorClass, Strictness, strictness_for
//! - `operand`     — Operand, BarField, AggregateOp, DerivedOp, ArithmeticOp
//! - `window`      — Window
//! - `composition` — CompositionSpec, Guard, CmpOp, validate_event, validate_composition
//! - `role`        — RoleKind, role_kind_for

pub mod event;
pub mod operator;
pub mod operand;
pub mod window;
pub mod composition;
pub mod role;

pub use event::{Event, ZoneBounds, EventTrigger};
pub use operator::{OperatorClass, Strictness, strictness_for};
pub use operand::{Operand, BarField, AggregateOp, DerivedOp, ArithmeticOp};
pub use window::Window;
pub use composition::{CompositionSpec, Guard, CmpOp, validate_event, validate_composition};
pub use role::{RoleKind, role_kind_for};
