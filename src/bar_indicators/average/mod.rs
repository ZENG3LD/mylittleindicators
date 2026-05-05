// High-performance bar-based average indicators
pub mod sma;
pub mod ema;
pub mod wma;       // O(1) optimized WMA
pub mod rma;
pub mod hma;       // O(1) optimized HMA (uses Wma internally)
pub mod dema;
pub mod ama;       // O(1) optimized AMA using ring window ER
pub mod vidya;
pub mod vwap;
pub mod lr;
pub mod tma;
pub mod tema;
pub mod frama;
pub mod frama_advanced;

// Новые Ehlers индикаторы
pub mod ehlers_zero_lag_ema;
pub mod ehlers_fractal_adaptive_ma;

// Дополнительные MA индикаторы
pub mod alma;
pub mod jurik_ma;
pub mod mcginley_dynamic;
pub mod t3;
pub mod trima;
pub mod vwma;

pub mod moving_average;
// pub mod ohlcv_average;  // REMOVED: Replaced by MovingAverageWithField
// pub mod box_average;

// Universal Indicator System catalog
pub mod average_catalog;

pub use moving_average::{MovingAverageProvider, MovingAverageType, MovingAverageWithField, OhlcvField};
// pub use ohlcv_average::*;  // REMOVED: OhlcvAverage trait replaced by MovingAverageWithField
// pub use box_average::{BoxedAverage, BoxAverageFactory};
pub use tma::Tma;
pub use tema::Tema;
pub use frama::Frama;
pub use frama_advanced::FramaAdvanced;
pub use ehlers_zero_lag_ema::*;























