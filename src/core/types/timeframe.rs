//! # Timeframe
//!
//! Таймфреймы для агрегации и анализа данных.
//! Используется в CoreDataAggregator и по всему research слою.

use std::fmt;

/// Временные интервалы для агрегации данных
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum ResearchTimeframe {
    Tick,
    S1,  // 1 second
    S5,  // 5 seconds
    S10, // 10 seconds
    S15, // 15 seconds
    S30, // 30 seconds
    M1,
    M5,
    M15,
    M30,
    H1,
    H4,
    D1,
    W1,          // 1 week
    Custom(u32), // в секундах для большей точности
}

impl fmt::Display for ResearchTimeframe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResearchTimeframe::Tick => write!(f, "Tick"),
            ResearchTimeframe::S1 => write!(f, "1s"),
            ResearchTimeframe::S5 => write!(f, "5s"),
            ResearchTimeframe::S10 => write!(f, "10s"),
            ResearchTimeframe::S15 => write!(f, "15s"),
            ResearchTimeframe::S30 => write!(f, "30s"),
            ResearchTimeframe::M1 => write!(f, "1m"),
            ResearchTimeframe::M5 => write!(f, "5m"),
            ResearchTimeframe::M15 => write!(f, "15m"),
            ResearchTimeframe::M30 => write!(f, "30m"),
            ResearchTimeframe::H1 => write!(f, "1h"),
            ResearchTimeframe::H4 => write!(f, "4h"),
            ResearchTimeframe::D1 => write!(f, "1d"),
            ResearchTimeframe::W1 => write!(f, "1w"),
            ResearchTimeframe::Custom(s) => write!(f, "{}s", s),
        }
    }
}

impl ResearchTimeframe {
    /// Конвертировать таймфрейм в секунды
    pub fn to_seconds(&self) -> u32 {
        match self {
            ResearchTimeframe::Tick => 0,
            ResearchTimeframe::S1 => 1,
            ResearchTimeframe::S5 => 5,
            ResearchTimeframe::S10 => 10,
            ResearchTimeframe::S15 => 15,
            ResearchTimeframe::S30 => 30,
            ResearchTimeframe::M1 => 60,
            ResearchTimeframe::M5 => 300,
            ResearchTimeframe::M15 => 900,
            ResearchTimeframe::M30 => 1800,
            ResearchTimeframe::H1 => 3600,
            ResearchTimeframe::H4 => 14400,
            ResearchTimeframe::D1 => 86400,
            ResearchTimeframe::W1 => 604800,
            ResearchTimeframe::Custom(s) => *s,
        }
    }

    /// Конвертировать таймфрейм в минуты
    pub fn to_minutes(&self) -> u32 {
        self.to_seconds() / 60
    }
}
