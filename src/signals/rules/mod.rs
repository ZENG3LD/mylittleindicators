//! Signal Rules - configurable signal generation rules for indicators
//!
//! This module provides:
//! - `DetectorConfig` - configuration for individual detectors
//! - `SignalProfile` - collection of detector configs for an indicator
//! - `SignalEngine` - runtime engine that processes indicator values
//! - Default profiles for common indicators (RSI, MACD, BB, etc.)
//!
//! ## Architecture
//!
//! ```text
//! SignalProfile (config)
//!     └── Vec<DetectorConfig>
//!             └── DetectorParams (thresholds, sources, etc.)
//!
//! SignalEngine (runtime)
//!     └── processes IndicatorValue through configured detectors
//!     └── emits Vec<SignalEvent>
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use zengeld_chart_indicators::signals::rules::{SignalProfile, SignalEngine, defaults};
//!
//! // Get default profile for RSI
//! let profile = defaults::rsi_profile();
//!
//! // Customize thresholds
//! let mut profile = profile;
//! profile.update_threshold("overbought", 80.0, 20.0);
//!
//! // Create engine and process values
//! let mut engine = SignalEngine::from_profile(profile);
//! let signals = engine.process(indicator_value);
//! ```

mod config;
mod profile;
mod defaults;
mod engine;

pub use config::{
    DetectorConfig, DetectorType, DetectorParams, ValueSource,
};
pub use profile::SignalProfile;
pub use defaults::default_profile;
pub use engine::SignalEngine;
