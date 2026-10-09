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

pub use moving_average::OhlcvField;
pub use tma::Tma;
pub use tema::Tema;
pub use frama::Frama;
pub use ehlers_zero_lag_ema::*;

// adaptive/ folded into average/ (adaptive MAs are MAs).
pub mod kaufman_adaptive_ma;
pub mod mesa_adaptive_ma;
pub use kaufman_adaptive_ma::*;
pub use mesa_adaptive_ma::*;
