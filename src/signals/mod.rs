//! Signal Catalog - полная библиотека торговых сигналов
//!
//! Этот модуль содержит:
//! - Типы сигналов (SignalKind) - что именно произошло
//! - Детекторы условий (Condition) - логика срабатывания
//! - Утилиты для обнаружения (CrossoverDetector, ThresholdMonitor, etc.)
//! - Правила сигналов (rules) - конфигурируемые профили для индикаторов
//!
//! Архитектура:
//! ```text
//! Indicator → SignalProfile → Detectors → SignalKind → SignalEvent
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use zengeld_chart_indicators::signals::rules::{SignalProfile, SignalEngine, default_profile};
//!
//! // Get default profile for RSI
//! let profile = default_profile("RSI").unwrap();
//!
//! // Create engine
//! let mut engine = SignalEngine::from_profile(&profile);
//!
//! // Process indicator values
//! let signals = engine.process_simple(&indicator_value, timestamp);
//! ```

pub mod conditions;
pub mod detectors;
pub mod catalog;
pub mod rules;
pub mod signal;

pub use conditions::*;
pub use detectors::*;
pub use catalog::*;
pub use signal::{Signal, Direction, BarConfirmation, SignalSource};
