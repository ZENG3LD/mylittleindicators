//! Re-export из `strategies::conditions::atoms` (миграция).
//!
//! Старые callers `mli::signals::conditions::*` продолжают работать.
//! Новый код использует `mli::strategies::conditions::*`.

pub use crate::strategies::conditions::atoms::*;
