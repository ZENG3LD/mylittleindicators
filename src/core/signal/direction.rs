//! Direction + BarConfirmation — signal direction and bar confirmation status.

use serde::{Deserialize, Serialize};

/// Направление сигнала.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
    /// Бычье — вверх.
    Up,
    /// Медвежье — вниз.
    Down,
    /// Без направления (Doji, объёмный spike, сжатие канала).
    Neutral,
}

impl Direction {
    pub fn as_i8(self) -> i8 {
        match self {
            Self::Up => 1,
            Self::Down => -1,
            Self::Neutral => 0,
        }
    }

    pub fn from_i8(v: i8) -> Self {
        match v {
            1 => Self::Up,
            -1 => Self::Down,
            _ => Self::Neutral,
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Neutral => Self::Neutral,
        }
    }
}

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
