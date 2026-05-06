//! Signal — runtime trading event для UI/alerts.
//!
//! Эмитируется индикатором или стратегией. Чарт использует для отрисовки
//! конфигурируемых меток (kind → иконка, direction → цвет, confirmation → стиль).
//! Алерт использует для фильтрации и триггеринга уведомлений.
//!
//! В hot loop MLQ оптимизатора НЕ участвует.

use serde::{Deserialize, Serialize};

use crate::strategies::events::{SignalKind, Direction, SignalSource, BarConfirmation};

/// Торговый сигнал — декларация события для чарта и алертов.
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
