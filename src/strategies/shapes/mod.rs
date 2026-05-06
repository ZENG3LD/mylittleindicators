//! Shapes — топологии стратегий.
//!
//! `role_kind` — taxonomy ролей индикатора. Дальше переедут shape templates
//! из mlq-strategies-codegen.

pub mod role_kind;
pub mod structure;

pub use role_kind::{RoleKind, role_kind_for};
