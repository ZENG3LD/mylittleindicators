//! Defaults — энциклопедия чисел per-индикатор.
//!
//! Migrated from `signals/rules/defaults.rs` (1466 LOC monolith). Сейчас всё
//! в одном `profiles.rs`, по мере роста разнесём по категориям индикаторов
//! (profiles_momentum.rs / profiles_trend.rs / profiles_volatility.rs / ...).
//!
//! Используется:
//! - MLC runtime: `default_profile(BarIndicatorId)` для UI алертов;
//! - MLQ codegen: `default_thresholds(BarIndicatorId) -> Option<(f64, f64)>` —
//!   seed для slot filling в shapes (RSI=30/70, LaguerreRSI=0.2/0.8, etc).
//!   TODO: добавить `default_thresholds()`.

pub mod profiles;

pub use profiles::default_profile;
