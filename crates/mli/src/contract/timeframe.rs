//! `ResearchTimeframe` — the research/backtest timeframe axis.
//!
//! Moved down from `mlq-core::data::timeframe` (2026-06-28 strategy-migration) so the
//! strategy role model (timeframed roles) is owned by the mli family, not MLQ. Pure
//! data, **no serde** (the mli family forbids serde): the MLQ JSON config boundary
//! parses timeframe strings into this enum on a loader-side DTO, not via a derive here.
//!
//! **Unit cutover (2026-07-06)**: the bar-time contract is MILLISECONDS end-to-end
//! (dig3 supplies ms verbatim). `to_seconds()` is REMOVED — `Custom(u32)` is now
//! milliseconds (not seconds); the canonical accessor is [`ResearchTimeframe::to_millis`].
//! A consumer that genuinely needs seconds derives it locally from `to_millis() / 1000`.

use std::fmt;

/// Временные интервалы для агрегации данных.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
    Custom(u32), // в миллисекундах для большей точности
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
            ResearchTimeframe::Custom(ms) => write!(f, "{}ms", ms),
        }
    }
}

impl ResearchTimeframe {
    /// Конвертировать таймфрейм в миллисекунды — канонический accessor.
    pub fn to_millis(&self) -> i64 {
        match self {
            ResearchTimeframe::Tick => 0,
            ResearchTimeframe::S1 => 1_000,
            ResearchTimeframe::S5 => 5_000,
            ResearchTimeframe::S10 => 10_000,
            ResearchTimeframe::S15 => 15_000,
            ResearchTimeframe::S30 => 30_000,
            ResearchTimeframe::M1 => 60_000,
            ResearchTimeframe::M5 => 300_000,
            ResearchTimeframe::M15 => 900_000,
            ResearchTimeframe::M30 => 1_800_000,
            ResearchTimeframe::H1 => 3_600_000,
            ResearchTimeframe::H4 => 14_400_000,
            ResearchTimeframe::D1 => 86_400_000,
            ResearchTimeframe::W1 => 604_800_000,
            ResearchTimeframe::Custom(ms) => *ms as i64,
        }
    }

    /// Конвертировать таймфрейм в минуты.
    pub fn to_minutes(&self) -> u32 {
        (self.to_millis() / 60_000) as u32
    }
}
