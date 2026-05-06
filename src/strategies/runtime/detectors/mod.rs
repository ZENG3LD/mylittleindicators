//! Stateful detectors для streaming UI/alerts.
//!
//! Каждый детектор: `update(value | bar) -> Option<Signal>` (или Vec, если может
//! несколько сигналов за бар).
//!
//! TODO: переезд из `signals/detectors.rs` (1029 LOC) — 11 детекторов:
//! CrossoverDetector, ThresholdMonitor, ZeroCrossDetector, HistogramDetector,
//! ChannelDetector, DivergenceDetector, TrendDetector, VolatilityDetector,
//! VolumeDetector, SwingDetector, MultiSignalDetector.
