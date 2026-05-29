//! Mathematical utilities for indicators
//!
//! Централизованные математические функции для избежания дублирования кода.

pub mod percentile;

/// Probability distributions — normal/t/χ² CDFs + inverses + ln-gamma.
/// Foundation for honest statistical inference (p-values, Sharpe ratios).
pub mod distributions;

/// Dense linear algebra — real OLS / Gaussian solve / Cholesky. Replaces the
/// diagonal-only-OLS shortcut in regression indicators.
pub mod linalg;

/// Time-series primitives — Newey-West LRV, augmented Dickey-Fuller regression,
/// MacKinnon/Kwiatkowski critical values. Shared by unit-root / stationarity
/// indicators so they emit REAL test statistics.
pub mod timeseries;
