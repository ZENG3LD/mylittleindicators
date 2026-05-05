//! `Signal` — торговый сигнал, интерфейс между индикатором/стратегией и чартом/алертом.
//!
//! Индикатор или стратегия эмитит `Signal`, чарт рисует конфигурируемую метку,
//! алерт триггерит уведомление. В хотлупе кванта не участвует.

use serde::{Deserialize, Serialize};

use super::catalog::SignalKind;

// ============================================================================
// DIRECTION
// ============================================================================

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

// ============================================================================
// BAR CONFIRMATION
// ============================================================================

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

// ============================================================================
// SIGNAL SOURCE
// ============================================================================

/// Кто создал сигнал.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalSource {
    /// Индикатор по имени (e.g. "RSI", "MACD", "EMA").
    Indicator(String),
    /// Стратегия по имени.
    Strategy(String),
}

// ============================================================================
// SIGNAL
// ============================================================================

/// Торговый сигнал — декларация события для чарта и алертов.
///
/// Эмитируется индикатором или стратегией. Чарт использует для отрисовки
/// конфигурируемых меток (kind → иконка, direction → цвет, confirmation → стиль).
/// Алерт использует для фильтрации и триггеринга уведомлений.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    /// Монотонно возрастающий уникальный ID.
    pub id: u64,
    /// Индекс бара.
    pub bar_index: usize,
    /// Unix timestamp (ms).
    pub timestamp: i64,
    /// Цена в момент сигнала.
    pub price: f64,
    /// ЧТО произошло.
    pub kind: SignalKind,
    /// КУДА — направление.
    pub direction: Direction,
    /// Статус бара: pending / closed / wick only.
    pub confirmation: BarConfirmation,
    /// КТО создал.
    pub source: SignalSource,
}

impl Signal {
    pub fn new(
        id: u64,
        bar_index: usize,
        timestamp: i64,
        price: f64,
        kind: SignalKind,
        direction: Direction,
        confirmation: BarConfirmation,
        source: SignalSource,
    ) -> Self {
        Self {
            id,
            bar_index,
            timestamp,
            price,
            kind,
            direction,
            confirmation,
            source,
        }
    }
}
