//! `core` — все базовые типы крейта.
//!
//! - `types/`  — сырые рыночные данные (Bar, Tick, TimeService, CalendarService, ResearchTimeframe)
//! - `signal/` — runtime taxonomy (SignalKind, Direction, BarConfirmation)
//! - `ast/`    — codegen AST (Event, EventTrigger, OperatorClass, Operand, Window,
//!               CompositionSpec, Guard, RoleKind, validation)
//!
//! Здесь нет runtime (детекторов, engine), нет defaults, нет StrategySpec.
//! Только типы которые нужны и backtest-стратегиям, и chart-детекторам одинаково.

pub mod types;
pub mod signal;
pub mod ast;
