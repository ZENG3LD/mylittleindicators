//! Re-export из `strategies::runtime::detectors` (миграция).
//!
//! Старые callers `mli::signals::detectors::*` продолжают работать.
//! Новый код использует `mli::strategies::runtime::detectors::*`.

pub use crate::strategies::runtime::detectors::*;
