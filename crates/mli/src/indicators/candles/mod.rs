// pub mod box_candles;
pub mod candle_anatomy;
pub mod heikin_ashi;

// Relocated from volatility/ (Phase 1 reorg): fuzzy candlestick classifier.
pub mod fuzzy;
pub use fuzzy::*;

// Decomposed from events/ (Phase 1 reorg 2026-06-16): candle-pattern + wick detectors.
pub mod candle_pattern;
pub mod statistical_wick_detector;
pub use candle_pattern::*;
pub use statistical_wick_detector::*;
