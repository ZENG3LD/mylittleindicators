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
pub mod candle_pattern;
pub mod confluence;
pub mod crossover;
pub mod direction_detector;
pub mod divergence;
pub mod event_at_level;
pub mod pivot;
pub mod regime_gate;
pub mod relative_position;
pub mod swing_detection;
pub mod threshold;
pub mod volatility_regime;
pub mod volume_event;
pub mod zone_enter;
pub mod zone_exit;

pub use breakout::Breakout;
pub use candle_pattern::CandlePatternDetector;
pub use confluence::Confluence;
pub use crossover::Crossover;
pub use direction_detector::DirectionDetector;
pub use divergence::Divergence;
pub use event_at_level::EventAtLevel;
pub use pivot::Pivot;
pub use regime_gate::RegimeGate;
pub use relative_position::RelativePosition;
pub use swing_detection::SwingDetection;
pub use threshold::Threshold;
pub use volatility_regime::VolatilityRegimeDetector;
pub use volume_event::VolumeEventDetector;
pub use zone_enter::ZoneEnter;
pub use zone_exit::ZoneExit;
