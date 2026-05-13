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

pub mod candle_pattern;
pub mod confluence;
pub mod direction_detector;
pub mod divergence;
pub mod line_cross;
pub mod oscillator_with_divergence;
pub mod pivot;
pub mod price_line_cross;
pub mod regime_gate;
pub mod relative_position;
pub mod swing_detection;
pub mod threshold;
pub mod volatility_regime;
pub mod volume_event;

pub use candle_pattern::{CandlePatternDetector, CandlePatternKind};
pub use confluence::Confluence;
pub use direction_detector::DirectionDetector;
pub use divergence::Divergence;
pub use line_cross::{CrossMode, LineCross, LineSource as LineCrossSource};
pub use oscillator_with_divergence::OscillatorWithDivergence;
pub use pivot::Pivot;
pub use price_line_cross::{LineSource as PriceLineSource, PriceLineCross, TouchMode};
pub use regime_gate::RegimeGate;
pub use relative_position::RelativePosition;
pub use swing_detection::SwingDetection;
pub use threshold::Threshold;
pub use volatility_regime::VolatilityRegimeDetector;
pub use volume_event::VolumeEventDetector;
