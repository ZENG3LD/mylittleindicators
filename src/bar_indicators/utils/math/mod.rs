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

/// Maximum-likelihood / least-squares model fitting via a derivative-free
/// Nelder-Mead simplex, for indicators whose parameters have no closed form
/// and must be estimated numerically (GARCH/EGARCH variance recursion, ARIMA
/// MA terms). Named `mle_fit` (not `optimize`) to avoid confusion with mlq's
/// strategy optimizer — this fits ONE indicator's own params on its own
/// window, it is not strategy parameter search. Replaces the
/// hardcoded-coefficient heuristics that were in `regression/`.
pub mod mle_fit;
