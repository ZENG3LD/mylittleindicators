//! Candlestick Pattern Detectors
//!
//! This module contains individual candlestick pattern detectors extracted from
//! AdvancedPatternRecognition. Each pattern is now a standalone, lightweight indicator.

// Single-candle patterns
pub mod doji;
pub mod hammer;
pub mod shooting_star;
pub mod marubozu;

// Two-candle patterns
pub mod engulfing;
pub mod harami;
pub mod piercing_pattern;
pub mod tweezer;

// Three-candle patterns
pub mod morning_star;
pub mod three_white_soldiers;
pub mod dark_cloud_cover;
pub mod evening_star;
pub mod three_black_crows;

// Re-exports for convenience
pub use doji::{Doji, DojiResult};
pub use hammer::{Hammer, HammerResult};
pub use shooting_star::{ShootingStar, ShootingStarResult};
pub use marubozu::{Marubozu, MarubozuResult};

pub use engulfing::{Engulfing, EngulfingResult};
pub use harami::{Harami, HaramiResult};
pub use piercing_pattern::{PiercingPattern, PiercingPatternResult};
pub use tweezer::{Tweezer, TweezerResult};

pub use morning_star::{MorningStar, MorningStarResult};
pub use three_white_soldiers::{ThreeWhiteSoldiers, ThreeWhiteSoldiersResult};
pub use dark_cloud_cover::DarkCloudCover;
pub use evening_star::EveningStar;
pub use three_black_crows::ThreeBlackCrows;
