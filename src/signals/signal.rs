//! Re-export из `strategies::runtime::signal` + `strategies::events::{direction, confirmation}`.
//!
//! Старые callers `mli::signals::signal::*` продолжают работать.
//! Новый код использует `mli::strategies::runtime::Signal` и
//! `mli::strategies::events::{Direction, SignalSource, BarConfirmation}`.

pub use crate::strategies::runtime::signal::Signal;
pub use crate::strategies::events::direction::{Direction, SignalSource};
pub use crate::strategies::events::confirmation::BarConfirmation;
