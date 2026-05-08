//! BarConfirmation — статус подтверждения бара.
//!
//! Migrated from `signals/signal.rs`.

use serde::{Deserialize, Serialize};

/// Статус подтверждения бара в момент эмиссии сигнала.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BarConfirmation {
    /// Бар ещё не закрыт — сигнал может исчезнуть.
    Pending,
    /// Бар закрылся, тело подтвердило пересечение.
    Closed,
    /// Бар закрылся, но тело не подтвердило — только прокол тенью.
    WickOnly,
}
