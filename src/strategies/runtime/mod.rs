//! Runtime — streaming layer для UI/alerts.
//!
//! НЕ участвует в MLQ hot loop (есть allocations, runtime dispatch, ArrayVec).
//! Используется MLC чартом для отрисовки маркеров и алертами для уведомлений.
//!
//! Codegen-сгенерированные стратегии в MLQ **повторяют** state machines детекторов
//! отсюда — это и есть склейка: один источник правил для алерта и backtest.
//!
//! Roadmap (переезжает из `signals/`):
//! - `signal.rs`        — Signal struct + Direction + BarConfirmation + SignalSource
//! - `engine.rs`        — SignalEngine::process(value, ts) -> Vec<Signal>
//! - `profile.rs`       — SignalProfile (per-indicator config)
//! - `config.rs`        — config types
//! - `detectors/`       — 11 stateful детекторов:
//!     - `crossover.rs` / `threshold.rs` / `zero_cross.rs` / `histogram.rs`
//!     - `channel.rs` / `divergence.rs` / `trend.rs` / `volatility.rs`
//!     - `volume.rs` / `swing.rs`
//!     - `multi_signal.rs` — vote-based для UI Composite::Strong/Conflict

pub mod detectors;
