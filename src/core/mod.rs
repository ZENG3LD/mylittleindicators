//! `core` — низкоуровневый клей: типы событий, conditions, composition,
//! role_kind. Это пограничная зона между фабрикой индикаторов (MLI) и
//! доменной логикой стратегий (MLQ) / визуализации (MLC).
//!
//! Здесь нет runtime (детекторов, engine), нет defaults, нет StrategySpec.
//! Только типы которые нужны и для backtest-стратегий, и для chart-детекторов
//! одинаково.
//!
//! Layout:
//!
//! ```text
//! core/
//! ├── events/        — taxonomy + axes событий (kind, operator, operand, window,
//! │                     direction, event_direction, confirmation, event, signal_type)
//! ├── conditions/    — атомарные предикаты-чекеры (threshold, crossover, ...)
//! ├── composition/   — CompositionSpec (And/Or/Not/Sequence), Guard, validation
//! └── shapes/        — RoleKind taxonomy (роль индикатора в стратегии)
//! ```

pub mod events;
pub mod conditions;
pub mod composition;
pub mod shapes;
