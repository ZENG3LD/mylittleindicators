//! Re-export из `strategies::runtime` + `strategies::defaults` (миграция).
//!
//! Старые callers `mli::signals::rules::*` продолжают работать.
//! Новый код использует `mli::strategies::runtime::*` и `mli::strategies::defaults::*`.

pub use crate::strategies::runtime::config::{
    DetectorConfig, DetectorType, DetectorParams, ValueSource,
};
pub use crate::strategies::runtime::profile::SignalProfile;
pub use crate::strategies::runtime::engine::SignalEngine;

pub use crate::strategies::defaults::default_profile;
pub mod defaults {
    pub use crate::strategies::defaults::profiles::*;
}
