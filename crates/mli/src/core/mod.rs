//! `core` — все базовые типы крейта.
//!
//! - `types/`  — сырые рыночные данные (Bar, Tick, TimeService, CalendarService, ResearchTimeframe)
//! - `signal/` — runtime taxonomy (SignalKind, Direction, BarConfirmation)
//!
//! Strategy AST (Event/Operand/OperatorClass/CompositionSpec/Guard/RoleKind)
//! moved OUT to the family crate `crates/mli-strategies` (non-OSS). OSS core
//! stays pure indicators + events + runtime taxonomy. Was `pub mod ast;`.

pub mod types;
pub mod signal;
