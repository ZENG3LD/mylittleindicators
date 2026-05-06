//! Re-export из `strategies::events::kind` (миграция).
//!
//! Старые callers `mli::signals::catalog::*` продолжают работать.
//! Новый код использует `mli::strategies::events::*`.

pub use crate::strategies::events::kind::*;
