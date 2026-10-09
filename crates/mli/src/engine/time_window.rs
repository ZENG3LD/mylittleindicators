//! `TimeWindow` — a typed duration for indicator config.
//!
//! Replaces bare `window_ms: i64` parameters (e.g. LiquidationRate) where the
//! unit was implicit and the value rode the stringly bag as a raw float. This is
//! the first by-the-fact type-gap of the self-declaring contract: created
//! because a real indicator needed it, not designed up front.

/// A duration expressed in an explicit unit; resolves to milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(mli_contract_macros::ParamScalar)]
pub enum TimeWindow {
    Millis(i64),
    Seconds(i64),
    Minutes(i64),
    Hours(i64),
}

impl TimeWindow {
    /// The window length in milliseconds.
    pub fn as_millis(self) -> i64 {
        match self {
            Self::Millis(n) => n,
            Self::Seconds(n) => n * 1_000,
            Self::Minutes(n) => n * 60_000,
            Self::Hours(n) => n * 3_600_000,
        }
    }
}
