//! Guard conditions — extra filters that must pass for an event to fire.

/// Comparison operator used in guard conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CmpOp {
    Gt,
    Gte,
    Lt,
    Lte,
    Eq,
    Neq,
}

/// A single guard condition that gates an event.
///
/// All guards in a `Vec<Guard>` must evaluate to `true` for the event
/// to be considered active.
#[derive(Debug, Clone, PartialEq)]
pub enum Guard {
    /// Indicator role output must satisfy a threshold comparison.
    Regime {
        /// Index into `StrategySpec::roles`.
        role_idx: usize,
        op: CmpOp,
        threshold: f64,
    },
    /// A named state variable must match a value.
    State {
        /// Index into `StateSpec::vars`.
        var_idx: usize,
        op: CmpOp,
        val: i32,
    },
    /// Only trigger within a specific hour range (UTC).
    TimeOfDay {
        hour_start: u8,
        hour_end: u8,
    },
    /// Volume must be at least `mult` times the rolling average volume.
    VolumeMin {
        /// Index into `StrategySpec::roles` for the volume indicator.
        role_idx: usize,
        mult: f64,
    },
    /// No open position (flat before entry).
    PositionFlat,
}
