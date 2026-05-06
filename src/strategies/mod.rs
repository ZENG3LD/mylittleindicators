//! `strategies` — единое пространство для taxonomy событий, conditions,
//! composition, shapes, defaults и runtime-детекторов.
//!
//! Объединяет два источника:
//! - MLI `signals/*` — taxonomy SignalKind, BarConfirmation, runtime detectors,
//!   defaults per-индикатор (то что работает в чарте/алертах);
//! - MLQ codegen `mlq-strategies-codegen/src/*` — operator/operand/window/
//!   strictness/guard/state/composition оси, shape-templates, validation rules.
//!
//! Layout:
//!
//! ```text
//! strategies/
//! ├── events/        — что и как распознаётся (kind, strictness, operator, ...)
//! ├── conditions/    — атомарные предикаты (threshold, crossover, divergence, ...)
//! ├── composition/   — как собирать стратегию (CompositionSpec, Guard, StateVar, ValidateRule)
//! ├── shapes/        — топологии стратегий (cross_2roles, threshold_zone_exit, ...)
//! │   └── structure/ — SMC events (BOS, CHoCH, FVG, OrderBlock, ...)
//! ├── multi_tf/      — multi-TF orchestration (htf/ltf, change_flags)
//! ├── defaults/      — энциклопедия чисел per-индикатор (RSI 30/70, ...)
//! └── runtime/       — UI/alerts streaming layer (Signal, SignalEngine, detectors)
//! ```
//!
//! **Слои относительно hot loop:**
//! - `runtime/` — НЕ участвует в MLQ hot loop (allocations, virtual dispatch),
//!   используется MLC для streaming UI и алертов;
//! - `events/`, `conditions/`, `composition/`, `shapes/`, `defaults/` —
//!   определения, используются MLQ codegen для генерации hot-loop кода.

pub mod events;
pub mod conditions;
pub mod composition;
pub mod shapes;
pub mod multi_tf;
pub mod defaults;
pub mod runtime;
