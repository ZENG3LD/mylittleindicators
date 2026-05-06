//! Direction + SignalSource — кто и куда.
//!
//! Migrated from `signals/signal.rs`.

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

/// Кто создал сигнал.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalSource {
    /// Индикатор по имени (e.g. "RSI", "MACD", "EMA").
    Indicator(String),
    /// Стратегия по имени.
    Strategy(String),
}
