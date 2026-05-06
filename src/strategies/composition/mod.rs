//! Composition — как собирать стратегию из условий.
//!
//! MLQ-ведущая часть. MLI имеет только `MultiSignalDetector` (vote ≥2 в одном
//! баре) — слабее composition tree, остаётся в `runtime/` для UI Composite::Strong.
//!
//! Roadmap:
//! - `spec.rs`        — CompositionSpec: And/Or/Not/Sequence{max_bars}
//!                      (sequence — MLQ-only, temporal "A then B within N bars")
//! - `guard.rs`       — Guard: Regime/State/TimeOfDay/PositionFlat/VolumeMin
//!                      (filter перед входом в позицию, MLI не имеет аналога)
//! - `state.rs`       — StateVar (BoolFlag/F64NanSentinel/Counter/SignalState/
//!                      I32State/EnumState) + Transition
//! - `validation.rs`  — ValidateRule: RolePeriodLess/Greater/NotEqual
//!                      (compile-time контракты для cartesian-перебора)
