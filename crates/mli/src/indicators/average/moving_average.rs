//! Shared typed configs for period-based indicators + `OhlcvField` re-export.
//!
//! The legacy `MovingAverage` enum / `MovingAverageType` / `MovingAverageProvider`
//! smoother-factory ring was removed — scalar smoothing now goes through the
//! contract's `SmootherSlot` / `SmootherId`. What stays here are two small typed
//! configs reused by indicators whose only parameter is a period.

pub use crate::engine::ohlcv_field::OhlcvField;

/// Typed config for an indicator taking a period + a configurable OHLCV source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaConfig {
    pub period: usize,
    pub source: OhlcvField,
}

/// Shared typed config for indicators whose only parameter is a period and whose
/// price input is INTRINSIC (no configurable source) — e.g. the volume-weighted
/// averages VWMA (close × volume) and VWAP (typical price × volume). Not
/// MA-specific; reusable by any period-only indicator. Using this instead of
/// `MaConfig` keeps the contract honest: there is no `source` knob to expose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PeriodConfig {
    pub period: usize,
}

/// The slot sweep for a period-only member (every smoother except ALMA, every flagged
/// oscillator): a curated moderate set of sub-component lookbacks. NOT the host's full
/// `1..=10000` — a slot smoother/oscillator is a sub-part, so a small sensible set keeps a
/// multi-slot host's cube tractable (the host's own period axis sweeps the wide range).
impl crate::contract::SlotParamsSweep for PeriodConfig {
    fn machine_params() -> Vec<Self> {
        [5usize, 9, 14, 20, 30, 50, 100, 200]
            .into_iter()
            .map(|period| PeriodConfig { period })
            .collect()
    }
}

