//! Defaults — энциклопедия чисел per-индикатор.
//!
//! MLI ведущий. Источник: бывший `signals/rules/defaults.rs` (1466 LOC monolith)
//! разнесён по категориям индикаторов (momentum/trend/volatility/...).
//!
//! Используется:
//! - MLC runtime: `default_signal_profile(BarIndicatorId)` для UI алертов;
//! - MLQ codegen: `default_thresholds(BarIndicatorId) -> Option<(f64, f64)>` —
//!   seed для slot filling в shapes (RSI=30/70, LaguerreRSI=0.2/0.8,
//!   IFT_RSI tolerance=0.05, Vortex=1.1/0.9).
//!
//! Roadmap:
//! - `mod.rs`                   — public API: default_signal_profile / default_thresholds
//! - `profiles_momentum.rs`     — RSI, Stoch, CCI, MACD, ROC, etc
//! - `profiles_trend.rs`        — ADX, Aroon, Vortex, Supertrend, etc
//! - `profiles_volatility.rs`   — ATR, BBands, Keltner, etc
//! - `profiles_channels.rs`     — Donchian, Envelope, etc
//! - `profiles_volume.rs`       — OBV, MFI, CMF, etc
//! - ... (по остальным 23 категориям индикаторов)
