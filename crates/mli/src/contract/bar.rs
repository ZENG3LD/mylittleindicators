//! `ResearchBar` — the OHLCV bar the strategy hot-loop reads.
//!
//! Moved down from `mlq-core::data::bar` (2026-06-28 strategy-migration) so the
//! strategy↔engine contract is owned by the mli family, not MLQ. Pure data, **no
//! serde** (the mli family forbids serde): the MLQ data loader constructs bars via
//! [`ResearchBar::new`], and any serialization happens at the MLQ boundary on a
//! loader-side DTO, not on this type.

/// Базовый бар OHLCV (Open, High, Low, Close, Volume).
#[derive(Debug, Clone, Copy)]
pub struct ResearchBar {
    /// Bar open time in **unix milliseconds** (2026-07-06 unit cutover — dig3
    /// supplies ms verbatim, no `/1000` at ingest). NOT seconds.
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl ResearchBar {
    /// Создать новый бар.
    pub fn new(time: i64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Self {
        Self { time, open, high, low, close, volume }
    }
}
