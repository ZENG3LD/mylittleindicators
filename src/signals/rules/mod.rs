//! Re-export из `strategies::runtime` (миграция).
//!
//! Старые callers `mli::signals::rules::*` продолжают работать.
//! Новый код использует `mli::strategies::runtime::*`.
//!
//! `defaults` остаётся здесь временно — переедет в `strategies::defaults`.

pub use crate::strategies::runtime::config::{
    DetectorConfig, DetectorType, DetectorParams, ValueSource,
};
pub use crate::strategies::runtime::profile::SignalProfile;
pub use crate::strategies::runtime::engine::SignalEngine;

pub mod defaults;
pub use defaults::default_profile;
