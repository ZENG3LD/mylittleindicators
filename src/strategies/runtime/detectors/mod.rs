//! Stateful detectors для streaming UI/alerts.
//!
//! Каждый детектор: `update(value | bar) -> Option<Signal>` (или Vec, если
//! может несколько сигналов за бар).
//!
//! Migrated from `signals/detectors.rs` (1029 LOC). Все 11 детекторов пока в
//! одном файле `all.rs`, по мере необходимости разнесём:
//! CrossoverDetector, ThresholdMonitor, ZeroCrossDetector, HistogramDetector,
//! ChannelDetector, DivergenceDetector, TrendDetector, VolatilityDetector,
//! VolumeDetector, SwingDetector, MultiSignalDetector.

pub mod all;

pub use all::*;
