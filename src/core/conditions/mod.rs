//! Conditions — атомарные предикаты.
//!
//! Словарь предикатов, используемый и runtime detectors, и codegen-templates.
//! Migrated from `signals/conditions.rs`.
//!
//! Roadmap (все enum'ы пока в `atoms.rs`, по мере роста разнесём):
//! - ThresholdCondition (Above/Below/InRange/OutOfRange/Near)
//! - CrossoverType (CrossUp/CrossDown/CrossAny)
//! - CompareCondition (GT/LT/EQ/NE с tolerance)
//! - TrendCondition (Rising/Falling/Sideways)
//! - DivergenceType (RegularBull/RegularBear/HiddenBull/HiddenBear)
//!   — спецификация ArrayVec<32> детектора в `runtime/detectors/divergence.rs`
//! - ChannelPosition (BelowLower/AboveUpper/Inside/...)
//! - PatternState (Forming/Confirmed/Broken)
//! - CandlePattern
//! - VolatilityRegime (Squeeze/Expansion/Normal)
//! - VolumeCharacter (Climax/Surge/Dry)
//! - LogicOp (And/Or/Not) + ConfirmationRequirement
//!   (Immediate/NextBar/WithinBars(N)/CloseConfirmation)

pub mod atoms;

pub use atoms::*;
