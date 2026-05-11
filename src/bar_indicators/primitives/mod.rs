//! Composite-detector primitives.
//!
//! Primitives are indicators whose own logic detects a SIGNAL/EVENT from
//! the interaction of OHLCV data with N inner indicator outputs. Unlike
//! plain composite indicators (Bollinger, MACD) that produce continuous
//! scalar values, primitives produce event-shaped outputs
//! (`IndicatorValue::Signal(i8)`, `Flag`, `DoubleFlag`, etc.).
//!
//! Each primitive owns its inner indicators via `Box<IndicatorInstance>`
//! and drives them in lock-step with the host bar stream. Dedup happens
//! at the slice-cache layer through `IndicatorKey.param_hash` which folds
//! inner-indicator configs recursively.

pub mod breakout;
pub mod confluence;
pub mod crossover;
pub mod divergence;
pub mod event_at_level;
pub mod relative_position;
pub mod swing_detection;

pub use breakout::Breakout;
pub use confluence::Confluence;
pub use crossover::Crossover;
pub use divergence::Divergence;
pub use event_at_level::EventAtLevel;
pub use relative_position::RelativePosition;
pub use swing_detection::SwingDetection;
