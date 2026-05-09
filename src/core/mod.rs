//! `core` — все базовые типы крейта.
//!
//! - `types/`        — сырые рыночные данные (Bar, Tick, TimeService, CalendarService, ResearchTimeframe)
//! - `events/`       — taxonomy + axes событий (kind, operator, operand, window,
//!                     direction, event_direction, confirmation, event, signal_type)
//! - `conditions/`   — атомарные predicate-чекеры (threshold, crossover, ...)
//! - `composition/`  — CompositionSpec (And/Or/Not/Sequence), Guard, validation
//! - `shapes/`       — RoleKind taxonomy (роль индикатора в стратегии)
//!
//! Здесь нет runtime (детекторов, engine), нет defaults, нет StrategySpec.
//! Только типы которые нужны и backtest-стратегиям, и chart-детекторам одинаково.

pub mod types;
pub mod events;
pub mod conditions;
pub mod composition;
pub mod shapes;
