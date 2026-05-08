//! `strategies` — taxonomy событий, conditions, composition, shapes, spec.
//!
//! Чистая фабрика типов. Здесь нет runtime (детекторы, engine, signal struct
//! с timestamp/price) — это доменная логика потребителя (mlq optimizer / mlc chart).
//! Здесь нет defaults — параметры всегда передаются через runtime config.
//!
//! Layout:
//!
//! ```text
//! strategies/
//! ├── events/        — taxonomy событий (kind, operator, operand, window, ...)
//! ├── conditions/    — атомарные предикаты (threshold, crossover, divergence, ...)
//! ├── composition/   — CompositionSpec (And/Or/Not/Sequence), Guard, validation
//! ├── shapes/        — топологии стратегий (cross_2roles, threshold_zone_exit, ...)
//! ├── multi_tf/      — multi-TF orchestration types
//! └── spec.rs        — RoleSpec / StateSpec / ActionMap / StrategySpec
//! ```

pub mod events;
pub mod conditions;
pub mod composition;
pub mod shapes;
pub mod multi_tf;
pub mod spec;

pub use spec::{
    RoleSpec, StateVar, Transition, StateSpec,
    ValidationRuleSpec, ActionMap, TfArity, StrategySpec,
};
