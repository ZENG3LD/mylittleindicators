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

pub mod bos_event_detector;
pub mod fvg_event_detector;
pub mod candle_pattern;
pub mod confluence;
pub mod cross_asset_beta;
pub mod direction_detector;
pub mod divergence;
pub mod line_cross;
pub mod oscillator_with_divergence;
pub mod oscillator_with_volume_weight;
pub mod pairs_cointegration_proxy;
pub mod pivot;
pub mod price_line_cross;
pub mod regime_gate;
pub mod relative_position;
pub mod relative_strength_cross;
pub mod statistical_wick_detector;
pub mod swing_detection;
pub mod threshold;
pub mod volatility_regime;
pub mod volume_event;

// Factory / catalog modules
pub mod event_id;
pub mod event_config;
pub mod event_instance;
pub mod events_catalog;

pub use bos_event_detector::BosEventDetector;
pub use fvg_event_detector::FvgEventDetector;
pub use candle_pattern::{CandlePatternDetector, CandlePatternKind};
pub use statistical_wick_detector::StatisticalWickDetector;
pub use confluence::Confluence;
pub use cross_asset_beta::CrossAssetBeta;
pub use direction_detector::DirectionDetector;
pub use divergence::Divergence;
pub use line_cross::{CrossMode, LineCross, LineSource as LineCrossSource};
pub use oscillator_with_divergence::OscillatorWithDivergence;
pub use oscillator_with_volume_weight::OscillatorWithVolumeWeight;
pub use pairs_cointegration_proxy::PairsCointegrationProxy;
pub use pivot::Pivot;
pub use price_line_cross::{LineSource as PriceLineSource, PriceLineCross, TouchMode};
pub use regime_gate::RegimeGate;
pub use relative_position::RelativePosition;
pub use relative_strength_cross::RelativeStrengthCross;
pub use swing_detection::SwingDetection;
pub use threshold::Threshold;
pub use volatility_regime::VolatilityRegimeDetector;
pub use volume_event::VolumeEventDetector;

pub use event_id::EventId;
pub use event_config::EventConfig;
pub use event_instance::EventInstance;
