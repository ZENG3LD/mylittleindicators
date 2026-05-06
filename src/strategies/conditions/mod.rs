//! Conditions — атомарные предикаты.
//!
//! Berется из MLI `signals/conditions.rs` (12 enum'ов). Это словарь предикатов
//! который используют и runtime detectors, и codegen-templates.
//!
//! Roadmap:
//! - `threshold.rs`    — ThresholdCondition (Above/Below/Between/...)
//! - `crossover.rs`    — CrossoverType (BullishCross, BearishCross, ZeroCross)
//! - `compare.rs`      — CompareCondition (GT/LT/EQ/NE с tolerance)
//! - `trend.rs`        — TrendCondition (Rising/Falling/Sideways)
//! - `divergence.rs`   — DivergenceType (Regular Bull/Bear, Hidden Bull/Bear) +
//!                       спецификация ArrayVec<32> детектора (MLI ведущий)
//! - `channel.rs`      — ChannelPosition / ZoneEnter / ZoneExit (MLI + MLQ объединены)
//! - `pattern.rs`      — PatternState (Forming/Confirmed/Broken) + CandlePattern
//! - `volatility.rs`   — VolatilityRegime (Squeeze/Expansion/Normal)
//! - `volume.rs`       — VolumeCharacter (Climax/Surge/Dry)
//! - `logic.rs`        — LogicOp (And/Or/Not) + ConfirmationRequirement
//!                       (Immediate/NextBar/WithinBars(N)/CloseConfirmation)
